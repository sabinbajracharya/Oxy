#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::vm::run_compiled;

    #[test]
    fn test_valid_code_passes_type_checking() {
        let result = run_compiled(
            r#"
            fn add(x: Int, y: Int) -> Int { x + y }
            fn main() {
                let a: Int = 42;
                let b: Int = add(a, 10);
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
                let x: Int = "not a number";
            }
            "#,
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("type mismatch"),
            "expected type mismatch, got: {err}"
        );
        assert!(err.contains("Int"), "expected Int in error, got: {err}");
    }

    #[test]
    fn test_return_type_mismatch_fails() {
        let result = run_compiled(
            r#"
            fn foo() -> Int { "wrong" }
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
            const X: Int = "wrong";
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
            fn apply_Int(f: fn(Int) -> Int, x: Int) -> Int { f(x) }
            fn main() {
                let _ = apply_Int(|x| x * 2, 21);
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
            fn apply_Int(f: fn(Int) -> Int, x: Int) -> Int { f(x) }
            fn main() {
                // |x: Float| ... produces fn(Float) -> ?, not fn(Int) -> Int
                let _ = apply_Int(|x: Float| x * 2.0, 21);
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
            fn fold_two(a: Int, b: Int, f: fn(Int, Int) -> Int) -> Int { f(a, b) }
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
                let x: Float = 42;
                let y: Float = 0;
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
                let a: Byte = 0;
                let b: Byte = 255;
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_empty_array_typed_from_expected_list() {
        // `let v: Vec<String> = []` — empty array typed from expected Vec.
        let result = run_compiled(
            r#"
            fn main() {
                let v: List<String> = [];
                let w: List<Int> = [];
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
            fn first_elem<T>(v: List<T>) -> Option<T> {
                if v.len() == 0 { None } else { Some(v[0]) }
            }
            fn main() {
                let x = first_elem([1, 2, 3]);
                // x should be Option<Int>
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
                // Expected type fn(Int) -> Int flows into closure via
                // bidirectional inference from the let annotation.
                let f: fn(Int) -> Int = |x| x * 2;
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
            fn map_list(v: List<Int>, f: fn(Int) -> Int) -> List<Int> { v.map(f) }
            fn main() {
                let v = [1, 2, 3];
                // Closure |x| x + 1: x inferred as Int from fn(Int) -> Int
                let result = map_list(v, |x| x + 1);
                let _ = result;
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    // --- Phase 3.1: pipeline operator |> ---

    #[test]
    fn test_pipeline_basic_call() {
        // `5 |> double()` desugars to `double(5)`
        let result = run_compiled(
            r#"
            fn double(x: Int) -> Int { x * 2 }
            fn main() {
                let r = 5 |> double();
                let _ = r;
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_pipeline_with_args() {
        // `5 |> add(3)` desugars to `add(5, 3)`
        let result = run_compiled(
            r#"
            fn add(a: Int, b: Int) -> Int { a + b }
            fn main() {
                let r = 5 |> add(3);
                let _ = r;
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_pipeline_chain() {
        // `5 |> double() |> add(3)` desugars to `add(double(5), 3)`
        let result = run_compiled(
            r#"
            fn double(x: Int) -> Int { x * 2 }
            fn add(a: Int, b: Int) -> Int { a + b }
            fn main() {
                let r = 5 |> double() |> add(3);
                let _ = r;
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_pipeline_bare_ident() {
        // `21 |> double` desugars to `double(21)`
        let result = run_compiled(
            r#"
            fn double(x: Int) -> Int { x * 2 }
            fn main() {
                let r = 21 |> double;
                let _ = r;
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_pipeline_type_mismatch_is_rejected() {
        // `s |> doubler` where s is a String and doubler expects int
        let result = run_compiled(
            r#"
            fn doubler(x: Int) -> Int { x * 2 }
            fn main() {
                let s = "hello";
                let _ = s |> doubler;
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

    // --- Phase 3.2: single-line function syntax `fn name(params) -> T = expr` ---

    #[test]
    fn test_single_line_fn_basic() {
        // `fn double(x: int) -> int = x * 2` desugars to block with tail expr
        let result = run_compiled(
            r#"
            fn double(x: Int) -> Int = x * 2
            fn main() { let _ = double(21); }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_single_line_fn_no_return_type() {
        // `fn add(x: int, y: int) = x + y` — return type inferred
        let result = run_compiled(
            r#"
            fn add(x: Int, y: Int) = x + y
            fn main() { let _ = add(10, 32); }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_single_line_fn_with_generics() {
        // Single-line with generic params
        let result = run_compiled(
            r#"
            fn first<T>(v: List<T>) -> Option<T> =
                if v.len() == 0 { None } else { Some(v[0]) }
            fn main() {
                let v = [1, 2, 3];
                let _ = first(v);
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_single_line_fn_pipeline_chain() {
        // Single-line functions compose with pipelines
        let result = run_compiled(
            r#"
            fn double(x: Int) -> Int = x * 2
            fn add(a: Int, b: Int) -> Int = a + b
            fn main() {
                let _ = 5 |> double() |> add(3);
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_single_line_fn_type_mismatch_rejected() {
        // Type mismatch in single-line fn should be caught
        let result = run_compiled(
            r#"
            fn bad(x: Int) -> Int = "wrong"
            fn main() { let _ = bad(1); }
            "#,
        );
        assert!(result.is_err(), "expected type mismatch, got Ok");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("type mismatch"),
            "expected type mismatch error, got: {err}"
        );
    }

    // --- Phase 3.3: pipeline-friendly free functions ---

    #[test]
    fn test_free_fn_map_basic() {
        let result = run_compiled(
            r#"
            fn main() {
                let v = [1, 2, 3];
                let _ = map(v, |x| x * 2);
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_free_fn_filter() {
        let result = run_compiled(
            r#"
            fn main() {
                let v = [1, 2, 3, 4];
                let _ = filter(v, |x| x > 2);
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_free_fn_fold() {
        let result = run_compiled(
            r#"
            fn main() {
                let v = [1, 2, 3];
                let _ = fold(v, 0, |acc, x| acc + x);
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_free_fn_any_all() {
        let result = run_compiled(
            r#"
            fn main() {
                let v = [1, 2, 3];
                let _ = any(v, |x| x > 2);
                let _ = all(v, |x| x > 0);
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_free_fn_find() {
        let result = run_compiled(
            r#"
            fn main() {
                let v = [1, 2, 3];
                let _ = find(v, |x| x > 1);
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_free_fn_collect() {
        let result = run_compiled(
            r#"
            fn main() {
                let v = [1, 2, 3];
                let _ = collect(v);
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_free_fn_sort() {
        let result = run_compiled(
            r#"
            fn main() {
                let v = [3, 1, 2];
                let _ = sort(v);
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_free_fn_with_pipeline() {
        let result = run_compiled(
            r#"
            fn double(x: Int) -> Int = x * 2
            fn is_positive(x: Int) -> bool = x > 0
            fn main() {
                let v = [1, 2, 3];
                // Pipeline chain using free functions
                let _ = v |> map(|x| x + 1) |> filter(|x| x > 2);
            }
            "#,
        );
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_free_fn_wrong_arg_count_fails() {
        let result = run_compiled(
            r#"
            fn main() { let _ = map([1]); }
            "#,
        );
        assert!(result.is_err(), "expected error (wrong arg count), got Ok");
    }
}
