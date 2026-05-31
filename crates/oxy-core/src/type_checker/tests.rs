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

    // --- Phase 2.2: closure parameter inference from expected types ---

    #[test]
    fn test_closure_param_inferred_from_expected_fn_type() {
        // The closure |x| x * 2 has no annotation on x. When passed to
        // apply_int(f: fn(int) -> int, ...), the type checker should infer
        // x: int from the expected function signature.
        let result = run_compiled(
            r#"
            fn apply_int(f: fn(int) -> int, x: int) -> int { f(x) }
            fn main() {
                let _ = apply_int(|x| x * 2, 21);
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_closure_param_annotation_overrides_inference() {
        // When a closure param has an explicit annotation that's incompatible
        // with the expected type, the type checker should reject it.
        let result = run_compiled(
            r#"
            fn apply_int(f: fn(int) -> int, x: int) -> int { f(x) }
            fn main() {
                // |x: float| ... produces fn(float) -> ?, not fn(int) -> int
                let _ = apply_int(|x: float| x * 2.0, 21);
            }
            "#,
        );
        assert!(result.is_err(), "expected type mismatch, got Ok");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("type mismatch"),
            "expected type mismatch error, got: {err}"
        );
    }

    #[test]
    fn test_multi_param_closure_inference() {
        // Multi-param closures should infer all params from expected type.
        let result = run_compiled(
            r#"
            fn fold_two(a: int, b: int, f: fn(int, int) -> int) -> int { f(a, b) }
            fn main() {
                let _ = fold_two(10, 32, |acc, x| acc + x);
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    // --- Phase 2.3: literal auto-cast to expected type ---

    #[test]
    fn test_int_literal_auto_casts_to_float() {
        // `let x: float = 42` — int literal should auto-cast to float.
        let result = run_compiled(
            r#"
            fn main() {
                let x: float = 42;
                let y: float = 0;
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_int_literal_auto_casts_to_byte() {
        // `let b: byte = 0` — int literal should auto-cast to byte.
        let result = run_compiled(
            r#"
            fn main() {
                let a: byte = 0;
                let b: byte = 255;
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_empty_array_typed_from_expected_vec() {
        // `let v: Vec<String> = []` — empty array typed from expected Vec.
        let result = run_compiled(
            r#"
            fn main() {
                let v: Vec<String> = [];
                let w: Vec<int> = [];
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_let_without_annotation_still_infers() {
        // Without type annotation, `let x = 42` should infer int (default).
        let result = run_compiled(
            r#"
            fn main() {
                let x = 42;
                let y = x + 1;
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    // --- Phase 2.4/2.5: generic return + nested/local function inference ---

    #[test]
    fn test_generic_return_type_inference() {
        // first_elem<T>(Vec<T>) -> Option<T> should infer T=int and
        // return Option<int>.
        let result = run_compiled(
            r#"
            fn first_elem<T>(v: Vec<T>) -> Option<T> {
                if v.len() == 0 { None } else { Some(v[0]) }
            }
            fn main() {
                let x = first_elem(vec![1, 2, 3]);
                // x should be Option<int>
                let _ = x.unwrap() + 1;
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_closure_infers_params_from_let_binding_type() {
        // When a closure is assigned to a typed let binding, the expected
        // fn type should flow into the closure (Phase 2.2 + 2.3 combined).
        let result = run_compiled(
            r#"
            fn main() {
                // Expected type fn(int) -> int flows into closure via
                // bidirectional inference from the let annotation.
                let f: fn(int) -> int = |x| x * 2;
                let _ = f(21);
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_closure_infers_from_nested_call_context() {
        // Closure passed to a function with concrete fn type should have
        // params inferred, and the return type flows from the body.
        let result = run_compiled(
            r#"
            fn map_vec(v: Vec<int>, f: fn(int) -> int) -> Vec<int> { v.map(f) }
            fn main() {
                let v = vec![1, 2, 3];
                // Closure |x| x + 1: x inferred as int from fn(int) -> int
                let result = map_vec(v, |x| x + 1);
                let _ = result;
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }
}
