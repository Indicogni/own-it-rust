# Which of these blocks does rustdoc test?

This file is the input of the predictions of lesson 06. You do not need to
edit it. `rustdoc --test fixtures/l06/fences.md` reads it the way it reads a
documentation comment.

Block 1, no label:

```
assert_eq!(2 + 2, 4);
```

Block 2, labelled `rust`:

```rust
assert_eq!("a-b".len(), 3);
```

Block 3, labelled `text`:

```text
assert_eq!(1, 2);
```

Block 4, labelled `ignore`:

```ignore
this is not even Rust
```

Block 5, labelled `no_run`:

```no_run
loop {
    std::thread::park();
}
```

Block 6, labelled `should_panic`:

```should_panic
let empty: Vec<u8> = Vec::new();
let first = empty[0];
```

Block 7, labelled `compile_fail`:

```compile_fail
let text: String = 5;
```

Block 8, indented by four spaces, without a fence:

    assert!("docs".starts_with('d'));

Block 9, labelled `sh`:

```sh
cargo test --doc
```
