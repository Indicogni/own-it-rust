//@ expect: E0308
//@ why: multiplying (2 x 3) by (2 x 3) must be a type error: the inner dimensions differ.
use m1_types::l07_sealed_typestate::Matrix;

fn main() {
    let a: Matrix<2, 3> = Matrix::from_rows([[1, 2, 3], [4, 5, 6]]);
    let b: Matrix<2, 3> = Matrix::from_rows([[1, 0, 0], [0, 1, 0]]);
    let product = a.mul(&b);
    println!("{:?}", product.rows());
}
