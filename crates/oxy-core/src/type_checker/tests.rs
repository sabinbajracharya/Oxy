#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::vm::run;

    #[test]
    fn test_valid_code_passes_type_checking() {
        let result = run(r#"
            fn add(x: i64, y: i64) -> i64 { x + y }
            fn main() {
                let a: i64 = 42;
                let b: i64 = add(a, 10);
                let c: String = "hello";
            }
            "#);
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_let_type_mismatch_fails() {
        let result = run(r#"
            fn main() {
                let x: i64 = "not a number";
            }
            "#);
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("type mismatch"),
            "expected type mismatch, got: {err}"
        );
        assert!(err.contains("i64"), "expected i64 in error, got: {err}");
    }

    #[test]
    fn test_return_type_mismatch_fails() {
        let result = run(r#"
            fn foo() -> i64 { "wrong" }
            fn main() { foo(); }
            "#);
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("type mismatch"),
            "expected type mismatch, got: {err}"
        );
    }

    #[test]
    fn test_valid_without_type_annotations_passes() {
        let result = run(r#"
            fn main() {
                let x = 42;
                let y = x + 1;
                let z = "hello";
            }
            "#);
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_const_type_mismatch_fails() {
        let result = run(r#"
            const X: i64 = "wrong";
            fn main() {}
            "#);
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("type mismatch"),
            "expected type mismatch, got: {err}"
        );
    }
}
