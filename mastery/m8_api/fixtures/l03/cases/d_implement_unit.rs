//@ expect: E0277
//@ why: (D) if users can implement `Unit`, every new required item in the trait breaks them; a supertrait that they cannot name keeps all implementations inside the library

struct Kelvin;

impl gauge::Unit for Kelvin {
    const SYMBOL: &'static str = "K";

    fn from_celsius(celsius: f64) -> f64 {
        celsius + 273.15
    }
}

fn main() {
    println!("{}", <Kelvin as gauge::Unit>::SYMBOL);
}
