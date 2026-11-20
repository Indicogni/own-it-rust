# 30: `summary_names_failed_tasks` fails every other run

**Labels:** `bug`, `flaky-test`, `good first issue` · **State:** open

---

**jun** (maintainer) opened this issue

`crates/tasklane-core/tests/report.rs`, test `summary_names_failed_tasks`. On CI and
on my machine it fails roughly every second run:

```text
assertion `left == right` failed
  left: "1 done, 2 failed, 1 skipped; failed: test (fail: 2 tests failed), lint (fail: style)"
 right: "1 done, 2 failed, 1 skipped; failed: lint (fail: style), test (fail: 2 tests failed)"
```

Run it ten times to see it:

```text
for i in 1 2 3 4 5 6 7 8 9 10; do cargo test -q -p tasklane-core --test report; done
```

The two failed tasks come out in either order. The documentation of
`RunReport::summary` says "The failed tasks are listed in plan order", and `lint`
is planned before `test`, so the test asserts the documented behaviour.

Marked `good first issue`: the cause should be easy to find once you look at how
`summary` collects the failed tasks.

---

**ada** (maintainer) commented

To be clear about what we want: the test is right. Please do not "fix" this by
making the test accept both orders, by sorting in the test, or by ignoring the test.
Users see this line in their CI logs and compare logs between runs.
