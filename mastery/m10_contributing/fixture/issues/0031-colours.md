# 31: Colours and emoji in the output

**Labels:** `feature`, `declined` · **State:** closed

---

**shiny-cli-fan** opened this issue

The output is boring. Every modern tool has colours and a green check mark for
success. Please add this, it is easy with a crate like the ones every other project
uses. +1 if you agree!

---

**ada** (maintainer) commented

Thank you for the suggestion. We are going to decline it. tasklane's output is read
by other programs and compared between CI runs, and the project keeps its dependency
list very short on purpose. A wrapper script can colour the summary line if you want
that locally.

Closing as declined. This is a decision about scope, not about the idea being bad.
