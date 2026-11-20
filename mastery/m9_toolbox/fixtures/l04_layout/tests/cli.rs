mod helpers;

#[test]
fn prints_the_double() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_layout"))
        .arg("21")
        .output()
        .expect("the binary runs");
    assert_eq!(helpers::trimmed(&output.stdout), "42");
}
