# Coming to Rust from Java and C#

[Course overview](../README.md) · [All guides](README.md)

Java and C# taught you to model a problem with classes, interfaces and generics, and to let a garbage collector and a runtime worry about memory. Rust keeps strong static types and generics, and it adds a question that your compiler never asked: who owns this object, and who may change it right now? This page tells you what you can keep, what you must unlearn, and where to spend your time in this course.

## What transfers

- **Static types and generics.** `List<T>` becomes `Vec<T>`, and bounded generics such as `<T extends Comparable<T>>` or `where T : IComparable<T>` become `<T: Ord>`. Unlike Java, generics are not erased: each concrete type gets its own compiled code.
- **Interfaces.** A Rust trait is close to an interface with default methods. A trait that you define can also be implemented for types you did not write, including `i32` and `String`.
- **Streams and LINQ.** `stream().filter().map().collect()` and `Where().Select().ToList()` look almost the same in Rust, and they are just as lazy.
- **Deterministic clean-up.** `try`-with-resources and `using` become the default: a value is cleaned up when its owner goes out of scope, with no extra syntax.
- **Packages and build tools.** Packages or namespaces become modules; Maven, Gradle or NuGet become Cargo.

## What does not transfer

- **No inheritance of implementation.** There are no base classes and no `protected`. Share behaviour with traits, share data with composition.
- **No `null`.** In safe code, a reference always points to a live value; absence is `Option<T>`.
- **No exceptions.** Recoverable failures are `Result<T, E>` return values. A panic is for bugs, and it is not meant to be caught.
- **Objects are not all references.** A struct value lives where you put it: in a local variable, inside another struct, in a `Vec`. Assignment moves it.
- **Mutation is controlled.** Something may be changed only through its owner or through one exclusive `&mut` reference at a time. Types built for shared mutation, such as `RefCell` and `Mutex`, check that rule at run time or with a lock instead.

## Five mistakes Java and C# programmers make

### 1. Rebuilding a class hierarchy

An abstract base class with shared fields has no direct translation. Split it: the shared data goes into a struct that others contain, and the shared behaviour goes into a trait with default methods.

```rust
trait Shape {
    fn area(&self) -> f64;
    fn describe(&self) -> String {
        // a default method, like a non-abstract method of an interface
        format!("shape with area {:.1}", self.area())
    }
}

struct Square(f64);
struct Circle(f64);

impl Shape for Square {
    fn area(&self) -> f64 { self.0 * self.0 }
}
impl Shape for Circle {
    fn area(&self) -> f64 { std::f64::consts::PI * self.0 * self.0 }
}

fn main() {
    let shapes: Vec<Box<dyn Shape>> = vec![Box::new(Square(2.0)), Box::new(Circle(1.0))];
    let total: f64 = shapes.iter().map(|s| s.area()).sum();
    assert!((total - 7.14).abs() < 0.01);
    assert_eq!(shapes[0].describe(), "shape with area 4.0");
}
```

When the set of variants is closed (every shape is known), an `enum` with a `match` is often simpler than a trait object.

### 2. Holding a reference from a getter while changing the object

In Java, a getter can return the inner list and the caller can keep it while calling other methods. In Rust, a returned reference borrows the whole object:

```rust,compile_fail,E0502
struct Team {
    members: Vec<String>,
}

impl Team {
    fn captain(&self) -> &String { &self.members[0] }
    fn add(&mut self, name: &str) { self.members.push(name.to_string()); }
}

fn main() {
    let mut team = Team { members: vec!["Ana".to_string()] };
    let captain = team.captain();
    team.add("Ben"); // E0502: `team` is still borrowed by `captain`
    println!("{captain}");
}
```

Finish using the reference first, clone the small value you need, or restructure so that the read and the write do not overlap.

### 3. Building object graphs with back references

A tree where children point to their parent is easy with a garbage collector. In Rust, give the parent shared ownership of its children (`Rc`) and give the children a non-owning link back (`Weak`), or store all nodes in a `Vec` and link them by index. Chapter 9 has a lesson on exactly this.

### 4. Using exceptions for control flow

A parse that can fail returns `Result`. The caller decides: pass it up with `?`, handle it with `match`, or turn it into a default.

```rust
fn parse_age(text: &str) -> Result<u8, String> {
    let age: u8 = text.parse().map_err(|e| format!("{text:?}: {e}"))?;
    if age > 150 {
        return Err(format!("{age} is not a plausible age"));
    }
    Ok(age)
}

fn main() {
    assert_eq!(parse_age("42"), Ok(42));
    assert!(parse_age("x").is_err());
    assert_eq!(parse_age("x").unwrap_or(0), 0);
}
```

`unwrap()` turns an error into a panic. It belongs in tests and in places where an error would be a bug, not in place of `catch`.

### 5. Expecting `equals` and `hashCode` for free

Structs have no equality until you ask for it:

```rust,compile_fail,E0369
struct Point { x: i32, y: i32 }

fn main() {
    let same = Point { x: 1, y: 2 } == Point { x: 1, y: 2 }; // E0369: binary operation `==` cannot be applied
}
```

Derive the traits, and the compiler writes field-by-field implementations that agree with each other, which is the contract that `equals` and `hashCode` (or `Equals` and `GetHashCode`) must keep by hand:

```rust
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Point { x: i32, y: i32 }

fn main() {
    let mut seen = HashSet::new();
    seen.insert(Point { x: 1, y: 2 });
    assert!(seen.contains(&Point { x: 1, y: 2 }));
}
```

## Where to spend your time

| Priority | Unit | Why |
| --- | --- | --- |
| High | [Chapter 4](../exercises/chapter04/README.md) and [Chapter 5](../exercises/chapter05/README.md) | Ownership and borrowing replace the garbage collector and shared references. |
| High | [Chapter 9, Structs and methods](../exercises/chapter09/README.md) | Designing types without inheritance; `Rc`, `Weak` and `RefCell` for shared, changing data. |
| High | [Chapter 10, Enums and pattern matching](../exercises/chapter10/README.md) | Enums with data replace many small class hierarchies. |
| High | [Chapter 7, Handling errors](../exercises/chapter07/README.md) | `Result` instead of checked and unchecked exceptions. |
| Medium | [Chapters 11](../exercises/chapter11/README.md), [12](../exercises/chapter12/README.md) and [13](../exercises/chapter13/README.md) | Traits, `PartialEq`/`Hash`/`Ord` contracts, conversions. Familiar ideas, different rules. |
| Then | [M1, the type system in depth](../mastery/m1_types/README.md) and [M8, library design](../mastery/m8_api/README.md) | Where your API-design experience meets Rust's rules. |

## Idiom map

| Java / C# | Rust |
| --- | --- |
| `class` with fields and methods | `struct` plus `impl` |
| `interface` with default methods | `trait` with default methods |
| `abstract class` | trait plus a struct for the shared data |
| sealed hierarchy, `switch` on type | `enum` and `match` |
| `null`, `Optional<T>`, `T?` | `Option<T>` |
| checked or unchecked exception | `Result<T, E>`; `panic!` for bugs |
| `List<T>`, `ArrayList<T>` | `Vec<T>` |
| `Map<K, V>`, `Dictionary<K, V>` | `HashMap<K, V>`, `BTreeMap<K, V>` |
| `equals` / `hashCode` | `#[derive(PartialEq, Eq, Hash)]` |
| `toString()` / `ToString()` | `impl Display` (for users) and `#[derive(Debug)]` (for programmers) |
| `Comparable`, `IComparable` | `PartialOrd`, `Ord` |
| streams, LINQ | iterator adapters |
| `try`-with-resources, `using` | `Drop`, automatically at the end of scope |
| `synchronized`, `lock` | `Mutex<T>`, which owns the data it protects |
| `final` / `readonly` | the default: bindings are immutable unless declared `mut` |
| JUnit, xUnit | `#[test]` and `cargo test` |

## Read more

- [Trait objects in the Rust Reference](https://doc.rust-lang.org/reference/types/trait-object.html)
- [std::hash::Hash](https://doc.rust-lang.org/std/hash/trait.Hash.html), including the rule that equal values must hash equally
- [std::rc::Weak](https://doc.rust-lang.org/std/rc/struct.Weak.html)
