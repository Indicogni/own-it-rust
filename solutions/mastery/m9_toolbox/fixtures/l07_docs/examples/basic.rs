//! Prints the slug of a title: `cargo run --example basic`.

fn main() {
    let title = "The Professional Toolbox";
    println!("{}", slugs::slugify(title));
}
