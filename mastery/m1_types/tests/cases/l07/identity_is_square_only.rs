//@ expect: E0599
//@ why: `identity` must exist for square matrices only.
use m1_types::l07_sealed_typestate::Matrix;

fn main() {
    let wide = Matrix::<2, 3>::identity();
    println!("{:?}", wide.rows());
}
