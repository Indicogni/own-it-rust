use std::io::Read;

fn main() -> std::io::Result<()> {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text)?;
    let counts = shipit::count(&text);
    println!("{} {} {}", counts.lines, counts.words, counts.bytes);
    Ok(())
}
