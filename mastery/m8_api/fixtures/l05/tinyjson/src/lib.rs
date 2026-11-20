//! A very small JSON writer. You do not need to edit this file.

/// A JSON object whose values are all numbers, with the members in the given
/// order: `object(&[("r", 255), ("g", 0)])` is `{"r":255,"g":0}`.
pub fn object(members: &[(&str, u32)]) -> String {
    let members: Vec<String> = members
        .iter()
        .map(|(name, value)| format!("\"{name}\":{value}"))
        .collect();
    format!("{{{}}}", members.join(","))
}

#[cfg(test)]
mod tests {
    #[test]
    fn objects_keep_their_order() {
        assert_eq!(super::object(&[("r", 255), ("g", 0)]), r#"{"r":255,"g":0}"#);
        assert_eq!(super::object(&[]), "{}");
    }
}
