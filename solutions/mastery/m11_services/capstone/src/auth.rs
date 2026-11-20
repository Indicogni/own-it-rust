//! Authentication: one administrator, Argon2id, HMAC-signed tokens (lesson 04).

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

/// How long a token is valid.
pub const TOKEN_TTL_SECS: u64 = 3600;

/// What the server needs to check a login and a token. Holds no plain password.
#[derive(Clone)]
pub struct Authenticator {
    user: String,
    password_hash: String,
    decoy_hash: String,
    key: Vec<u8>,
}

fn hash(password: &str) -> Result<String, String> {
    // The crate's default parameters: Argon2id, 19 MiB, 2 passes, 1 lane.
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|phc| phc.to_string())
        .map_err(|err| err.to_string())
}

fn matches(stored: &str, password: &str) -> bool {
    PasswordHash::new(stored)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2)
        || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

impl Authenticator {
    /// Hashes the configured password once, at start-up. The plain text is
    /// dropped by the caller afterwards.
    pub fn new(user: &str, password: &str, key: &[u8]) -> Result<Authenticator, String> {
        Ok(Authenticator {
            user: user.to_owned(),
            password_hash: hash(password)?,
            decoy_hash: hash("no user has this password")?,
            key: key.to_vec(),
        })
    }

    /// Blocking and slow by design: call it from `spawn_blocking`. An unknown
    /// user costs the same hash computation as a known one.
    pub fn check_password(&self, user: &str, password: &str) -> bool {
        let known = user == self.user;
        let stored = if known {
            &self.password_hash
        } else {
            &self.decoy_hash
        };
        matches(stored, password) && known
    }

    fn mac(&self) -> Hmac<Sha256> {
        Hmac::<Sha256>::new_from_slice(&self.key).expect("HMAC accepts any key length")
    }

    /// `v1.<hex user>.<expiry seconds>.<hex mac>`
    pub fn issue_token(&self, user: &str) -> String {
        let signed = format!("v1.{}.{}", to_hex(user.as_bytes()), now() + TOKEN_TTL_SECS);
        let mut mac = self.mac();
        mac.update(signed.as_bytes());
        format!("{signed}.{}", to_hex(&mac.finalize().into_bytes()))
    }

    /// The user of a valid, unexpired token.
    pub fn verify_token(&self, token: &str) -> Option<String> {
        let (signed, mac_hex) = token.rsplit_once('.')?;
        let mut mac = self.mac();
        mac.update(signed.as_bytes());
        // Constant-time comparison; nothing below runs for a forged token.
        mac.verify_slice(&from_hex(mac_hex)?).ok()?;
        let mut parts = signed.split('.');
        let (Some("v1"), Some(user_hex), Some(expiry), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return None;
        };
        if now() >= expiry.parse::<u64>().ok()? {
            return None;
        }
        String::from_utf8(from_hex(user_hex)?).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passwords_and_tokens() {
        let auth = Authenticator::new("admin", "open sesame", &[7; 32]).unwrap();
        assert!(auth.check_password("admin", "open sesame"));
        assert!(!auth.check_password("admin", "open sesam"));
        assert!(!auth.check_password("nobody", "open sesame"));
        assert!(!auth.check_password("nobody", "no user has this password"));

        let token = auth.issue_token("admin");
        assert_eq!(auth.verify_token(&token).as_deref(), Some("admin"));
        let other = Authenticator::new("admin", "open sesame", &[8; 32]).unwrap();
        assert_eq!(other.verify_token(&token), None, "signed with another key");
        assert_eq!(auth.verify_token("v1.61.99999999999.00"), None);
        assert_eq!(auth.verify_token(""), None);
    }
}
