# slugs

Turns titles into URL slugs: lower-case ASCII letters and digits, with single
dashes between words.

```rust
use slugs::{Slug, slugify};

assert_eq!(slugify("Hello, World!"), "hello-world");

let slug = Slug::new("Own It: Hands-On Mastery of Rust").expect("the title has letters");
assert_eq!(slug.as_str(), "own-it-hands-on-mastery-of-rust");
```

## Features

- `titles` (off by default): adds `title_case`, which turns a slug back into a
  title.

## Examples

`cargo run --example basic` prints the slug of a title.

See [CHANGELOG.md](CHANGELOG.md) for the release history.
