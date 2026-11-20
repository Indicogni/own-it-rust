//! The table from step names to step implementations.

use tasklane_plugin::Step;

use crate::builtin;
use crate::error::Error;

/// The steps a [`Runner`](crate::Runner) can use.
///
/// ```
/// use tasklane_core::Registry;
///
/// let registry = Registry::with_builtins();
/// assert!(registry.names().contains(&"upper"));
/// ```
pub struct Registry {
    steps: Vec<Box<dyn Step>>,
}

impl Registry {
    /// A registry without any steps.
    pub fn new() -> Self {
        Registry { steps: Vec::new() }
    }

    /// A registry with every step of the [`builtin`] module.
    pub fn with_builtins() -> Self {
        let mut registry = Registry::new();
        for step in builtin::all() {
            registry
                .register(step)
                .expect("built-in step names are unique");
        }
        registry
    }

    /// Adds a step.
    ///
    /// # Errors
    ///
    /// [`Error::DuplicateStep`] if a step with the same name is already registered.
    /// The registry is unchanged in that case.
    pub fn register(&mut self, step: Box<dyn Step>) -> Result<(), Error> {
        if self.steps.iter().any(|known| known.name() == step.name()) {
            return Err(Error::DuplicateStep(step.name().to_string()));
        }
        self.steps.push(step);
        Ok(())
    }

    /// The step called `name`.
    // The signature has been public since 0.1 and plugins call it. Changing it is a
    // breaking change; see issues/0027-registry-lookup-signature.md.
    #[allow(clippy::ptr_arg, clippy::borrowed_box)]
    pub fn lookup(&self, name: &String) -> Option<&Box<dyn Step>> {
        self.steps.iter().find(|step| step.name() == name)
    }

    /// The names of all steps, in registration order.
    pub fn names(&self) -> Vec<&str> {
        self.steps.iter().map(|step| step.name()).collect()
    }
}

impl Default for Registry {
    /// The same as [`Registry::with_builtins`].
    fn default() -> Self {
        Registry::with_builtins()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tasklane_plugin::{StepContext, StepError, Workspace};

    struct Nop(&'static str);

    impl Step for Nop {
        fn name(&self) -> &str {
            self.0
        }
        fn usage(&self) -> &str {
            "nop"
        }
        fn run(
            &self,
            _: &[String],
            _: &mut Workspace,
            _: &StepContext<'_>,
        ) -> Result<(), StepError> {
            Ok(())
        }
    }

    #[test]
    fn register_and_lookup() {
        let mut registry = Registry::new();
        registry.register(Box::new(Nop("one"))).unwrap();
        registry.register(Box::new(Nop("two"))).unwrap();
        assert_eq!(registry.names(), ["one", "two"]);
        assert!(registry.lookup(&"two".to_string()).is_some());
        assert!(registry.lookup(&"three".to_string()).is_none());
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let mut registry = Registry::new();
        registry.register(Box::new(Nop("one"))).unwrap();
        assert_eq!(
            registry.register(Box::new(Nop("one"))).err(),
            Some(Error::DuplicateStep("one".to_string()))
        );
        assert_eq!(registry.names().len(), 1);
    }

    #[test]
    fn a_plugin_cannot_replace_a_builtin() {
        let mut registry = Registry::with_builtins();
        assert!(registry.register(Box::new(Nop("write"))).is_err());
    }
}
