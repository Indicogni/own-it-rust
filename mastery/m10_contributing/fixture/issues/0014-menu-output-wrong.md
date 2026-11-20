# 14: menu output wrong

**Labels:** `bug`, `needs-repro` · **State:** open

---

**hungry-dev** opened this issue

hi, since I moved our cafe's menu build to tasklane the generated menu is wrong, lots
of things are missing in the file!! it worked with my old shell script. This is my
Lanefile (sorry it's long, I removed nothing because I don't know what matters):

```text
set SHOP = Corner Cafe
set OUT = out
set DAY = Tuesday

task clean
  flags silent
  step remove ${OUT}/*

task header : clean
  step write ${OUT}/00-header.txt ${SHOP}
  step append ${OUT}/00-header.txt "Menu for ${DAY}"
  step upper ${OUT}/00-header.txt

task drinks : clean
  step write ${OUT}/10-drinks.txt Drinks
  step append ${OUT}/10-drinks.txt coffee $3, tea $2
  step append ${OUT}/10-drinks.txt "juice (small) $2, juice (large) $4"

task food : clean
  step write ${OUT}/20-food.txt Food
  step append ${OUT}/20-food.txt soup of the day
  step append ${OUT}/20-food.txt "cake $4.50 a slice"

task offers : drinks food
  flags optional
  step write ${OUT}/30-offers.txt Offers
  step append ${OUT}/30-offers.txt "100$ voucher for 90$ - ask at the counter"

task menu : header drinks food offers
  step concat ${OUT}/menu.txt "${OUT}/*-*.txt"
  step count ${OUT}/menu.txt
```

I run `tasklane run menu`. Using the latest version. Please fix asap, we print the
menu every morning.

---

**ada** (maintainer) commented

Thanks for the report. I can see that something is wrong in the file you get, but
there is a lot going on in this Lanefile: five tasks, three variables, quotes, a
pattern. Could you reduce it to the smallest Lanefile that still shows the problem,
and tell us what you expected in the file and what you got? Which version is "the
latest" (`tasklane --version`)?

Labelling `needs-repro` until then.

---

**hungry-dev** commented

it's 0.4.0. I don't have time to reduce it, the prices are just gone!! "coffee $,
tea $" who wants that

---

**jun** (maintainer) commented

"The prices are gone" is a useful hint, thank you. If somebody wants to pick this up:
a minimal reproduction and a failing test would be the first step. Please do not
change anything else about how `${...}` works; a lot of Lanefiles depend on it.
