// === Feature: Collections — List Iterators ===
// List supports iterator methods: map, filter, fold, any, all, find,
// position, sum, count, for_each, enumerate, zip, chain, take, skip,
// rev. Methods that return iterators (enumerate, zip, take, skip, chain,
// rev) don't have .len() — iterate over them instead.

// === map ===

#[test]
fn test_map() {
    val v = [1, 2, 3];
    val doubled = v.map(|x| x * 2);
    assert::eq(doubled.len(), 3);
}

// === filter ===

#[test]
fn test_filter() {
    val v = [1, 2, 3, 4, 5, 6];
    val evens = v.filter(|x| x % 2 == 0);
    assert::eq(evens.len(), 3);
}

// === fold ===

#[test]
fn test_fold_sum() {
    val v = [1, 2, 3, 4];
    val result = v.fold(0, |acc, x| acc + x);
    assert::eq(result, 10);
}

#[test]
fn test_fold_multiply() {
    val v = [1, 2, 3, 4];
    val result = v.fold(1, |acc, x| acc * x);
    assert::eq(result, 24);
}

// === any / all ===

#[test]
fn test_any_true() {
    val v = [1, 3, 5, 8];
    assert::true(v.any(|x| x % 2 == 0));
}

#[test]
fn test_any_false() {
    val v = [1, 3, 5, 7];
    assert::true(!v.any(|x| x % 2 == 0));
}

#[test]
fn test_all_true() {
    val v = [2, 4, 6, 8];
    assert::true(v.all(|x| x % 2 == 0));
}

#[test]
fn test_all_false() {
    val v = [2, 3, 4];
    assert::true(!v.all(|x| x % 2 == 0));
}

// === find ===

#[test]
fn test_find_some() {
    val v = [1, 3, 5, 8, 9];
    val r = v.find(|x| x % 2 == 0);
    assert::true(r.is_some());
}

#[test]
fn test_find_none() {
    val v = [1, 3, 5, 7];
    val r = v.find(|x| x % 2 == 0);
    assert::true(r.is_none());
}

// === position ===

#[test]
fn test_position_found() {
    val v = ["a", "b", "c", "d"];
    val r = v.position(|x| x == "c");
    assert::true(r.is_some());
}

#[test]
fn test_position_not_found() {
    val v = ["a", "b", "c"];
    val r = v.position(|x| x == "z");
    assert::true(r.is_none());
}

// === sum / count ===

#[test]
fn test_sum() {
    val v = [1, 2, 3, 4, 5];
    assert::eq(v.sum(), 15);
}

#[test]
fn test_count() {
    val v = [1, 2, 3, 4, 5];
    assert::eq(v.count(), 5);
}

// === for_each ===

#[test]
fn test_for_each() {
    val v = [1, 2, 3, 4, 5];
    var count = 0;
    v.for_each(|x| count = count + 1);
    assert::eq(count, 5);
}

// === enumerate (returns Iterator) ===

#[test]
fn test_enumerate() {
    val v = ["a", "b", "c"];
    val pairs = v.enumerate();
    var count = 0;
    for pair in pairs {
        count = count + 1;
    }
    assert::eq(count, 3);
}

// === zip (returns Iterator) ===

#[test]
fn test_zip() {
    val a = [1, 2, 3];
    val b = ["a", "b", "c"];
    val zipped = a.zip(b);
    var count = 0;
    for pair in zipped {
        count = count + 1;
    }
    assert::eq(count, 3);
}

// === take (returns Iterator) ===

#[test]
fn test_take() {
    val v = [1, 2, 3, 4, 5];
    val first_two = v.take(2);
    var count = 0;
    for x in first_two {
        count = count + 1;
    }
    assert::eq(count, 2);
}

// === skip (returns Iterator) ===

#[test]
fn test_skip() {
    val v = [1, 2, 3, 4, 5];
    val rest = v.skip(3);
    var count = 0;
    for x in rest {
        count = count + 1;
    }
    assert::eq(count, 2);
}

// === chain (returns Iterator) ===

#[test]
fn test_chain() {
    val a = [1, 2];
    val b = [3, 4];
    val combined = a.chain(b);
    var count = 0;
    for x in combined {
        count = count + 1;
    }
    assert::eq(count, 4);
}

// === rev / reverse (mutates in place) ===

#[test]
fn test_rev_iter() {
    var v = [1, 2, 3];
    v.reverse();
    // Now iterate over the reversed vec
    var count = 0;
    for x in v {
        count = count + 1;
    }
    assert::eq(count, 3);
}
