// === Feature: Fixed-Size Arrays [T; N] ===

// === Repeat Expression ===

#[test]
fn test_array_repeat() {
    val arr = [0; 5];
    assert_eq(arr.len(), 5);
    assert_eq(arr[0], 0);
    assert_eq(arr[4], 0);
}

#[test]
fn test_array_repeat_with_expression() {
    val arr = [42; 3];
    assert_eq(arr.len(), 3);
    assert_eq(arr[0], 42);
    assert_eq(arr[1], 42);
    assert_eq(arr[2], 42);
}

// === Type Annotation ===

#[test]
fn test_array_type_annotation() {
    val arr: [Int; 3] = [10, 20, 30];
    assert_eq(arr.len(), 3);
    assert_eq(arr[0], 10);
    assert_eq(arr[1], 20);
    assert_eq(arr[2], 30);
}

// === Indexing ===

#[test]
fn test_array_indexing() {
    val arr = [100, 200, 300];
    assert_eq(arr[0], 100);
    assert_eq(arr[2], 300);
}

// === Equality ===

#[test]
fn test_array_equality() {
    val a = [1, 2, 3];
    val b = [1, 2, 3];
    val c = [4, 5, 6];
    assert_eq(a, b);
    assert_ne(a, c);
}

// === Nested Arrays ===

#[test]
fn test_nested_array() {
    val matrix: [[Int; 2]; 2] = [[1, 2], [3, 4]];
    assert_eq(matrix[0][0], 1);
    assert_eq(matrix[0][1], 2);
    assert_eq(matrix[1][0], 3);
    assert_eq(matrix[1][1], 4);
}

// === Iteration ===

#[test]
fn test_array_iteration() {
    val arr = [1, 2, 3];
    var sum = 0;
    for v in arr {
        sum = sum + v;
    }
    assert_eq(sum, 6);
}

// === Display ===

#[test]
fn test_array_display() {
    val arr = [1, 2, 3];
    assert_eq(format("{}", arr), "[1, 2, 3]");
}

// === String Array ===

#[test]
fn test_string_array() {
    val arr: [String; 2] = ["hello".to_string(), "world".to_string()];
    assert_eq(arr[0], "hello");
    assert_eq(arr[1], "world");
}

// === Bool Array ===

#[test]
fn test_bool_array() {
    val arr = [true, false, true];
    assert(arr[0]);
    assert(!arr[1]);
    assert(arr[2]);
}

// === Compile Error: Non-constant repeat count ===

#[compile_error]
fn test_array_repeat_non_constant_count() {
    // Repeat count must be a compile-time constant
    val n = 5;
    val arr = [0; n];
}
