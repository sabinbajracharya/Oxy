// === Feature: Let-Destructuring ===
// `val` accepts tuple patterns to bind multiple variables at once. Nested
// tuple patterns work recursively so you can pull values out of deeply
// structured data in one statement.

#[test]
fn test_let_destructure_pair() {
    val (a, b) = (1, 2);
    assert::eq(a, 1);
    assert::eq(b, 2);
}

#[test]
fn test_let_destructure_triple() {
    val (a, b, c) = (10, 20, 30);
    assert::eq(a, 10);
    assert::eq(b, 20);
    assert::eq(c, 30);
}

#[test]
fn test_let_destructure_with_wildcard() {
    val (a, _, c) = (1, 2, 3);
    assert::eq(a, 1);
    assert::eq(c, 3);
}

#[test]
fn test_let_destructure_nested_pair_of_pairs() {
    val ((a, b), (c, d)) = ((1, 2), (3, 4));
    assert::eq(a, 1);
    assert::eq(b, 2);
    assert::eq(c, 3);
    assert::eq(d, 4);
}

#[test]
fn test_let_destructure_nested_three_deep() {
    val ((a, b), c) = ((1, 2), (3, 4));
    val (c1, c2) = c;
    assert::eq(a, 1);
    assert::eq(b, 2);
    assert::eq(c1, 3);
    assert::eq(c2, 4);
}

#[test]
fn test_let_destructure_nested_with_wildcard() {
    val ((a, _), (_, d)) = ((1, 2), (3, 4));
    assert::eq(a, 1);
    assert::eq(d, 4);
}

#[test]
fn test_let_destructure_mixed_simple_and_nested() {
    val (a, (b, c)) = (1, (2, 3));
    assert::eq(a, 1);
    assert::eq(b, 2);
    assert::eq(c, 3);
}

#[test]
fn test_let_destructure_in_loop_body() {
    val pairs = [(1, 10), (2, 20), (3, 30)];
    var sum = 0;
    for pair in pairs {
        val (a, b) = pair;
        sum = sum + a + b;
    }
    assert::eq(sum, 66);
}
