//! Variables and `${NAME}` expansion.

use std::collections::BTreeMap;

use crate::error::Error;
use crate::model::Lanefile;

/// A set of variables. Setting a name again replaces its value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vars {
    values: BTreeMap<String, String>,
}

impl Vars {
    /// Creates an empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// The variables of a Lanefile's `set` directives.
    pub fn from_lanefile(file: &Lanefile) -> Self {
        let mut vars = Vars::new();
        for (name, value) in &file.vars {
            vars.set(name, value);
        }
        vars
    }

    /// Sets `name` to `value`.
    pub fn set(&mut self, name: &str, value: &str) {
        self.values.insert(name.to_string(), value.to_string());
    }

    /// The value of `name`.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }

    /// The number of variables.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether no variable is set.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Replaces every `${NAME}` in `template` with the value of `NAME`.
///
/// - `$$` gives one `$`. Use it to write a literal `${`: `$${NAME}` gives `${NAME}`.
/// - Any other `$` is kept as it is.
/// - Values are inserted as they are. A `${...}` inside a value is not expanded again.
///
/// # Errors
///
/// [`Error::UnknownVariable`] if a name is not set, and
/// [`Error::UnterminatedVariable`] if a `${` has no closing `}`.
///
/// ```
/// use tasklane_core::{Vars, expand};
///
/// let mut vars = Vars::new();
/// vars.set("OUT", "build");
/// assert_eq!(expand("${OUT}/index.txt", &vars).unwrap(), "build/index.txt");
/// assert_eq!(expand("$${OUT}", &vars).unwrap(), "${OUT}");
/// ```
pub fn expand(template: &str, vars: &Vars) -> Result<String, Error> {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars();
    while let Some(c) = chars.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('{') => {
                let mut name = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(part) => name.push(part),
                        None => return Err(Error::UnterminatedVariable(template.to_string())),
                    }
                }
                let value = vars.get(&name).ok_or(Error::UnknownVariable(name))?;
                out.push_str(value);
            }
            Some('$') => out.push('$'),
            _ => out.push('$'),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Vars {
        let mut vars = Vars::new();
        vars.set("A", "apple");
        vars.set("B", "bee");
        vars
    }

    #[test]
    fn later_set_wins() {
        let mut vars = vars();
        vars.set("A", "ant");
        assert_eq!(vars.get("A"), Some("ant"));
        assert_eq!(vars.len(), 2);
    }

    #[test]
    fn expands_names() {
        assert_eq!(expand("${A} and ${B}", &vars()).unwrap(), "apple and bee");
        assert_eq!(expand("${A}${B}", &vars()).unwrap(), "applebee");
        assert_eq!(expand("no variables", &vars()).unwrap(), "no variables");
    }

    #[test]
    fn double_dollar_is_an_escape() {
        assert_eq!(expand("$${A}", &vars()).unwrap(), "${A}");
        assert_eq!(expand("a$$", &vars()).unwrap(), "a$");
    }

    #[test]
    fn errors() {
        assert_eq!(
            expand("${C}", &vars()),
            Err(Error::UnknownVariable("C".to_string()))
        );
        assert_eq!(
            expand("x${A", &vars()),
            Err(Error::UnterminatedVariable("x${A".to_string()))
        );
    }

    #[test]
    fn from_lanefile_applies_sets_in_order() {
        let file = Lanefile {
            vars: vec![("X".into(), "1".into()), ("X".into(), "2".into())],
            tasks: vec![],
        };
        assert_eq!(Vars::from_lanefile(&file).get("X"), Some("2"));
    }
}
