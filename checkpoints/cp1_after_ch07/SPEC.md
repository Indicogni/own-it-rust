# Checkpoint 1 specification: `splitpot`

[All checkpoints](../README.md) · [Rubric](RUBRIC.md)

`splitpot` is a small library for a group of people who share expenses, for
example on a trip. Somebody writes one line per expense into a text file.
The library reads the file, works out who is owed money and who owes money,
and plans the payments that settle everything.

```text
# Weekend trip
ana paid 30.00 for ana, ben, cy
ben paid 12.50 for ana, ben
cy paid 7 for ben
```

After these three lines ana is owed 13.75, ben owes 10.75 and cy owes 3.00.
Two payments settle the trip: ben pays ana 10.75, and cy pays ana 3.00.

This document is the contract. Where it says "must", a test checks it. The
public names and signatures are already in `src/lib.rs`, so that the tests
compile. Everything behind them is yours to design.

## 1. Rules that apply everywhere

1. **Money is a whole number of cents.** Amounts are `u64` cents. Balances
   are `i64` cents. No floating-point type appears anywhere.
2. **Names** are compared exactly, byte by byte. `ana` and `Ana` are two
   people. A name is one or more characters, none of which is white space
   or a comma. Any other character is allowed, so `Zoë`, `李`, `o'neil`,
   `x#1`, `paid` and `for` are all names.
3. **White space** means every character for which `char::is_whitespace`
   returns true.
4. **No function may panic** for any input, and no function may print.
5. **Errors** are values of the types in `src/lib.rs`. Each of them must
   implement `std::error::Error`. The messages and the sources are fixed in
   section 7.
6. Only the standard library may be used. `unsafe` is not allowed.

## 2. `parse_amount`

```text
pub fn parse_amount(text: &str) -> Result<u64, AmountError>
```

Reads a decimal amount and returns it in cents. The grammar is

```text
amount = DIGITS [ "." DIGIT [ DIGIT ] ]
DIGITS = one or more DIGIT
DIGIT  = one of the ASCII characters 0 1 2 3 4 5 6 7 8 9
```

and the whole text must match it. There is no sign, no white space, no
exponent and no separator.

| text | result |
|---|---|
| `7` | `Ok(700)` |
| `12.5` | `Ok(1250)` (one decimal digit means tenths) |
| `12.50` | `Ok(1250)` |
| `0.07` | `Ok(7)` |
| `007` | `Ok(700)` (leading zeros are allowed) |
| `184467440737095516.15` | `Ok(u64::MAX)` |
| the empty text | `Err(AmountError::Empty)` |
| `184467440737095516.16` | `Err(AmountError::TooLarge)` |
| `+5`, `-5`, ` 5`, `5.`, `.5`, `5.123`, `1e3`, `٣` | `Err(AmountError::Malformed)` |

- A text that does not match the grammar is `Malformed`, however long it is.
- A text that matches the grammar but whose value in cents is greater than
  `u64::MAX` is `TooLarge`, however many digits it has.
- `parse_amount` must not allocate.

## 3. `parse_entry`

```text
pub struct Entry<'a> { pub payer: &'a str, pub cents: u64, pub sharers: Vec<&'a str> }
pub fn parse_entry(line: &str) -> Result<Entry<'_>, EntryError>
```

An entry line says who paid, how much, and who shares the cost:

```text
<payer> paid <amount> for <name>, <name>, ...
```

Read the line in this order. The first problem, from the left, decides the
error.

1. White space at the start and at the end of the line is ignored.
2. The first word is the payer. A *word* is a run of characters that are
   not white space; words are separated by one or more white-space
   characters. If there is no first word, or if it is not a name (it
   contains a comma): `EntryError::Syntax`.
3. The second word must be exactly `paid`. Otherwise: `Syntax`.
4. The third word is the amount. If there is no third word: `Syntax`. If
   `parse_amount` rejects it: `EntryError::Amount` with the reason inside.
5. The fourth word must be exactly `for`. Otherwise: `Syntax`.
6. The rest of the line is the list of sharers: one or more items separated
   by commas. White space around an item is ignored. Check the items from
   left to right. An item that is not a name (it is empty, or it has white
   space inside): `Syntax`. An item that is equal to an earlier item:
   `EntryError::DuplicateSharer`.

So a list is never empty, `a,,b` and a comma at the end are `Syntax`, and
the payer may or may not be one of the sharers.

The names in the returned `Entry` must be slices of `line`. They are not
copies. For a line with three sharers `parse_entry` must allocate at most
once (the `Vec` of sharers). If steps 1 to 5 find the error, it must not
have allocated at all.

## 4. `Ledger`

```text
impl Ledger {
    pub fn new() -> Ledger
    pub fn add(&mut self, entry: &Entry<'_>) -> Result<(), Overflow>
    pub fn balance(&self, name: &str) -> Option<i64>
    pub fn names(&self) -> Vec<&str>
    pub fn into_balances(self) -> Vec<(String, i64)>
}
```

A ledger holds one balance for every person it has seen. A positive balance
means that the group owes this person money. A negative balance means that
this person owes money. A new ledger has no people.

### 4.1 `add`

`add` applies one entry.

1. If `entry.sharers` is empty, nothing changes and the result is `Ok(())`.
   (`parse_entry` never returns such an entry, but the fields of `Entry` are
   public.) You may assume that the sharers are distinct.
2. Let `n` be the number of sharers. Every sharer owes `cents / n` (integer
   division). The division leaves `cents % n` cents over; the first
   `cents % n` sharers **in the order of the list** owe one cent more.
3. For every person in the entry, the *change* is: `cents` if the person is
   the payer, minus what the person owes as a sharer. A payer who is also a
   sharer has one change that combines both.
4. The new balance of each person is the old balance (zero for a new
   person) plus the change. People who were not in the ledger are added,
   also when their balance is zero.
5. If any new balance is outside the range of `i64`, `add` returns
   `Err(Overflow)` and **the ledger is exactly as it was before the call**:
   no balance has changed and no person has been added. Only the final
   balances are checked: `ana paid 184467440737095516.15 for ana` succeeds
   and leaves ana's balance unchanged, although the amount alone does not
   fit in an `i64`.

A consequence of rules 1 to 4: the balances of a ledger always add up to
zero.

`add` must not make a copy of the ledger to restore it after a failure.

### 4.2 Reading the ledger

- `balance(name)` returns the balance of a person, or `None` if the ledger
  has never seen the name. It must not allocate.
- `names()` returns every name, sorted in the order of `str` (`"Zed"` comes
  before `"abe"`).
- `into_balances()` consumes the ledger and returns one `(name, balance)`
  pair per person, sorted by name. The `String`s it returns must be the
  ones the ledger owned: they are moved, not copied.

## 5. `read_ledger` and `report`

```text
pub struct ReadError { pub line: usize, pub cause: Cause }
pub enum Cause { Entry(EntryError), Overflow }
pub fn read_ledger(text: &str) -> Result<Ledger, ReadError>
pub fn report(error: &dyn Error) -> String
```

`read_ledger` reads a whole file, given as one `&str`.

- The file is split into lines as `str::lines` does it, so both `\n` and
  `\r\n` end a line. Lines are numbered from 1. Every line counts.
- A line that is empty, or has only white space, is skipped.
- A line whose first character that is not white space is `#` is a comment
  and is skipped. A `#` anywhere else has no special meaning.
- Every other line is parsed with the rules of section 3 and added to the
  ledger with the rules of section 4.
- The first line that fails ends the reading. The result is a `ReadError`
  with the number of that line and the cause: `Cause::Entry` with the
  `EntryError`, or `Cause::Overflow`.

`report` turns any error into one line of text for a person: the message of
the error, then the message of its source, then of the source's source, and
so on, joined by a colon and a space (`": "`).

```text
line 2: invalid entry: invalid amount: amount is too large
```

## 6. `settle`

```text
pub struct Transfer<'a> { pub from: &'a str, pub to: &'a str, pub cents: u64 }
pub fn settle(balances: &[(String, i64)]) -> Result<Vec<Transfer<'_>>, Unbalanced>
```

`settle` plans payments that bring every balance to zero. A transfer of
`cents` from `from` to `to` raises the balance of `from` by `cents` and
lowers the balance of `to` by `cents`.

1. If the balances do not add up to exactly zero, the result is
   `Err(Unbalanced)`. "Exactly" means the mathematical sum: two balances of
   `i64::MIN` do not add up to zero.
2. Otherwise repeat the following step until every balance is zero.
   - The *debtor* is the person with the lowest balance. The *creditor* is
     the person with the highest balance. If several people have that
     balance, take the one that comes first in the slice.
   - The debtor pays the creditor as much as possible without making the
     debtor's balance positive or the creditor's balance negative: the
     smaller of the debt and the credit.
3. The result lists the transfers in the order in which they were made.

The slice may be in any order, and every balance from `i64::MIN` to
`i64::MAX` may occur. A slice without open balances gives an empty plan.
The names in the transfers must be borrowed from `balances`.

Example: for balances `a: 700, b: -400, c: 300, d: -600` the plan is
`d → a 600`, then `b → c 300`, then `b → a 100`.

## 7. Error messages and sources

`Display` must print exactly these texts.

| value | text |
|---|---|
| `AmountError::Empty` | `amount is empty` |
| `AmountError::Malformed` | `amount is not a number with at most two decimal places` |
| `AmountError::TooLarge` | `amount is too large` |
| `EntryError::Syntax` | `expected: <payer> paid <amount> for <name>, <name>, ...` |
| `EntryError::Amount(_)` | `invalid amount` |
| `EntryError::DuplicateSharer` | ``a name appears twice after `for` `` (the word `for` is between two backquotes) |
| `Overflow` | `balance out of range` |
| `Unbalanced` | `balances do not add up to zero` |
| `ReadError` with `Cause::Entry(_)` | `line N: invalid entry` (N is the line number) |
| `ReadError` with `Cause::Overflow` | `line N: balance out of range` |

`Error::source` must return:

| value | source |
|---|---|
| `EntryError::Amount(e)` | the `AmountError` `e` |
| `ReadError` with `Cause::Entry(e)` | the `EntryError` `e` |
| every other error value | `None` |

`EntryError` must implement `From<AmountError>`. Every error type must be
usable with `?` in a function that returns
`Result<_, Box<dyn Error + Send + Sync + 'static>>`.

## 8. Cost

This section collects the rules about memory from sections 2 to 6 in one
place. It adds no new rule. The technique tests check each of them.

| operation | rule | from |
|---|---|---|
| `parse_amount` | must not allocate, for any text | section 2 |
| `parse_entry` | the names in the `Entry` are slices of `line`, not copies | section 3 |
| `parse_entry` | for a line with three sharers, at most one allocation (the `Vec` of sharers) | section 3 |
| `parse_entry` | no allocation at all when steps 1 to 5 find the error | section 3 |
| `Ledger::add` | must not copy the ledger to restore it after a failure | section 4.1 |
| `Ledger::balance` | must not allocate | section 4.2 |
| `Ledger::into_balances` | returns the `String`s that the ledger owned: moved, not copied | section 4.2 |
| `settle` | the names in the transfers are borrowed from `balances` | section 6 |

## 9. What is not specified

Anything this document does not mention is your decision: the fields of
`Ledger`, private functions and types, the order in which you write the
code, and your own tests. You may add derives and trait implementations to
the public types. You may not remove or change a public name, a public
field, or a signature.
