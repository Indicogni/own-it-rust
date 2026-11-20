# M10 notes — Working in real codebases and contributing

Your workbook. Write answers in your own words, *before* checking the hints. When you
come back to this module for review, answer the mastery questions at the bottom
again from memory, then compare with what you wrote here.

## l01 · Orienting in an unfamiliar codebase

**My map of tasklane (crates, modules, one path), drawn before answering:**

> 

**Predictions I got wrong, and why:**

- 

**Explain:** Describe the path of one Lanefile through tasklane, from the command line to the summary line, naming the crate and module of each stage. Say which of your lookups was fastest with which tool.

> 

**Something in the fixture that surprised me:**

- 

## l02 · Reproducing a bug report

**My reduction steps (what I deleted, and whether the problem stayed):**

- 

**Predictions I got wrong, and why:**

- 

**Explain:** State the bug in one sentence that names the input, the expected output and the actual output. Explain why each `b_unchanged_` test exists and which careless fix it would catch.

> 

**Edge case I added to the fixture's tests:**

- 

## l03 · Regression hunting with git bisect

**The probe test I wrote, and how I checked it on the good and the bad commit:**

> 

**Predictions I got wrong, and why:**

- 

**Explain:** Explain why the plain run blamed the wrong commit, what exit code 125 means, and why bisecting needs a history in which most commits build.

> 

**What the first bad commit changes, in my words:**

- 

## l04 · Making a change the maintainers will accept

**The shape the maintainers accepted, copied from the issue before I wrote code:**

- 

**Predictions I got wrong, and why:**

- 

**Explain:** Explain why `RunOptions` could grow and `TaskStatus` could not, and what you would have had to do if the feature had needed a new status.

> 

**Edge case I added to the fixture's tests:**

- 

## l05 · Code review, both sides

**My review of pull request 33, written before opening `candidate-comments.md`:**

- Patch 1: 
- Patch 2: 
- Patch 3: 
- Patch 4: 

**Claims I checked by compiling or running, and what I found:**

- 

**Predictions I got wrong, and why:**

- 

**Explain:** Pick the comment you found hardest to judge and explain your verdict. Then explain why declining a request can be the helpful answer.

> 

## l06 · Writing for maintainers

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what belongs in the commit message and what belongs only in the pull request. Then name the two sentences of your issue report that save the maintainer the most time.

> 

**A commit message from my own past work that I would now write differently:**

> 

## l07 · Unsafe code and soundness reports

**Every requirement of every unsafe call in the block, and where (or whether) it is checked:**

- 

**What Miri printed for my exposing call (first two lines):**

> 

**Predictions I got wrong, and why:**

- 

**Explain:** State the safety requirement of the `unsafe` block as it was written, the requirement it really has, and which of the two problems of the function Miri can see and which it cannot.

> 

## l08 · Dependencies and upgrades

**What the changelog told me to expect, before I built anything:**

- 

**Predictions I got wrong, and why:**

- 

**Explain:** Explain which change the compiler found for you, which change only a test found, and why this upgrade could have been a breaking change for the users of tasklane although tasklane's own source hardly changed.

> 

## l09 · Contributing to the wider ecosystem

**Predictions I got wrong, and why:**

- 

**Explain:** Describe the path of a new standard-library function from an idea to a stable release, naming each document or issue that is created on the way. Then say which of the project's rules surprised you.

> 

**My plan for a first real contribution:**

- Project, and why I use it: 
- Its CONTRIBUTING file says I must: 
- Its rule about LLM assistance, or "none stated": 
- The issue or gap I chose, and why it needs no design decision: 
- Built from source and tests run on (date): 
- First step I will take this week: 

## l10 · Capstone

**The issue I chose, and why:**

> 

**Predictions I got wrong, and why:**

- 

**Explain:** Tell the story of your contribution in five sentences: the report, the reproduction, the cause, the fix, and how you know nothing else changed.

> 

**My rubric score (README, "Capstone rubric"), and the step I would redo:**

- 

## Mastery check (answer from memory)

**Open a project you have never seen and, within an hour, say what each crate is for, follow one input from the entry point to the output, and name the test that pins a behaviour you were asked about.**

> 

**Turn a report without a reproduction into a test of a few lines that fails for the reported reason, and say what a fix must not change.**

> 

**Find the commit that introduced a regression with `git bisect run`, including in a history where some commits do not build, and explain the exit codes.**

> 

**Look at a proposed API change and say whether it breaks a user, why, and which design would not.**

> 

**Review a patch: separate what blocks a merge from what is a suggestion, check a claim before you post it, and answer a review of your own work, including a request you decline.**

> 

**Write a commit message, a pull request description and an issue report that a maintainer can act on without asking back.**

> 

**Explain what makes a safe function unsound, show it with a test under Miri, and say what Miri cannot see.**

> 

**Upgrade a dependency across a breaking release: read its changelog, find the changes that the compiler reports and the ones it does not, and keep your own users unbroken.**

> 

**Describe how a change reaches stable Rust (RFC or ACP, implementation behind a feature gate, FCP, stabilisation), where the rules are written, and what the project asks of contributors who use an LLM.**

> 

**Redo the capstone with a different issue, after a break, without looking at your first solution.**

>
