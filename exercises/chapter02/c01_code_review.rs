//! CODE REVIEW 01 — A teammate's first pull request
//! Run: cargo test --test chapter02 c01_
//! Then: cargo clippy --test chapter02
//!
//! This is the kind of code you will be asked to review at work. It compiles
//! and it looks reasonable, and some of its tests already pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug that you can find, and every line that could be written
//!    in a simpler way.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. You MAY change parameter types where the doc comment says so;
//!    the tests are written to accept the better signature.
//! 4. Run clippy and resolve its warnings for this file.

/// Return the average of the scores, or 0.0 for an empty list. The average
/// of 1 and 2 is 1.5. You may change the parameter type.
fn average(scores: &Vec<u32>) -> f64 {
    let mut total = 0;
    for i in 0..scores.len() {
        total = total + scores[i];
    }
    return (total / scores.len() as u32) as f64;
}

/// Count the scores that pass. A score passes when it is at least
/// `pass_mark`. You may change the parameter type.
fn count_passing(scores: &Vec<u32>, pass_mark: u32) -> usize {
    let mut count = 0;
    for i in 1..scores.len() {
        if (scores[i] > pass_mark) == true {
            count = count + 1;
        }
    }
    return count;
}

/// Return a result line such as "Ada: pass" or "Tim: fail". You may change
/// the parameter type.
fn result_line(name: &String, passed: bool) -> String {
    let mut line = String::new();
    line.push_str(&name.clone());
    line.push_str(": ");
    if passed == true {
        line.push_str("pass");
    } else {
        line.push_str("fail");
    }
    return line;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a vector. `&scores(&[1, 2])` can be passed to a `&Vec<u32>`
    /// parameter and also to a `&[u32]` parameter.
    fn scores(values: &[u32]) -> Vec<u32> {
        values.to_vec()
    }

    #[test]
    fn a_average_of_whole_numbers() {
        assert_eq!(average(&scores(&[10, 20, 30])), 20.0);
        assert_eq!(average(&scores(&[7])), 7.0);
    }

    #[test]
    fn a_average_keeps_the_fraction() {
        assert_eq!(
            average(&scores(&[1, 2])),
            1.5,
            "the average of 1 and 2 is 1.5; a result of 1.0 means that the division \
             was done with whole numbers"
        );
        assert_eq!(average(&scores(&[1, 2, 2])), 5.0 / 3.0);
    }

    #[test]
    fn a_average_of_nothing_is_zero() {
        assert_eq!(
            average(&scores(&[])),
            0.0,
            "an empty list has the average 0.0, and it must not panic"
        );
    }

    #[test]
    fn b_scores_above_the_mark_pass() {
        assert_eq!(count_passing(&scores(&[10, 80, 90]), 50), 2);
        assert_eq!(count_passing(&scores(&[]), 50), 0);
    }

    #[test]
    fn b_the_first_score_counts_too() {
        assert_eq!(
            count_passing(&scores(&[90, 10, 80]), 50),
            2,
            "90 and 80 pass; if you got 1, the loop skipped the first score"
        );
    }

    #[test]
    fn b_a_score_equal_to_the_mark_passes() {
        assert_eq!(
            count_passing(&scores(&[10, 50, 49, 51]), 50),
            2,
            "`at least 50` includes 50 itself"
        );
    }

    #[test]
    fn c_result_line() {
        assert_eq!(result_line(&String::from("Ada"), true), "Ada: pass");
        assert_eq!(result_line(&String::from("Tim"), false), "Tim: fail");
    }
}
