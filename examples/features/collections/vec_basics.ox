// === Feature: Collections — List Basics ===
// List is a growable array. Construct with `list(...)` or `[...]` syntax.
// Methods: push, pop, len, is_empty, contains, first, last, get, insert,
// remove, clear. List uses Rc<RefCell<>> — cloning shares the same data.

// === Construction ===

#[test]
fn test_vec_macro_empty() {
    let v = list();
    assert_eq(v.len(), 0);
    assert(v.is_empty());
}

#[test]
fn test_vec_macro_elements() {
    let v = list(1, 2, 3);
    assert_eq(v.len(), 3);
}

#[test]
fn test_array_literal() {
    let v = [10, 20, 30];
    assert_eq(v.len(), 3);
}

#[test]
fn test_array_literal_empty() {
    let v = [];
    assert_eq(v.len(), 0);
}

// === push / pop ===

#[test]
fn test_push_pop() {
    let mut v = list();
    v.push(10);
    v.push(20);
    v.push(30);
    assert_eq(v.len(), 3);

    let x = v.pop();
    assert(x.is_some());
    assert_eq(v.len(), 2);
}

#[test]
fn test_pop_empty() {
    let mut v = list();
    let x = v.pop();
    // pop on empty returns None
    assert(x.is_none());
}

#[test]
fn test_push_many() {
    let mut v = list();
    let mut i = 0;
    while i < 100 {
        v.push(i);
        i = i + 1;
    }
    assert_eq(v.len(), 100);
}

// === len / is_empty ===

#[test]
fn test_len_and_is_empty() {
    let v = list(1, 2, 3);
    assert_eq(v.len(), 3);
    assert(!v.is_empty());

    let empty = list();
    assert_eq(empty.len(), 0);
    assert(empty.is_empty());
}

// === contains ===

#[test]
fn test_contains() {
    let v = list(10, 20, 30);
    assert(v.contains(10));
    assert(v.contains(30));
    assert(!v.contains(99));
}

#[test]
fn test_contains_empty() {
    let v = list();
    assert(!v.contains(42));
}

// === first / last ===

#[test]
fn test_first() {
    let v = list("a", "b", "c");
    let f = v.first();
    assert(f.is_some());
    assert(!f.is_none());
}

#[test]
fn test_first_empty() {
    let v = list();
    let f = v.first();
    assert(f.is_none());
}

#[test]
fn test_last() {
    let v = list("a", "b", "c");
    let l = v.last();
    assert(l.is_some());
}

#[test]
fn test_last_empty() {
    let v = list();
    let l = v.last();
    assert(l.is_none());
}

// === get ===

#[test]
fn test_get_valid() {
    let v = list("a", "b", "c");
    let x = v.get(1);
    assert(x.is_some());
}

#[test]
fn test_get_out_of_bounds() {
    let v = list("a", "b", "c");
    let x = v.get(10);
    assert(x.is_none());
}

// === insert ===

#[test]
fn test_insert_front() {
    let mut v = list(2, 3);
    v.insert(0, 1);
    assert_eq(v.len(), 3);
}

#[test]
fn test_insert_middle() {
    let mut v = list(1, 3);
    v.insert(1, 2);
    assert_eq(v.len(), 3);
}

// === remove ===

#[test]
fn test_remove_valid() {
    let mut v = list("a", "b", "c");
    v.remove(1);
    assert_eq(v.len(), 2);
}

// === clear ===

#[test]
fn test_clear() {
    let mut v = list(1, 2, 3);
    v.clear();
    assert_eq(v.len(), 0);
    assert(v.is_empty());
}
