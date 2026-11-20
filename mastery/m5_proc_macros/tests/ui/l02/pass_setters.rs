//! Lesson 02, task C: `#[derive(Setters)]`.
//! This program must compile and run.

mod model {
    use m5_proc_macros::Setters;

    /// Not `Clone` and not `Default`: a setter must move values, nothing more.
    #[derive(Debug, PartialEq)]
    pub struct Token(pub String);

    #[derive(Setters)]
    pub struct Session {
        pub user: String,
        pub token: Token,
        pub(crate) r#type: u8,
        // A private field gets a private setter. It is used only in this module.
        attempts: u32,
    }

    impl Session {
        pub fn new() -> Session {
            Session {
                user: "ann".into(),
                token: Token("t1".into()),
                r#type: 1,
                attempts: 0,
            }
        }

        pub fn fail(&mut self) -> u32 {
            let seen = self.attempts;
            self.set_attempts(seen + 1)
        }

        pub fn attempts(&self) -> u32 {
            self.attempts
        }
    }
}

fn main() {
    let mut session = model::Session::new();

    let old = session.set_user("bob".into());
    assert_eq!(old, "ann", "a setter returns the old value");
    assert_eq!(session.user, "bob");

    let old = session.set_token(model::Token("t2".into()));
    assert_eq!(old, model::Token("t1".into()));
    assert_eq!(session.token, model::Token("t2".into()));

    assert_eq!(
        session.set_type(7),
        1,
        "`r#type` gets the setter `set_type`"
    );
    assert_eq!(session.r#type, 7);

    assert_eq!(session.fail(), 0);
    assert_eq!(session.attempts(), 1);
}
