//@ expect: run
//@ why: Matrix must carry its shape in its type: transpose swaps the parameters, mul chains them, identity is square.
use m1_types::l07_sealed_typestate::Matrix;

fn main() {
    let a: Matrix<2, 3> = Matrix::from_rows([[1, 2, 3], [4, 5, 6]]);
    let t: Matrix<3, 2> = a.transpose();
    assert_eq!(t.rows(), &[[1, 4], [2, 5], [3, 6]]);

    // (2 x 3) times (3 x 2) is (2 x 2); the result type is inferred.
    let product = a.mul(&t);
    let expected: &[[i64; 2]; 2] = &[[14, 32], [32, 77]];
    assert_eq!(product.rows(), expected);

    // (3 x 2) times (2 x 3) is (3 x 3).
    let other: Matrix<3, 3> = t.mul(&a);
    assert_eq!(other.rows()[0], [17, 22, 27]);

    let identity = Matrix::<3, 3>::identity();
    assert_eq!(identity.rows(), &[[1, 0, 0], [0, 1, 0], [0, 0, 1]]);
    assert_eq!(a.mul(&identity).rows(), a.rows());

    let column: Matrix<3, 1> = Matrix::from_rows([[1], [0], [2]]);
    assert_eq!(a.mul(&column).rows(), &[[7], [16]]);

    assert_eq!(
        size_of::<Matrix<2, 3>>(),
        size_of::<[[i64; 3]; 2]>(),
        "a matrix must be exactly its numbers; the shape lives in the type"
    );
}
