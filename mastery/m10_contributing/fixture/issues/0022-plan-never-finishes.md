# 22: `plan` never finishes for our generated Lanefile

**Labels:** `bug`, `performance`, `regression` · **State:** open

---

**pipeline-kai** opened this issue

**Version:** 0.4.0. With 0.3.0 the same file is planned at once.

**What I do.** Our build has stages. Every stage has a "left" and a "right" task, and
both depend on both tasks of the stage before. The Lanefile is generated; this
script prints one with N stages:

```sh
n=48
echo "task l0"; echo "task r0"
i=1
while [ "$i" -le "$n" ]; do
  p=$((i - 1))
  echo "task l$i : l$p r$p"
  echo "task r$i : l$p r$p"
  i=$((i + 1))
done
```

```text
sh ladder.sh > ladder.lane
tasklane -f ladder.lane plan l48
```

**Expected:** 97 task names, immediately, as with 0.3.0.

**Actual:** no output. One CPU core is at 100 %. I stopped it after ten minutes.

**What I found out.** The time doubles with each stage: with `n=22` it takes about
half a second on my machine, with `n=24` about two seconds, with `n=26` about nine.
`tasklane run` and `tasklane check` hang in the same way, so I think it is the
planning and not the running.

---

**jun** (maintainer) commented

Confirmed, thank you for the measurements and the generator; that is everything we
need. Doubling per stage smells like the same sub-plan being computed again and
again. The planner was rewritten in 0.4.0 for the cycle path (#15), so that is where
I would look.

A fix must keep two things as they are: the order of the plan (dependencies in the
order they are written, every task once) and the cycle errors with their full path.
