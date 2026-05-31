// === Feature: Collections — List Basics ===
// List is a growable array. Construct with `[...]` or `[...]` syntax.
// Methods: push, pop, len, is_empty, contains, first, last, get, insert,
// remove, clear. List uses Rc<RefCell<>> — cloning shares the same data.

// === Construction ===

#[test]
fn test_vec_macro_empty() {
    val v = [];
    assert_eq(v.len(), 0);
    assert(v.is_empty());
}

#[test]
fn test_vec_macro_elements() {
    val v = [1, 2, 3];
    assert_eq(v.len(), 3);
}

#[test]
fn test_array_literal() {
    val v = [10, 20, 30];
    assert_eq(v.len(), 3);
}

#[test]
fn test_array_literal_empty() {
    val v = [];
    assert_eq(v.len(), 0);
}

// === push / pop ===

#[test]
fn test_push_pop() {
    var v = [];
    v.push(10);
    v.push(20);
    v.push(30);
    assert_eq(v.len(), 3);

    val x = v.pop();
    assert(x.is_some());
    assert_eq(v.len(), 2);
}

#[test]
fn test_pop_empty() {
    var v = [];
    val x = v.pop();
    // pop on empty returns None
    assert(x.is_none());
}

#[test]
fn test_push_many() {
    var v = [];
    var i = 0;
    while i < 100 {
        v.push(i);
        i = i + 1;
    }
    assert_eq(v.len(), 100);
}

// === len / is_empty ===

#[test]
fn test_len_and_is_empty() {
    val v = [1, 2, 3];
    assert_eq(v.len(), 3);
    assert(!v.is_empty());

    val empty = [];
    assert_eq(empty.len(), 0);
    assert(empty.is_empty());
}

// === contains ===

#[test]
fn test_contains() {
    val v = [10, 20, 30];
    assert(v.contains(10));
    assert(v.contains(30));
    assert(!v.contains(99));
}

#[test]
fn test_contains_empty() {
    val v = [];
    assert(!v.contains(42));
}

// === first / last ===

#[test]
fn test_first() {
    val v = ["a", "b", "c"];
    val f = v.first();
    assert(f.is_some());
    assert(!f.is_none());
}

#[test]
fn test_first_empty() {
    val v = [];
    val f = v.first();
    assert(f.is_none());
}

#[test]
fn test_last() {
    val v = ["a", "b", "c"];
    val l = v.last();
    assert(l.is_some());
}

#[test]
fn test_last_empty() {
    val v = [];
    val l = v.last();
    assert(l.is_none());
}

// === get ===

#[test]
fn test_get_valid() {
    val v = ["a", "b", "c"];
    val x = v.get(1);
    assert(x.is_some());
}

#[test]
fn test_get_out_of_bounds() {
    val v = ["a", "b", "c"];
    val x = v.get(10);
    assert(x.is_none());
}

// === insert ===

#[test]
fn test_insert_front() {
    var v = [2, 3];
    v.insert(0, 1);
    assert_eq(v.len(), 3);
}

#[test]
fn test_insert_middle() {
    var v = [1, 3];
    v.insert(1, 2);
    assert_eq(v.len(), 3);
}

// === remove ===

#[test]
fn test_remove_valid() {
    var v = ["a", "b", "c"];
    v.remove(1);
    assert_eq(v.len(), 2);
}

// === clear ===

#[test]
fn test_clear() {
    var v = [1, 2, 3];
    v.clear();
    assert_eq(v.len(), 0);
    assert(v.is_empty());
}
