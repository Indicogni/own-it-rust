# 29: slice prints garbage???

**Labels:** `needs-info` · **State:** open

---

**anon-4471** opened this issue

used the new slice step to cut the first chars of a file and got weird symbols in
the output file one time, like other text that was not in my file + some
unprintable stuff. cant reproduce it now. the numbers in my lanefile were maybe a bit
off because the input file was shorter that day. is the memory broken??

---

**mira** (maintainer) commented

Thanks. Without the Lanefile and the input we cannot do much with this. Which
numbers did you pass to `slice`, and how long was the file? `slice` should give an
error when the range is outside the file:

```text
slice: bytes 3+100 are outside `in.txt`
```

Labelling `needs-info`. If nobody can reproduce this, we will close it in a month.

---

**anon-4471** commented

dont have the file anymore sorry
