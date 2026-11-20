# Chapter 1 hints — Start here

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Each lesson has three hints. Open the first one only after a real attempt, the second one only if the first was not enough, and the third one only if you are still stuck: the third hint is nearly the answer. *Check your explanation* is a model answer to the lesson's EXPLAIN prompt; read it after you have written your own.

## 01 · Install, verify, and a first green test

<details><summary>Hint 1</summary>

Each task is solved by replacing the whole `todo!(...)` with one value. The value is the last thing in the function body, and it has no semicolon after it.

</details>
<details><summary>Hint 2</summary>

For A, a text is written between double quotes. For B, run `rustc --version` in a terminal that stands in the course directory; the version looks like `1.N.0`, and the task asks for `N` as a plain number without quotes. For C, the two parameters are called `a` and `b`, and `+` adds them.

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

A: the body is `"ready"`. B: the body is the middle number that you read from `rustc --version`, written as a number, such as `42` (but use yours). C: the body is `a + b`. For the predictions: `*` is done before `+`, and several `-` are done from left to right.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`todo!()` and `guess()` are placeholders that let an unfinished file compile. When the program reaches one of them, it panics: `todo!()` with the message "not yet implemented", `guess()` with a message that starts with "PREDICT". A test that panics is reported as `FAILED`. The block for that test names the file and the line of the placeholder, so the report shows exactly which task or prediction is still open. After I replaced one `todo!()` with a value, the test for that task changed to `ok`, and the last line of the report counted one more passed test.

</details>

## 02 · Cargo: new, run, check, build, test

<details><summary>Hint 1</summary>

The answers are not in the lesson file. Do the lab from CONCEPTS.md in a directory outside the course, and look at your terminal and at the files of the new package after each command.

</details>
<details><summary>Hint 2</summary>

For A, `cargo run` prints two or three lines that come from Cargo (they start with `Compiling`, `Finished`, `Running`) and then one line that comes from the program; that last line is the answer. For B and C, list the package directory before and after the first `cargo check`: one directory and one file are new. For D, find the row of the command table in CONCEPTS.md that says "Produces nothing that can run".

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

A: open `src/main.rs` in your lab package; the answer is the text between the quotes of `println!`, exactly as written. B: the directory name has six letters and starts with `t`. C: the file has the same first word as `Cargo.toml` and ends in `.lock`. D: `cargo` followed by the five-letter word that means "examine". P1: open the course's own `Cargo.toml` and copy the text between the quotes of `rust-version`. P2: list the top directory of the course and look.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`cargo check` runs the compiler far enough to find every error, but it produces no program; it is the fastest command and the right one while writing. `cargo build` compiles completely and leaves an executable in `target/debug/`. `cargo run` does a build if something changed and then starts that executable. `cargo test` builds a version of the package that runs all functions marked `#[test]` and prints a report. `target/` holds everything Cargo has built. It can be deleted at any time, and the only cost is the time to build again. `Cargo.lock` records the exact versions of all packages used in the build, so that the build can be repeated; Cargo writes it, and it should be kept and not edited.

</details>

## 03 · Reading a failing test

<details><summary>Hint 1</summary>

Do not search the functions for mistakes. Run the lesson and read one failure block at a time: the line after `panicked at` names a file and a line, and the message below it says what was wrong there.

</details>
<details><summary>Hint 2</summary>

For A, compare `left` (what `seconds_in` returned) with `right` (what it should return): by which factor are they apart? For B, the message is a sentence about one exact age; which comparison treats that age wrongly? For C, the line number in the report is not in the test, it is inside the function: the function did something that is not allowed for one of the inputs. For D, look at which of the two numbers came back.

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

A: a minute has 60 seconds, not 6. B: "from the age of 18" includes 18, so the comparison must be `>=`. C: dividing by zero panics; check for `people == 0` first and give 0 in that case, with `if people == 0 { ... } else { ... }` or with an early `return 0;`. D: the two branches of the `if` are exchanged. P1: `left` is the first value inside `assert_eq!`, so work out `triple(4)`; write the answer as a pair, `(left, right)`. P2: work out each of the four checks by hand.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Each failed test has its own block, which starts with the full name of the test between dashes. The next line says `panicked at` and gives the file, the line and the column of the panic. Below it stands the message. For a failed `assert_eq!` the message shows `left`, the value my function produced, and `right`, the expected value. The report of task A pointed into the test and showed two numbers, so the function had returned a wrong value. The report of task C had no `left` and `right`. Its line was inside `share_per_person`, and its message was "attempt to divide by zero": the function itself had panicked before the test could compare anything.

</details>

## 04 · Reading a compiler error

<details><summary>Hint 1</summary>

Every answer is printed in one of the three messages at the top of the file. Find the part of the message that the task asks about: the headline, the line with the arrow `-->`, the label next to the `^^^` marks, or the `help` section.

</details>
<details><summary>Hint 2</summary>

For A, the code stands in square brackets directly after the word `error`. For B, the arrow line ends in `file:line:column`. For C, the help section prints the repaired line; compare it with the original line above it. For D, the label next to the `^^^^^^` marks has the form "expected ..., found ...".

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

A: a text of five characters that starts with `E`, in double quotes. B: a pair such as `(7, 12)`, with the two numbers after `src/main.rs:`. C: the name with five letters that the help writes where the original had `totl`. D: a pair of two texts; the first is the type with a `u` and a number, the second starts with `&`. P1: count the headlines that start with `error[` and the headlines that start with `warning:` and describe the program; the last two lines only sum up. P2: the line number in the first arrow line, from the top.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

I read a compiler message from the top. The headline gives the level (error or warning), the error code, and one sentence that says what is wrong. The arrow line gives the file, the line and the column. Then come the source lines with marks: `^^^` under the place of the error, `---` under other places that explain it, each with a label. After that, `note` gives background and `help` proposes a change. The first error comes first because later errors are often only consequences of it: once a name is unknown or a type is wrong, the lines that use it cannot be right either. So I repair the first error, compile again, and read the new output.

</details>

## 05 · The lesson loop: predict, implement, explain

<details><summary>Hint 1</summary>

All three functions are one line of arithmetic or one comparison. For C, write the calculation on paper with the numbers from the failing test before you write it in Rust.

</details>
<details><summary>Hint 2</summary>

For B, an even number leaves the remainder 0 when it is divided by 2, and `%` gives the remainder. For C, the tip is `bill * percent / 100`; the order matters, because whole-number division drops the rest. For D, copy the shape of a test from CONCEPTS.md into `mod my_tests`: the line `#[test]`, then `fn some_name() { ... }` with one `assert_eq!` inside.

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

A: `items * unit_price`. B: `n % 2 == 0`; do not compare with 1, because the remainder of a negative odd number is -1. C: `bill + bill * percent / 100`. D: for example a test named `tip_on_one_cent_is_dropped` with the line `assert_eq!(with_tip(1, 10), 1);`. P1: 7 = 3 × 2 + 1. P2: work out each product or quotient from left to right, and drop the rest after every division.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Division between whole numbers drops the rest. In `bill * percent / 100` the multiplication comes first, so the full product is divided once and only the last part of a cent is lost. In `bill / 100 * percent` the division comes first and already drops up to 99 cents of the bill before the multiplication, so for a bill of 50 cents the tip is 0. The supplied tests check a few ordinary bills and one small one. My own test checks a case they do not have, for example a bill of 1 cent, where 10 percent is less than a cent and the total must stay 1.

</details>

## 06 · Capstone: the parking meter

<details><summary>Hint 1</summary>

Task A is one line of arithmetic. Task B has two cases, short stays and long stays, so it needs an `if` with an `else`. Task C already has a body: run the tests and read the message of the one that fails.

</details>
<details><summary>Hint 2</summary>

For B, decide first whether `minutes` is larger than 30. Only in that case subtract 30 and multiply by 2; in the other case the fee is 0. A `u32` cannot hold a negative number, so `minutes - 30` must not run for a stay of 10 minutes. For C, the failing check is about exactly 120 minutes.

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

A: `hours * 60 + minutes`. B: `if minutes > 30 { (minutes - 30) * 2 } else { 0 }`. C: "at most 120" includes 120, so the comparison is `<=`. D: one `#[test]` function in `mod my_tests`, for example a stay of 24 hours: `assert_eq!(total_minutes(24, 0), 1440);`. P2: is 30 larger than 30?

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A model answer for task B: "I predicted P2 wrongly, because I read `minutes > 30` as true for 30. My first version of `fee_in_cents` was one line, `(minutes - 30) * 2`. The report for the test about the free half hour did not show `left` and `right`. It said 'attempt to subtract with overflow' and pointed at my line, so the function had panicked: for 10 minutes the subtraction has no valid `u32` result. I put the subtraction inside an `if minutes > 30`, and the test passed. The step I wanted to skip was the prediction, because the code looked easy. Without it I would not have noticed that I was unsure about what happens at exactly 30."

</details>

## Drills

Run `cargo drill ch01` after lesson 04. The compiler output tells you *what* happened; these notes say *why*. Read a note only after you have seen the verdict for that drill.

<details><summary>d01 · A name the compiler does not know (lesson 04)</summary>

`E0425`. `peers` is not declared anywhere; the variable is called `pears`. The compiler even proposes the similar name. Correct the spelling.

</details>
<details><summary>d02 · Text where a number is expected (lesson 04)</summary>

`E0308`, mismatched types. `"4"` in quotes is a text, and `seats` is declared as a `u32`. Rust never turns a text into a number by itself. Write the number without quotes.

</details>
<details><summary>d03 · Changing a variable (lesson 04)</summary>

`E0384`. A variable declared with `let` alone cannot be changed after its first value. Declare it with `let mut score`. The help section of the message proposes exactly this.

</details>
<details><summary>d04 · Three lets, one name (lesson 04)</summary>

Compiles and runs. Every `let price` makes a **new** variable that takes over the name; this is called shadowing. Nothing is changed in place, so no `mut` is needed. Compare with d03: assigning with `=` alone needs `mut`, and a new `let` does not. Chapter 2 comes back to this.

</details>
<details><summary>d05 · Where does the statement end? (lesson 04)</summary>

A compile error without a code, so the answer is `error`. The semicolon after `let width = 6` is missing. The compiler notices the problem when it meets the next `let`, but its primary mark and its help point at the end of the line before, where the semicolon belongs.

</details>
<details><summary>d06 · One missing character (lesson 04)</summary>

`E0423`. `println` is a macro, and a macro is called with `!` after its name: `println!(...)`. Without the `!`, the compiler looks for a function called `println` and finds none. The help section says "use `!` to invoke the macro".

</details>
<details><summary>d07 · Calling a function (lesson 04)</summary>

`E0061`. `area` is declared with two parameters, and the call passes one value. The message names the missing argument and its type. The KEEP line expects 20, so the second value must be 5.

</details>
<details><summary>d08 · A function that returns nothing (lessons 01 and 04)</summary>

`E0308`. The semicolon after `n * 2` turns the last line into a statement, so the body produces no value, and the signature promises an `i32`. The message says "expected `i32`, found `()`"; `()` is how Rust writes "no value". Remove the semicolon.

</details>
<details><summary>d09 · It compiles. Is it right? (lesson 03)</summary>

Panics at run time. The program is correct Rust, so the compiler accepts it. The assertion then compares 12 with 120 and fails, and the report shows `left: 12` and `right: 120`. The compiler checks that the types fit; it cannot know that an hour has 60 minutes. That is what tests are for.

</details>
<details><summary>d10 · A first look at ownership (preview of Chapter 4)</summary>

`E0382`. A `String` has one owner. `let badge = name;` hands the text over to `badge`, and `name` holds nothing afterwards, so the next line may not read it. The help section proposes `name.clone()`, which makes a second, separate text. This rule is the subject of Chapter 4; for now it is enough to have seen the message once.

</details>

## Mastery check: model answers

Answer the questions in the [chapter guide](README.md#mastery-check) from memory first. Then compare.

<details><summary>Model answers (after writing your own)</summary>

**1. Which Rust version, and why one fixed version?**
`rustc --version`, typed in a terminal that stands in the directory in question, prints the version that is used there. The course directory contains `rust-toolchain.toml`, which names version 1.99.0; rustup reads the file and installs and selects that toolchain. With one fixed version, every compiler message and every behaviour is the same for every learner and the same as in the teaching text.

**2. The Cargo commands, `target/` and `Cargo.lock`.**
`cargo check` only looks for errors. `cargo build` compiles to an executable. `cargo run` builds if needed and runs the program. `cargo test` builds and runs the tests. `target/` is where Cargo puts everything it builds; it may be deleted, and it is built again on the next command. `Cargo.lock` records the exact versions used in the build; it is kept and not edited by hand.

**3. Reading a failed test.**
The block for the test starts with its name between dashes. The line with `panicked at` gives the file and the line. A failed `assert_eq!` prints `left` (in this course: what my code produced) and `right` (what was expected). The three kinds of failure are: a wrong value (two values are printed, the line is in the test), a false `assert!` (a sentence from the test's author, the line is in the test), and a panic inside my own function (another message, such as "attempt to divide by zero" or "not yet implemented", and the line is in my function).

**4. Reading a compiler message.**
Headline with level and code; arrow line with file, line and column; source lines with primary marks `^^^` and secondary marks `---` and their labels; `note` and `help`; closing lines with `rustc --explain` and the count. I repair the first error first, because later errors are often caused by it, and then I compile again and read the new output.

**5. The lesson loop.**
Read the section in CONCEPTS.md. Predict: answer every `guess()` before running anything. Implement the `todo!()` bodies until the lesson is green. Explain: write down the wrong predictions and their reasons, answer the EXPLAIN question, add a test of my own, and compare with the model explanation. The prediction comes first because an answer that I had to commit to shows me where my picture of the language is wrong; if I run first, I only copy numbers.

**6. The capstone again.**
There is no model text for this one. Use `cargo progress redo ch01 l06` to get the untouched file back (your solution is saved first), solve it again, and compare the two versions.

</details>
