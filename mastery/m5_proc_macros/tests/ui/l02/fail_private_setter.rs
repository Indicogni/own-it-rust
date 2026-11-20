//! Lesson 02, task C: a private field does not get a public setter.
//! This program must NOT compile. The expected compiler output is in
//! `fail_private_setter.stderr`.

mod model {
    use m5_proc_macros::Setters;

    #[derive(Setters, Default)]
    pub struct Account {
        pub owner: String,
        balance: i64,
    }
}

fn main() {
    let mut account = model::Account::default();
    account.set_owner("ann".into());
    account.set_balance(1_000_000);
}
