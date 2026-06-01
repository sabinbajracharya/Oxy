// === Feature: method-call & path-call argument type checking ===

struct Counter {
    count: Int,
}

impl Counter {
    fn new() -> Counter {
        Counter { count: 0 }
    }

    fn add(self, n: Int) -> Int {
        self.count + n
    }

    fn label(self, prefix: String) -> String {
        f"{prefix}: {self.count}"
    }
}

#[test]
fn test_method_call_matching_arg() {
    val c = Counter::new();
    assert::eq(c.add(5), 5);
}

#[test]
fn test_method_call_string_arg() {
    val c = Counter::new();
    assert::eq(c.label("count".to_string()), "count: 0".to_string());
}

#[test]
fn test_path_call_with_correct_types() {
    val c = Counter::new();
    assert::eq(c.add(10), 10);
}

#[compile_error]
fn test_method_call_string_for_int_param_rejected() {
    val c = Counter::new();
    val _ = c.add("five".to_string());
}

#[compile_error]
fn test_method_call_int_for_string_param_rejected() {
    val c = Counter::new();
    val _ = c.label(42);
}

#[compile_error]
fn test_method_call_too_few_args_rejected() {
    val c = Counter::new();
    val _ = c.add();
}

#[compile_error]
fn test_method_call_too_many_args_rejected() {
    val c = Counter::new();
    val _ = c.add(1, 2);
}

// Path-call variant: Counter::add(c, 5) doesn't compile in Oxy since
// methods take self as the first param implicitly — but a module-scoped
// free fn invoked via path should be checked the same way.

mod math {
    pub fn scale(n: Int, factor: Int) -> Int {
        n * factor
    }
}

#[test]
fn test_path_call_matching_args() {
    assert::eq(math::scale(3, 4), 12);
}

#[compile_error]
fn test_path_call_string_for_int_rejected() {
    val _ = math::scale("3".to_string(), 4);
}

#[compile_error]
fn test_path_call_arity_mismatch_rejected() {
    val _ = math::scale(3);
}
