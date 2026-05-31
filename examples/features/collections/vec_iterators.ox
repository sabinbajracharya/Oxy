// === Feature: Collections — List Iterators ===
// List supports iterator methods: map, filter, fold, any, all, find,
// position, sum, count, for_each, enumerate, zip, chain, take, skip,
// rev. Methods that return iterators (enumerate, zip, take, skip, chain,
// rev) don't have .len() — iterate over them instead.

// === map ===

#[test]
fn test_map() {
    let v = list(1, 2, 3);
    let doubled = v.map(|x| x * 2);
    assert_eq(doubled.len(), 3);
}

// === filter ===

#[test]
fn test_filter() {
    let v = list(1, 2, 3, 4, 5, 6);
    let evens = v.filter(|x| x % 2 == 0);
    assert_eq(evens.len(), 3);
}

// === fold ===

#[test]
fn test_fold_sum() {
    let v = list(1, 2, 3, 4);
    let result = v.fold(0, |acc, x| acc + x);
    assert_eq(result, 10);
}

#[test]
fn test_fold_multiply() {
    let v = list(1, 2, 3, 4);
    let result = v.fold(1, |acc, x| acc * x);
    assert_eq(result, 24);
}

// === any / all ===

#[test]
fn test_any_true() {
    let v = list(1, 3, 5, 8);
    assert(v.any(|x| x % 2 == 0));
}

#[test]
fn test_any_false() {
    let v = list(1, 3, 5, 7);
    assert(!v.any(|x| x % 2 == 0));
}

#[test]
fn test_all_true() {
    let v = list(2, 4, 6, 8);
    assert(v.all(|x| x % 2 == 0));
}

#[test]
fn test_all_false() {
    let v = list(2, 3, 4);
    assert(!v.all(|x| x % 2 == 0));
}

// === find ===

#[test]
fn test_find_some() {
    let v = list(1, 3, 5, 8, 9);
    let r = v.find(|x| x % 2 == 0);
    assert(r.is_some());
}

#[test]
fn test_find_none() {
    let v = list(1, 3, 5, 7);
    let r = v.find(|x| x % 2 == 0);
    assert(r.is_none());
}

// === position ===

#[test]
fn test_position_found() {
    let v = list("a", "b", "c", "d");
    let r = v.position(|x| x == "c");
    assert(r.is_some());
}

#[test]
fn test_position_not_found() {
    let v = list("a", "b", "c");
    let r = v.position(|x| x == "z");
    assert(r.is_none());
}

// === sum / count ===

#[test]
fn test_sum() {
    let v = list(1, 2, 3, 4, 5);
    assert_eq(v.sum(), 15);
}

#[test]
fn test_count() {
    let v = list(1, 2, 3, 4, 5);
    assert_eq(v.count(), 5);
}

// === for_each ===

#[test]
fn test_for_each() {
    let v = list(1, 2, 3, 4, 5);
    let mut count = 0;
    v.for_each(|x| count = count + 1);
    assert_eq(count, 5);
}

// === enumerate (returns Iterator) ===

#[test]
fn test_enumerate() {
    let v = list("a", "b", "c");
    let pairs = v.enumerate();
    let mut count = 0;
    for pair in pairs {
        count = count + 1;
    }
    assert_eq(count, 3);
}

// === zip (returns Iterator) ===

#[test]
fn test_zip() {
    let a = list(1, 2, 3);
    let b = list("a", "b", "c");
    let zipped = a.zip(b);
    let mut count = 0;
    for pair in zipped {
        count = count + 1;
    }
    assert_eq(count, 3);
}

// === take (returns Iterator) ===

#[test]
fn test_take() {
    let v = list(1, 2, 3, 4, 5);
    let first_two = v.take(2);
    let mut count = 0;
    for x in first_two {
        count = count + 1;
    }
    assert_eq(count, 2);
}

// === skip (returns Iterator) ===

#[test]
fn test_skip() {
    let v = list(1, 2, 3, 4, 5);
    let rest = v.skip(3);
    let mut count = 0;
    for x in rest {
        count = count + 1;
    }
    assert_eq(count, 2);
}

// === chain (returns Iterator) ===

#[test]
fn test_chain() {
    let a = list(1, 2);
    let b = list(3, 4);
    let combined = a.chain(b);
    let mut count = 0;
    for x in combined {
        count = count + 1;
    }
    assert_eq(count, 4);
}

// === rev / reverse (mutates in place) ===

#[test]
fn test_rev_iter() {
    let mut v = list(1, 2, 3);
    v.reverse();
    // Now iterate over the reversed vec
    let mut count = 0;
    for x in v {
        count = count + 1;
    }
    assert_eq(count, 3);
}
