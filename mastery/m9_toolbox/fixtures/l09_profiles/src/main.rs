//! Prints what the build profile decided, then panics while a guard is alive.

struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        println!("guard dropped");
    }
}

fn main() {
    println!("debug assertions: {}", cfg!(debug_assertions));
    println!(
        "panic strategy: {}",
        if cfg!(panic = "abort") {
            "abort"
        } else {
            "unwind"
        }
    );
    let small: u8 = std::hint::black_box(200);
    let sum = std::panic::catch_unwind(|| small + std::hint::black_box(100));
    println!(
        "200 + 100 as u8: {}",
        match sum {
            Ok(value) => value.to_string(),
            Err(_) => "panic".to_owned(),
        }
    );
    let _guard = Guard;
    panic!("fatal error");
}
