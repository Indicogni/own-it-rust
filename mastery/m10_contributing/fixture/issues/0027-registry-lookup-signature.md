# 27: `Registry::lookup` takes `&String`

**Labels:** `api`, `breaking-change`, `0.5` · **State:** open

---

**rustacean-bo** opened this issue

```rust
pub fn lookup(&self, name: &String) -> Option<&Box<dyn Step>>
```

Clippy complains twice about this signature (`ptr_arg`, `borrowed_box`), and it is
awkward to call with a string literal. It should be

```rust
pub fn lookup(&self, name: &str) -> Option<&dyn Step>
```

---

**ada** (maintainer) commented

Agreed that it is a wart; it has been there since 0.1 and I wrote it. But the return
type is part of the signature, and plugins store the result in variables of that
type. Changing it is a breaking change, so it cannot happen in 0.4.x.

The plan for when somebody has time:

1. In a 0.4.x release, add a new method with the good signature (the name `get` was
   suggested) and mark `lookup` as `#[deprecated]` with a note that points to it.
   Nobody's build breaks; users get a warning and time to move.
2. In 0.5.0, remove `lookup`.

Until then the `#[allow]` on `lookup` stays, with a comment that points here.

---

**rustacean-bo** commented

That is a lot of ceremony for five lines.

---

**ada** (maintainer) commented

It is. The ceremony is for the people whose builds we would otherwise break on a
patch update.
