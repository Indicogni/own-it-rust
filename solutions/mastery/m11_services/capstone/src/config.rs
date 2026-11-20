//! Settings from the environment. Parsing is a pure function of the variables,
//! so it is tested without touching the real environment.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

/// A configuration problem, phrased for the operator. It names the variable and
/// never contains a value, because some values are secrets.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} is required")]
    Missing(&'static str),
    #[error("{0}: {1}")]
    Invalid(&'static str, &'static str),
}

/// The variables with the `BEACON_` prefix.
pub struct Env(HashMap<String, String>);

impl Env {
    pub fn from_pairs(pairs: impl IntoIterator<Item = (String, String)>) -> Env {
        Env(pairs
            .into_iter()
            .filter(|(name, _)| name.starts_with("BEACON_"))
            .collect())
    }

    /// The variables of this process. `std::env::vars` panics when any
    /// variable is not valid Unicode; this skips such a variable instead. If it
    /// was a required one, it is then reported as missing.
    pub fn from_process() -> Env {
        Env::from_pairs(std::env::vars_os().filter_map(|(name, value)| {
            Some((name.into_string().ok()?, value.into_string().ok()?))
        }))
    }

    fn text(&self, name: &'static str) -> Option<&str> {
        self.0
            .get(name)
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }

    fn required(&self, name: &'static str) -> Result<&str, ConfigError> {
        self.text(name).ok_or(ConfigError::Missing(name))
    }

    fn number(&self, name: &'static str, default: u64, min: u64) -> Result<u64, ConfigError> {
        match self.text(name) {
            None => Ok(default),
            Some(text) => match text.parse::<u64>() {
                Ok(number) if number >= min => Ok(number),
                _ => Err(ConfigError::Invalid(
                    name,
                    "must be a whole number in range",
                )),
            },
        }
    }

    fn addr(&self, name: &'static str, default: &str) -> Result<SocketAddr, ConfigError> {
        self.text(name)
            .unwrap_or(default)
            .parse()
            .map_err(|_| ConfigError::Invalid(name, "must be an address such as 127.0.0.1:7400"))
    }
}

/// Settings of `beacon-server`.
#[derive(Clone)]
pub struct ServerConfig {
    pub ingest_addr: SocketAddr,
    pub http_addr: SocketAddr,
    pub database_url: String,
    pub admin_user: String,
    pub admin_password: String,
    pub token_key: Vec<u8>,
}

impl ServerConfig {
    pub fn from_env(env: &Env) -> Result<ServerConfig, ConfigError> {
        let token_key = env.required("BEACON_TOKEN_KEY")?.as_bytes().to_vec();
        if token_key.len() < 32 {
            return Err(ConfigError::Invalid(
                "BEACON_TOKEN_KEY",
                "must have at least 32 bytes",
            ));
        }
        Ok(ServerConfig {
            ingest_addr: env.addr("BEACON_INGEST_ADDR", "127.0.0.1:7400")?,
            http_addr: env.addr("BEACON_HTTP_ADDR", "127.0.0.1:7401")?,
            database_url: env
                .text("BEACON_DB")
                .unwrap_or("sqlite://beacon.db")
                .to_owned(),
            admin_user: env.required("BEACON_ADMIN_USER")?.to_owned(),
            admin_password: env.required("BEACON_ADMIN_PASSWORD")?.to_owned(),
            token_key,
        })
    }
}

/// Longest agent id and metric name, in bytes.
const MAX_NAME_BYTES: usize = 255;
/// Most samples in one batch: 3000 samples of 20 bytes, a metric name and the
/// counters stay below `beacon_wire::MAX_PAYLOAD`.
const MAX_BATCH: u64 = 3000;

/// Settings of `beacon-agent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfig {
    pub server: String,
    pub agent_id: String,
    pub metric: String,
    pub interval: Duration,
    /// Take this many samples, ship them, and exit. `None`: run until stopped.
    pub count: Option<u64>,
    pub buffer: usize,
    pub batch: usize,
}

impl AgentConfig {
    pub fn from_env(env: &Env) -> Result<AgentConfig, ConfigError> {
        match env.text("BEACON_SOURCE") {
            None | Some("counter") => {}
            Some(_) => return Err(ConfigError::Invalid("BEACON_SOURCE", "unknown source")),
        }
        let count = match env.text("BEACON_COUNT") {
            None => None,
            Some(_) => Some(env.number("BEACON_COUNT", 0, 0)?),
        };
        // Every message must fit in one frame (64 KiB of payload), or the agent
        // could never send it and would retry for ever. A sample takes at most
        // 20 bytes (two varints), so these limits leave room to spare.
        let agent_id = env.required("BEACON_AGENT_ID")?;
        if agent_id.len() > MAX_NAME_BYTES {
            return Err(ConfigError::Invalid("BEACON_AGENT_ID", "is too long"));
        }
        let metric = env.text("BEACON_METRIC").unwrap_or("counter");
        if metric.len() > MAX_NAME_BYTES {
            return Err(ConfigError::Invalid("BEACON_METRIC", "is too long"));
        }
        let batch = env.number("BEACON_BATCH", 50, 1)?;
        if batch > MAX_BATCH {
            return Err(ConfigError::Invalid("BEACON_BATCH", "is too large"));
        }
        Ok(AgentConfig {
            server: env.required("BEACON_SERVER")?.to_owned(),
            agent_id: agent_id.to_owned(),
            metric: metric.to_owned(),
            interval: Duration::from_millis(env.number("BEACON_INTERVAL_MS", 1000, 1)?),
            count,
            buffer: env.number("BEACON_BUFFER", 1000, 1)? as usize,
            batch: batch as usize,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> Env {
        Env::from_pairs(
            pairs
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned())),
        )
    }

    #[test]
    fn agent_defaults_and_overrides() {
        let config = AgentConfig::from_env(&env(&[
            ("BEACON_SERVER", "127.0.0.1:7400"),
            ("BEACON_AGENT_ID", "pump-7"),
            ("BEACON_COUNT", "0"),
            ("PATH", "/usr/bin"),
        ]))
        .unwrap();
        assert_eq!(config.metric, "counter");
        assert_eq!(config.interval, Duration::from_secs(1));
        assert_eq!(config.count, Some(0));
        assert_eq!((config.buffer, config.batch), (1000, 50));
    }

    #[test]
    fn the_largest_allowed_batch_fits_in_one_frame() {
        use beacon_wire::{Message, Sample, encode_frame};
        let name = "x".repeat(MAX_NAME_BYTES);
        let base = [
            ("BEACON_SERVER", "127.0.0.1:7400"),
            ("BEACON_AGENT_ID", "pump-7"),
        ];
        let with = |extra: (&'static str, String)| {
            let mut pairs: Vec<(String, String)> = base
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect();
            pairs.push((extra.0.to_owned(), extra.1));
            AgentConfig::from_env(&Env::from_pairs(pairs))
        };
        assert!(with(("BEACON_BATCH", MAX_BATCH.to_string())).is_ok());
        assert_eq!(
            with(("BEACON_BATCH", (MAX_BATCH + 1).to_string())),
            Err(ConfigError::Invalid("BEACON_BATCH", "is too large"))
        );
        assert!(with(("BEACON_METRIC", format!("{name}x"))).is_err());

        // Close to the worst case: times spread over the whole u64 range, so
        // every difference is a long varint, and values of ten bytes each.
        let worst = Message::Batch {
            seq: u64::MAX,
            metric: name.clone(),
            samples: (1..=MAX_BATCH)
                .map(|i| Sample {
                    ts_ms: i * (u64::MAX / MAX_BATCH),
                    value: i64::MIN,
                })
                .collect(),
        };
        encode_frame(&worst, &mut Vec::new()).expect("the largest batch fits in a frame");
        let hello = Message::Hello { agent: name };
        encode_frame(&hello, &mut Vec::new()).expect("the longest agent id fits in a frame");
    }

    #[test]
    fn errors_name_the_variable_and_hide_values() {
        let missing = AgentConfig::from_env(&env(&[("BEACON_SERVER", "x")])).unwrap_err();
        assert_eq!(missing, ConfigError::Missing("BEACON_AGENT_ID"));
        let short_key = ServerConfig::from_env(&env(&[
            ("BEACON_TOKEN_KEY", "too-short-secret"),
            ("BEACON_ADMIN_USER", "admin"),
            ("BEACON_ADMIN_PASSWORD", "pw"),
        ]));
        let text = short_key.err().unwrap().to_string();
        assert!(text.contains("BEACON_TOKEN_KEY") && !text.contains("too-short-secret"));
    }
}
