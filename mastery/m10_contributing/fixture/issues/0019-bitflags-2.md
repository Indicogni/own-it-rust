# 19: Move the library crates to bitflags 2

**Labels:** `deps` · **State:** open

---

**mira** (maintainer) opened this issue

`tasklane-cli` uses bitflags 2 since 0.4.0, but `tasklane-plugin` (and through it
`tasklane-core`) is still on 1.3.2. So every build of the command compiles bitflags
twice:

```text
$ cargo tree --duplicates
bitflags v1.3.2
└── tasklane-plugin v0.4.0
    ...

bitflags v2.13.2
└── tasklane-cli v0.4.0
```

I did the command line first, in 0.4.0, where breaking changes were allowed anyway;
its `Show` type appears in no signature of the two library crates. `TaskFlags` is
different: it is part of the public API that plugins build on, and we are in 0.4.x
now, so the upgrade must not change what users of `TaskFlags` can do with it. The bitflags changelog for 2.0.0
has a section about exactly that; read it before you start.

Wanted:

- one bitflags version for the whole workspace, declared once in
  `[workspace.dependencies]`;
- `TaskFlags` keeps every trait and method that 0.4.0 users can rely on;
- nothing changes in what `tasklane list` prints;
- a changelog entry under `Changed`.

---

**ada** (maintainer) commented

Careful with the second and third point. The compiler will tell you about some of
the differences and stay silent about others.
