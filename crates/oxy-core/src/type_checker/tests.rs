#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::vm::run_compiled;

    #[test]
    fn test_valid_code_passes_type_checking() {
        let result = run_compiled(
            r#"
            fn add(x: int, y: int) -> int { x + y }
            fn main() {
                let a: int = 42;
                let b: int = add(a, 10);
                let c: String = "hello";
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_let_type_mismatch_fails() {
        let result = run_compiled(
            r#"
            fn main() {
                let x: int = "not a number";
            }
            "#,
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("type mismatch"),
            "expected type mismatch, got: {err}"
        );
        assert!(err.contains("int"), "expected int in error, got: {err}");
    }

    #[test]
    fn test_return_type_mismatch_fails() {
        let result = run_compiled(
            r#"
            fn foo() -> int { "wrong" }
            fn main() { foo(); }
            "#,
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("type mismatch"),
            "expected type mismatch, got: {err}"
        );
    }

    #[test]
    fn test_valid_without_type_annotations_passes() {
        let result = run_compiled(
            r#"
            fn main() {
                let x = 42;
                let y = x + 1;
                let z = "hello";
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_const_type_mismatch_fails() {
        let result = run_compiled(
            r#"
            const X: int = "wrong";
            fn main() {}
            "#,
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("type mismatch"),
            "expected type mismatch, got: {err}"
        );
    }
}
