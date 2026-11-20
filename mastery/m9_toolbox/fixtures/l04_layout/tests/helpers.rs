//! Shared helper code, as a file directly in `tests/`.

#[allow(dead_code)]
pub fn trimmed(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_owned()
}
