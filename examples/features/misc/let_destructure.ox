// === Feature: Let-Destructuring ===
// `let` accepts tuple patterns to bind multiple variables at once. Nested
// tuple patterns work recursively so you can pull values out of deeply
// structured data in one statement.

#[test]
fn test_let_destructure_pair() {
    let (a, b) = (1, 2);
    assert_eq!(a, 1);
    assert_eq!(b, 2);
}

#[test]
fn test_let_destructure_triple() {
    let (a, b, c) = (10, 20, 30);
    assert_eq!(a, 10);
    assert_eq!(b, 20);
    assert_eq!(c, 30);
}

#[test]
fn test_let_destructure_with_wildcard() {
    let (a, _, c) = (1, 2, 3);
    assert_eq!(a, 1);
    assert_eq!(c, 3);
}

#[test]
fn test_let_destructure_nested_pair_of_pairs() {
    let ((a, b), (c, d)) = ((1, 2), (3, 4));
    assert_eq!(a, 1);
    assert_eq!(b, 2);
    assert_eq!(c, 3);
    assert_eq!(d, 4);
}

#[test]
fn test_let_destructure_nested_three_deep() {
    let ((a, b), c) = ((1, 2), (3, 4));
    let (c1, c2) = c;
    assert_eq!(a, 1);
    assert_eq!(b, 2);
    assert_eq!(c1, 3);
    assert_eq!(c2, 4);
}

#[test]
fn test_let_destructure_nested_with_wildcard() {
    let ((a, _), (_, d)) = ((1, 2), (3, 4));
    assert_eq!(a, 1);
    assert_eq!(d, 4);
}

#[test]
fn test_let_destructure_mixed_simple_and_nested() {
    let (a, (b, c)) = (1, (2, 3));
    assert_eq!(a, 1);
    assert_eq!(b, 2);
    assert_eq!(c, 3);
}

#[test]
fn test_let_destructure_in_loop_body() {
    let pairs = [(1, 10), (2, 20), (3, 30)];
    let mut sum = 0;
    for pair in pairs {
        let (a, b) = pair;
        sum = sum + a + b;
    }
    assert_eq!(sum, 66);
}
