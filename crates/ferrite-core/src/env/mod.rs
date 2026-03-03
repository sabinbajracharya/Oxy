//! Environment (scope) management for the Ferrite interpreter.
//!
//! Implements lexical scoping with a parent chain. Each scope holds variable
//! bindings and whether they are mutable.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::errors::FerriError;
use crate::types::Value;

/// A shared reference to an environment.
pub type Env = Rc<RefCell<Environment>>;

/// A lexical scope containing variable bindings.
#[derive(Debug, Clone, Default)]
pub struct Environment {
    /// Variable bindings: name → (value, is_mutable).
    values: HashMap<String, (Value, bool)>,
    /// Parent scope (if any).
    parent: Option<Env>,
}

impl Environment {
    /// Create a new global (root) environment.
    pub fn new() -> Env {
        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            parent: None,
        }))
    }

    /// Create a child scope with this environment as parent.
    pub fn child(parent: &Env) -> Env {
        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            parent: Some(Rc::clone(parent)),
        }))
    }

    /// Define a new variable in the current scope.
    pub fn define(&mut self, name: String, value: Value, mutable: bool) {
        self.values.insert(name, (value, mutable));
    }

    /// Look up a variable by name, searching up the parent chain.
    pub fn get(&self, name: &str) -> Result<Value, FerriError> {
        if let Some((value, _)) = self.values.get(name) {
            Ok(value.clone())
        } else if let Some(parent) = &self.parent {
            parent.borrow().get(name)
        } else {
            Err(FerriError::Runtime {
                message: format!("undefined variable '{name}'"),
                line: 0,
                column: 0,
            })
        }
    }

    /// Assign a new value to an existing variable. Searches up the parent chain.
    /// Returns an error if the variable doesn't exist or isn't mutable.
    pub fn set(&mut self, name: &str, value: Value) -> Result<(), FerriError> {
        if let Some((existing, mutable)) = self.values.get_mut(name) {
            if !*mutable {
                return Err(FerriError::Runtime {
                    message: format!("cannot assign to immutable variable '{name}'"),
                    line: 0,
                    column: 0,
                });
            }
            *existing = value;
            Ok(())
        } else if let Some(parent) = &self.parent {
            parent.borrow_mut().set(name, value)
        } else {
            Err(FerriError::Runtime {
                message: format!("undefined variable '{name}'"),
                line: 0,
                column: 0,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_define_and_get() {
        let env = Environment::new();
        env.borrow_mut()
            .define("x".into(), Value::Integer(42), false);
        assert_eq!(env.borrow().get("x").unwrap(), Value::Integer(42));
    }

    #[test]
    fn test_undefined_variable() {
        let env = Environment::new();
        assert!(env.borrow().get("x").is_err());
    }

    #[test]
    fn test_child_scope_lookup() {
        let parent = Environment::new();
        parent
            .borrow_mut()
            .define("x".into(), Value::Integer(1), false);

        let child = Environment::child(&parent);
        child
            .borrow_mut()
            .define("y".into(), Value::Integer(2), false);

        // Child can see parent's variables
        assert_eq!(child.borrow().get("x").unwrap(), Value::Integer(1));
        assert_eq!(child.borrow().get("y").unwrap(), Value::Integer(2));

        // Parent cannot see child's variables
        assert!(parent.borrow().get("y").is_err());
    }

    #[test]
    fn test_set_mutable() {
        let env = Environment::new();
        env.borrow_mut().define("x".into(), Value::Integer(1), true);
        env.borrow_mut().set("x", Value::Integer(2)).unwrap();
        assert_eq!(env.borrow().get("x").unwrap(), Value::Integer(2));
    }

    #[test]
    fn test_set_immutable_fails() {
        let env = Environment::new();
        env.borrow_mut()
            .define("x".into(), Value::Integer(1), false);
        let result = env.borrow_mut().set("x", Value::Integer(2));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot assign to immutable"));
    }

    #[test]
    fn test_set_in_parent_scope() {
        let parent = Environment::new();
        parent
            .borrow_mut()
            .define("x".into(), Value::Integer(1), true);

        let child = Environment::child(&parent);
        child.borrow_mut().set("x", Value::Integer(99)).unwrap();

        assert_eq!(parent.borrow().get("x").unwrap(), Value::Integer(99));
    }

    #[test]
    fn test_shadowing() {
        let parent = Environment::new();
        parent
            .borrow_mut()
            .define("x".into(), Value::Integer(1), false);

        let child = Environment::child(&parent);
        child
            .borrow_mut()
            .define("x".into(), Value::String("shadowed".into()), false);

        assert_eq!(
            child.borrow().get("x").unwrap(),
            Value::String("shadowed".into())
        );
        // Parent still has original
        assert_eq!(parent.borrow().get("x").unwrap(), Value::Integer(1));
    }
}
