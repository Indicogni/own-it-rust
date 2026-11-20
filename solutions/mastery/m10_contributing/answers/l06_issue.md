<!-- Lesson 06, task C: issue 14 as its reporter should have written it. You know the
     minimal case from lesson 02; write the report that lets a maintainer start
     working without a single question back. -->

# `$` followed by a character loses that character: `$3` becomes `$`

## Steps to reproduce

Save this as `Lanefile`:

```text
task menu
  step write menu.txt coffee $3
```

Then run:

```text
tasklane run menu --show files
```

## Expected

`file menu.txt (9 bytes)`: the file contains `coffee $3`. The documentation of
`expand` says that a `$` which is not part of `${NAME}` or `$$` "is kept as it is".

## Actual

`file menu.txt (8 bytes)`: the file contains `coffee $`. The character after the `$`
is gone. It happens with any character after the `$`, not only digits: `a$b` gives
`a$`. `$$` and `${NAME}` work as documented.

## Version

tasklane 0.4.0 (`tasklane --version`), built from the release tag with Rust 1.99.0 on
Linux x86-64. I did not try older versions.
