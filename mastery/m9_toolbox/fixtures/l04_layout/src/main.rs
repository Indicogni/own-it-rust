fn main() {
    let n: i64 = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(0);
    println!("{}", layout::double(n));
}
