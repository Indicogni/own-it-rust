# Chapter 2 hints — A first tour

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Each lesson has three hints. Open the first one only after a real attempt, the second one only if the first was not enough, and the third one only if you are still stuck: the third hint is nearly the answer. The rules in each task's doc comment ("use a `while` loop", "call `square` twice") are part of the exercise. *Check your explanation* is a model answer to the lesson's EXPLAIN prompt; read it after you have written your own.

## 01 · Variables, mutability and basic types

<details><summary>Hint 1</summary>

For A, how many full hundreds are in the price? Whole-number division answers that. For C, the two inputs are `i32` and the result is `f64`: something must be converted, and it matters when.

</details>
<details><summary>Hint 2</summary>

A: `price / 100` is the number of full hundreds; each costs 10 cents. B: declare `let mut total = balance;`, write `total += deposit;` three times, and end with `total` on a line of its own. C: `f64::from(a)` converts one number; convert both before you add, then divide by `2.0`. D: chars can be compared like numbers, `c >= 'A'`.

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

A: `let fee = price / 100 * 10;` then `price + fee`. C: `(f64::from(a) + f64::from(b)) / 2.0`. D: `c >= 'A' && c <= 'Z'` works. Clippy then proposes the range form `('A'..='Z').contains(&c)`; the method `c.is_ascii_uppercase()` says the same in one word. P1: the `x` inside the braces is a separate variable that ends with the block.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A `mut` variable is one variable whose value is replaced: `total += deposit` stores a new number in the same place, and the type stays the same. Shadowing makes a second variable that takes over the name: `let x = x * 2;` leaves the old variable untouched and hides it from that line on. The new variable may have another type, and when it was declared inside a block, the old one is visible again after the block. Rust makes you write `mut` because most variables never change. With the changing ones marked, a reader knows which values to watch, and the compiler can reject a change that was not intended.

</details>

## 02 · Functions, expressions and statements

<details><summary>Hint 1</summary>

Every body in this lesson is one expression, or a few `let` lines followed by one expression. The last line has no semicolon.

</details>
<details><summary>Hint 2</summary>

B: the body calls `square` twice and adds the two results. C: write every number with a decimal point (`9.0`, `5.0`, `32.0`); Rust does not mix `f64` with whole numbers. D: a tuple is written in parentheses with a comma, and the perimeter comes first.

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

A: `n * n`. B: `square(a) + square(b)`. C: `celsius * 9.0 / 5.0 + 32.0`. D: `(2 * (width + height), width * height)`. P2: work out the inner call first; its result is the argument of the outer call. P3: the `x` inside the block is a new variable; the outer `x` never changes.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

An expression produces a value: `n * n`, a function call, an `if` with an `else`, a block. A statement does something and produces no value: a `let` line, or an expression followed by a semicolon, which throws its value away. A function body is a block, and the value of a block is its last expression when that expression has no semicolon. So the last line of a function without a semicolon is the value that the function returns. If I put a semicolon there, the body produces `()`, "nothing", and the compiler reports that it expected the return type and found `()`.

</details>

## 03 · if, loops and ranges

<details><summary>Hint 1</summary>

Decide for each task which loop fits before you write it: do you know the sequence of numbers in advance (`for`), do you repeat while a condition holds (`while`), or do you stop in the middle (`loop`)? For C and D, trace the example from the task by hand first.

</details>
<details><summary>Hint 2</summary>

A: three branches, each of them just a text in braces; no `return` and no semicolons. B: a `let mut sum = 0;`, a `for` over `1..=n`, and `sum` as the last line. C: a parameter cannot be changed, so copy it first: `let mut n = n;`. Count the rounds in a second variable. D: start with `let mut multiple = step;` and add `step` in each round.

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

C: `while n > 0 { n /= 2; count += 1; }` and then `count`. D: inside `loop`, first `if multiple > limit { break; }`, then `multiple += step;`; after the loop, `multiple`. The comparison is `>`, because the result must be larger than the limit. P2: write down `i` and `total` for every round; the round with 7 ends the loop before anything is added.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

I use `for` when the values are known before the loop starts: a range of numbers or the elements of a list. It needs no counter of my own and cannot run for ever. I use `while` when I repeat as long as a condition is true and cannot say in advance how often, as with halving a number until it is 0. I use `loop` when the place to stop is inside the body, and leave it with `break`. The range `a..b` contains `a` and stops before `b`, so `0..5` has five numbers. The range `a..=b` also contains `b`, so `0..=5` has six.

</details>

## 04 · Text: &str, String and formatting

<details><summary>Hint 1</summary>

A, B and D are one `format!` each. Look up in CONCEPTS.md what can stand after the colon inside the braces: a number of decimals, a width, a direction.

</details>
<details><summary>Hint 2</summary>

A: a variable's name can stand directly inside the braces. B: `{price:.2}` prints two decimals. D: `{name:<8}` fills to 8 characters with the text on the left, `{score:>4}` fills to 4 with the number on the right. C: start with `String::new()` in a `let mut` variable; in a loop over `0..times`, add the word with `push_str`. The space is the difficult part: where must it go so that none is left at the end?

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

A: `format!("Hello, {name}!")`. B: `format!("{item}: {price:.2}")`. D: `format!("{name:<8}|{score:>4}")`. C: inside the loop, `if i > 0 { result.push(' '); }` and then `result.push_str(word);`: the space goes before every copy except the first. P1: a width fills with spaces up to that many characters, and `{:.1}` rounds to one decimal; it does not cut. P2: `{:?}` prints a text with its two quotation marks.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A function that only reads text takes `&str`. A `&str` is a view of text that is stored somewhere else, so the caller can pass a literal, or lend a `String` with `&`, and nothing is copied. A function that makes new text must return a `String`, because the new text needs memory of its own that someone owns; a view would have nothing to point at once the function has ended. `println!` prints its result and gives nothing back. `format!` uses the same patterns and returns the result as a `String`, which I can return, store, extend or compare in a test.

</details>

## 05 · Vectors: a list that can grow

<details><summary>Hint 1</summary>

All four tasks have the same shape: a `let mut` variable for the result (a number or an empty vector), a `for` loop that looks at one value in each round, and the result as the last line.

</details>
<details><summary>Hint 2</summary>

A: loop over `1..=n` and push `i * i`. B, C and D: `values` is already a reference to a list, so write `for value in values`. In that loop `value` is a `&i32`. Adding works directly (`sum += value`). For a comparison, or to push the number itself, write `*value`.

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

C: `if *value > limit { count += 1; }`. D: `if value % 2 == 0 { even.push(*value); }`. P1: follow the vector step by step: `[1, 2, 3]`, then with 4, then without it, then with 5. P3: the inner loop runs 0 times, then 1, 2 and 3 times.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`for x in &values` walks through the vector without taking it away. In each round, `x` is a reference to one element, a `&i32` for a `Vec<i32>`: it points at the element inside the vector, and `*x` is the number itself. After the loop the vector is unchanged and still mine. A function that only reads a list takes `&[i32]`, a slice. A slice is a view of a row of elements that are stored somewhere else. The caller lends its vector with `&` and keeps it, nothing is copied, and the same function also accepts an array or a part of a list.

</details>

## 06 · Structs and methods

<details><summary>Hint 1</summary>

Inside a method, the novel is called `self`, and its fields are `self.title`, `self.pages` and `self.pages_read`. In `new` there is no `self`: you build a `Novel` value and return it.

</details>
<details><summary>Hint 2</summary>

A: the field `title` is a `String` and the parameter is a `&str`, so convert with `title.to_string()`. C: after adding, `pages_read` may be larger than `pages`; in that case set it to `pages`. Think about what happens when the sum does not fit into a `u32`. D: `format!` with `self.title`, `self.pages_read` and `self.pages`.

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

A: `Novel { title: title.to_string(), pages, pages_read: 0 }`. B: `self.pages > 300`. C: the remaining pages are `self.pages - self.pages_read`, which cannot go below zero; if `pages` is larger than that, set `self.pages_read = self.pages`, otherwise add. Shorter: `self.pages_read = self.pages_read.saturating_add(pages).min(self.pages);`. D: `format!("{}: {} of {} pages", self.title, self.pages_read, self.pages)`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A method's first parameter says what the method may do with the value it is called on. `&self` lends the value for reading: `is_long` and `progress` look at fields and cannot change them, and they can be called on any `Novel`. `&mut self` lends it for changing: `read` assigns to `self.pages_read`, and the caller's variable must be declared `mut`. `Novel::new` has no `self` because no novel exists yet when it is called; its job is to build one. It is called on the type, `Novel::new("Dune", 412)`, not on a value.

</details>

## 07 · Enums and match

<details><summary>Hint 1</summary>

Each task is one `match` and nothing else. Write one arm for every variant, with the enum's name in front: `Light::Red => ...`.

</details>
<details><summary>Hint 2</summary>

A: the arms produce `Light` values. C: a pattern can give names to the numbers inside a variant, `Shape::Rectangle(width, height) => ...`. D: the arms are a single value, two ranges written with `..=`, and `_` at the end.

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

A: `Light::Red => Light::Green,` and two more arms. C: `Shape::Square(side) => side * side,`, then `Shape::Rectangle(width, height) => width * height,`, then `Shape::Dot => 0,`. D: `0 => "none", 1..=9 => "few", 10..=99 => "many", _ => "crowd",`. P1: arms are tried from the top, and the first one that fits is taken.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Exhaustive means that every value the type can have fits at least one arm. The compiler checks this and rejects a `match` that leaves a case out, with a message that names the missing pattern. When I list every variant of my own enum, this check protects me later: if someone adds a variant, every `match` over the enum stops compiling until it says what to do with the new case. With a `_` arm the program still compiles, and the new variant silently gets whatever `_` does, which is probably wrong. `_` is the right tool for numbers and texts, where listing all values is impossible.

</details>

## 08 · Option and Result at first contact

<details><summary>Hint 1</summary>

Look at each return type first. `Option<i32>` means you return `Some(number)` or `None`. `Result<i32, String>` means you return `Ok(number)` or `Err(text)`. A text for an error must be a `String`: `String::from("...")` or `format!(...)`.

</details>
<details><summary>Hint 2</summary>

A: inside the loop, `return Some(*value);` leaves the function at the first even value; after the loop, the last line is `None`. B: a `match` with the arms `Some(years)` and `None`; both arms must produce a `String`. D: `text.parse::<i32>()` is a `Result`; `unwrap_or(0)` gives the number or 0. E: write a small helper that turns one text into `Result<i32, String>` with the right message, and call it twice.

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

C: `if b == 0 { Err(String::from("cannot divide by zero")) } else { Ok(a / b) }`. E: the helper is `match text.parse::<i32>() { Ok(number) => Ok(number), Err(_) => Err(format!("not a number: {text}")) }`. In `sum_of_two`, write `let first = helper(a)?;` and `let second = helper(b)?;`, then `Ok(first + second)`. The `?` returns the error at once, so a wrong `a` is reported before `b` is looked at.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A function returns `Option` when "no value" is a normal answer that needs no explanation: the first even number of a list that has none. It returns `Result` when something went wrong and the caller should learn the reason: a division by zero, a text that is not a number. Rust has no null because a null can hide in every variable, and nothing forces the programmer to check for it. With `Option`, the possibility of "nothing" is part of the type. An `Option<i32>` cannot be used as an `i32`; the compiler makes me handle both cases first. A plain `i32` is always a number.

</details>

## 09 · Writing your own tests

<details><summary>Hint 1</summary>

Read the contract of `is_leap_year` again. It has three rules: divisible by 4, the exception for 100, and the exception to the exception for 400. Each rule needs a year that shows it, and you also need a year that fits none of them.

</details>
<details><summary>Hint 2</summary>

Tasks A and B: copy the shape of a test from CONCEPTS.md into `mod my_tests`: `#[test]`, then `fn a_name_that_says_what_is_checked() { assert!(...); }`. For B, put `#[should_panic]` on the line between `#[test]` and `fn`. Task C: inside `check_leap_year`, write `assert!(candidate(2024));` and more lines like it. When one of the "catch" tests fails, its message tells you what kind of year is still missing.

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

Four years are enough for C: `assert!(candidate(2024));`, `assert!(!candidate(2023));`, `assert!(!candidate(1900));`, `assert!(candidate(2000));`. The same four years, spread over three or four `#[test]` functions that call `is_leap_year`, solve A. B: a test whose body is `percent(1, 0);`. P1: `0.1 + 0.2` is not stored as exactly `0.3`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

I went through the contract sentence by sentence and chose one year for each rule: 2024 for "divisible by 4", 1900 for "but not centuries", 2000 for "except every fourth century", and 2023 as an ordinary year that is no leap year. Four ordinary years such as 2021 to 2024 would only exercise the first rule. A function that divides by 4 and knows nothing about centuries would pass all of them, and so would a function that forgets the 400 rule. A test is useful when some wrong version of the function fails it. So for each test I ask which mistake it would catch, and for each likely mistake I ask which test catches it.

</details>

## 10 · Capstone: a text statistics tool

<details><summary>Hint 1</summary>

Do not start with `run`. Start with the types: an `enum` with four variants for the commands, and a `struct` with three `usize` fields for the counts. Then write one small function for each job and test it in `mod my_tests`. `run` comes last and only connects the pieces.

</details>
<details><summary>Hint 2</summary>

A possible set of pieces: `fn parse_command(word: &str) -> Result<Command, String>` (a `match` on the word, with the error message in the `_` arm; a name instead of `_` lets you use the word in the message); `fn count(text: &str) -> Counts` (uses `lines().count()`, `split_whitespace().count()` and `chars().count()`); `fn longest_word(text: &str) -> Option<String>` (a loop over the words that remembers the best one so far); `fn average_word_length(text: &str) -> Option<f64>` (`None` when there are no words).

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

`run` begins with `let command = parse_command(command)?;` and continues with a `match command { ... }` that has four arms. The `Longest` arm is itself a `match` on the `Option`: `Some(word) => Ok(word)`, `None => Err(String::from("no words"))`. In `longest_word`, keep two variables, the best word so far and its length in characters, and replace them only when a word is strictly longer (`>`), so that the first of several equal words stays. The average is `characters as f64 / words as f64`, printed with `{average:.1}`. The count report is `format!("lines: {}\nwords: {}\ncharacters: {}", ...)`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A model answer: "My design has two types. `Command` is an enum with one variant for each command, so `run` can `match` on it and the compiler checks that no command is forgotten. `Counts` is a struct with the three numbers of the count report. `parse_command` is the only place that knows the command words; it returns a `Result`, because an unknown word is an error with a message. `count`, `longest_word` and `average_word_length` each take the text and return one answer; the last two return an `Option`, because a text without words has no longest word and no average. `run` connects them and does all the formatting. For ten commands I would move each command's code out of `run` into a function of its own, so that `run` stays a list of one-line arms, and I would give `Command` a method that returns its word, so that a help text can be built from the enum."

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are four bugs, two in `average` and two in `count_passing`. `result_line` gives right answers; it is only written in a long-winded way. For the bugs, read the contract of each function slowly and compare it with the body: which words of the contract are "at least", "empty", and "1.5"?

</details>
<details><summary>Hint 2</summary>

`average` divides two whole numbers, so the fraction is gone before the result becomes an `f64`; and for an empty list it divides by zero. `count_passing` starts its loop at index 1, and its comparison does not include the pass mark itself. Then run `cargo clippy --test chapter02`: it reports twelve places in this file, and each message proposes the simpler form.

</details>
<details><summary>Hint 3 (nearly the answer)</summary>

`average`: return `0.0` first when `scores.is_empty()`; add the scores up in an `f64` (`total += f64::from(*score)`), and divide by `scores.len() as f64`. `count_passing`: `for score in scores { if *score >= pass_mark { count += 1; } }`. `result_line`: `let verdict = if passed { "pass" } else { "fail" };` and `format!("{name}: {verdict}")`. The parameters become `&[u32]` and `&str`. The last line of each function loses `return` and its semicolon.

</details>
<details><summary>Check your review</summary>

**Bugs.** `average` computes `total / scores.len() as u32` with whole numbers, so the average of 1 and 2 is 1 and only then becomes `1.0`. For an empty list the same line divides by zero and panics, although the contract asks for `0.0`. `count_passing` loops over `1..scores.len()` and never looks at the first score. Its comparison is `>`, and the contract says "at least", which is `>=`.

**Style, all reported by clippy.** The parameters `&Vec<u32>` and `&String` should be `&[u32]` and `&str`: a function that only reads should accept any list and any text. A loop over indexes that only uses the index to read `scores[i]` should walk the values (`for score in scores`); that form cannot start at the wrong index, so it would have prevented one of the bugs. `total = total + x` is written `total += x`. `== true` adds nothing to a `bool`. `return` on the last line is written as a plain expression without a semicolon.

**Style that clippy does not report here.** `name.clone()` in `result_line` makes a copy of the text only to read it; `push_str(name)` is enough. The whole function is one `format!`.

One more risk is worth a line in a real review: a sum of many large scores can overflow a `u32`. Adding in `f64` avoids that.

</details>

## Drills

Run `cargo drill ch02` after lesson 09. The compiler output tells you *what* happened; these notes say *why*. Read a note only after you have seen the verdict for that drill. Each title names the lesson that the drill belongs to.

<details><summary>d01 · Growing a vector (lessons 01 and 05)</summary>

`E0596`. `push` changes the vector, and `numbers` was declared without `mut`. The message says "cannot borrow as mutable": `push` needs the vector in a form that allows change, and an immutable variable cannot give that. Write `let mut numbers`.

</details>
<details><summary>d02 · An if that produces a value (lesson 03)</summary>

`E0317`. The value of the `if` is bound to `label`, so the `if` must have a value on both paths. Without an `else` it would produce `()` when the condition is false. Add an `else` branch with another text.

</details>
<details><summary>d03 · Two kinds of number (lesson 01)</summary>

`E0277`, "cannot multiply `{float}` by `{integer}`". `price` is a float and `count` is a whole number, and Rust does not convert between them by itself. Make both floats: write `4.0`, or convert with `f64::from(count)`. The code `E0277` means in general that a type cannot do the operation that was asked of it; you will see it in other situations too.

</details>
<details><summary>d04 · A block with its own x (lesson 02)</summary>

Compiles and runs. The block is an expression, and its last line `x + 1` is its value. The `let x` inside the block is a new variable that shadows the outer `x` only until the closing brace, so the outer `x` is still 5 and `y` is 11.

</details>
<details><summary>d05 · A literal for a String parameter (lesson 04)</summary>

`E0308`, "expected `String`, found `&str`". A literal is a `&str`. The compiler proposes `"hello".to_string()`, which works. The better repair is in the function: `shout` only reads its text, so its parameter should be `&str`, and then the literal can be passed as it is.

</details>
<details><summary>d06 · Walking a vector by index (lessons 03 and 05)</summary>

Panics at run time: "index out of bounds: the len is 3 but the index is 3". The range `0..=scores.len()` includes 3, and the last index is 2. The compiler cannot see this, because the length is only known when the program runs. The smallest repair is `0..scores.len()`. The better one is `for score in &scores`, which needs no index at all.

</details>
<details><summary>d07 · One coin too few (lesson 07)</summary>

`E0004`, "non-exhaustive patterns: `Coin::Dime` not covered". A `match` must have an arm for every variant. Add `Coin::Dime => 10`. Do not add `_`: the next person who adds a coin should get this same error.

</details>
<details><summary>d08 · Text to number (lesson 08)</summary>

`E0308`, "expected `u32`, found `Result<_, _>`". Not every text is a number, so `parse` returns a `Result`, and a `Result` is not a `u32`. Say what should happen in the error case: `.unwrap_or(0)`, or a `match`. The compiler proposes `.expect("REASON")`, which panics when the text is not a number; that is acceptable only when the text is known to be right.

</details>
<details><summary>d09 · Are two points equal? (lesson 06)</summary>

`E0369`, "binary operation `==` cannot be applied to type `Point`". A new struct has no `==` until you say what it means. Add `#[derive(PartialEq)]` above the struct, and the compiler writes a comparison of all fields.

</details>
<details><summary>d10 · A variable and its block (lessons 01 and 03)</summary>

`E0425`, "cannot find value `label` in this scope". `label` is declared inside the braces of the `if` and stops existing at the closing brace. The help says that the name "is available in a different scope". Use the `if` as an expression: `let label = if guests > 10 { "many" } else { "few" };`.

</details>

## Mastery check: model answers

Answer the questions in the [chapter guide](README.md#mastery-check) from memory first. Then compare.

<details><summary>Model answers (after writing your own)</summary>

**1. `let`, `let mut` and shadowing.**
`let limit = 10;` gives a name to a value that never changes. `let mut total = 0; total += 5;` makes one variable whose value is replaced; use it for something that changes over time, such as a counter. `let width = 12; let width = f64::from(width) / 2.54;` makes a second variable that takes over the name; use it when a value is transformed in steps and the old form is not needed, or when the type changes.

**2. What a function returns.**
A function returns the value of the last expression of its body, the one without a semicolon. A semicolon turns that expression into a statement and throws the value away, so the body then produces `()`. `()` is the unit type, "no value". "expected `i32`, found `()`" in a message means that a value was needed and the code produces nothing, usually because of a semicolon on the last line or a missing `else`.

**3. Three loops.**

```rust
fn main() {
    let n = 4;
    let mut sum = 0;
    for i in 1..=n {
        sum += i;
    }
    assert_eq!(sum, 10);

    let mut rest = 100;
    while rest > 10 {
        rest /= 2;
    }
    assert_eq!(rest, 6);

    let mut tries = 0;
    loop {
        tries += 1;
        if tries * tries > 50 {
            break;
        }
    }
    assert_eq!(tries, 8);
}
```

`0..5` makes five rounds (0, 1, 2, 3, 4). `0..=5` makes six.

**4. `&str` and `String`.**
A `String` owns its text and can grow. A `&str` is a view of text that is stored somewhere else: in the program itself for a literal, or in a `String`. A function that only reads text takes `&str`, so that callers can pass a literal or lend a `String` with `&`. A function that makes new text returns `String`.

**5. A struct with methods, and an enum with data.**

```rust
struct Lamp {
    on: bool,
    level: u32,
}

impl Lamp {
    fn is_bright(&self) -> bool {
        self.on && self.level > 50
    }

    fn switch_off(&mut self) {
        self.on = false;
    }
}

enum Command {
    Off,
    Dim(u32),
}

fn apply(lamp: &mut Lamp, command: Command) {
    match command {
        Command::Off => lamp.switch_off(),
        Command::Dim(level) => lamp.level = level,
    }
}

fn main() {
    let mut lamp = Lamp { on: true, level: 80 };
    assert!(lamp.is_bright());
    apply(&mut lamp, Command::Dim(20));
    assert!(!lamp.is_bright());
    apply(&mut lamp, Command::Off);
    assert!(!lamp.on);
}
```

**6. `Option` and `Result`.**
`Option<T>` is `Some(value)` or `None`: a value that may be missing. `Result<T, E>` is `Ok(value)` or `Err(error)`: a value, or the reason why there is none. Neither can be used as the value itself; both cases must be handled first. Ways to get the value out: a `match` with one arm for each case; `if let Some(x) = option { ... }` when only one case matters; `unwrap_or(spare)` for a spare value; `?` to pass an `Err` on to the caller; `unwrap()` or `expect("...")`, which panic when there is no value.

**7. A test module.**

```rust
fn half(n: u32) -> u32 {
    assert!(n % 2 == 0, "half: `n` must be even");
    n / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_of_an_even_number() {
        assert_eq!(half(10), 5);
        assert_eq!(half(0), 0);
    }

    #[test]
    #[should_panic]
    fn an_odd_number_is_refused() {
        half(7);
    }
}
```

Kinds of case to look for: an ordinary case; each boundary, on both sides; nothing and zero; one case for each rule and each exception in the contract; the inputs that must be refused.

**8. The capstone again, with a fifth command.**
There is no model text for this one. Use `cargo progress redo ch02 l10` to get the untouched file back (your solution is saved first). When you add a command, notice what the compiler does after you add the variant to your enum and before you add the arm in `run`.

</details>
