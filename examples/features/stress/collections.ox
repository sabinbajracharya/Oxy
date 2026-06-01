// === STRESS: collections — List, Map, Set edge cases ===

use std::collections::Map;
use std::collections::Set;

// === List ===

#[test]
fn test_vec_empty() {
    val v: List<Int> = [];
    assert::eq(v.len(), 0);
    assert::eq(v.is_empty(), true);
}

#[test]
fn test_vec_singleton() {
    val v = [42];
    assert::eq(v.len(), 1);
    assert::eq(v[0], 42);
}

#[test]
fn test_vec_push_pop() {
    var v: List<Int> = [];
    v.push(1); v.push(2); v.push(3);
    assert::eq(v.len(), 3);
    assert::eq(v.pop(), Some(3));
    assert::eq(v.pop(), Some(2));
    assert::eq(v.pop(), Some(1));
    assert::eq(v.pop(), None);
}

#[test]
fn test_vec_indexing() {
    val v = [10, 20, 30];
    assert::eq(v[0], 10);
    assert::eq(v[2], 30);
}

#[test]
fn test_vec_iter_sum() {
    val v = [1, 2, 3, 4, 5];
    val s: Int = v.iter().sum();
    assert::eq(s, 15);
}

#[test]
fn test_vec_iter_map() {
    val v = [1, 2, 3];
    val r: List<Int> = v.iter().map(|x| x * 10).collect();
    assert::eq(r, [10, 20, 30]);
}

#[test]
fn test_vec_iter_filter() {
    val v = [1, 2, 3, 4, 5];
    val r: List<Int> = v.iter().filter(|x| x % 2 == 0).collect();
    assert::eq(r, [2, 4]);
}

#[test]
fn test_vec_contains() {
    val v = [1, 2, 3];
    assert::eq(v.contains(2), true);
    assert::eq(v.contains(99), false);
}

#[test]
fn test_vec_reverse_in_place() {
    var v = [1, 2, 3];
    v.reverse();
    assert::eq(v, [3, 2, 1]);
}

#[test]
fn test_vec_sort_ints() {
    var v = [3, 1, 4, 1, 5, 9, 2, 6, 5];
    v.sort();
    assert::eq(v, [1, 1, 2, 3, 4, 5, 5, 6, 9]);
}

#[test]
fn test_vec_nested() {
    val v: List<List<Int>> = [[1, 2], [3, 4], [5, 6]];
    assert::eq(v[1][0], 3);
    assert::eq(v.len(), 3);
}

#[test]
fn test_vec_large() {
    var v: List<Int> = [];
    var i = 0;
    while i < 1000 {
        v.push(i);
        i = i + 1;
    }
    assert::eq(v.len(), 1000);
    assert::eq(v[500], 500);
}

#[test]
fn test_vec_first_last() {
    val v = [10, 20, 30];
    assert::eq(v.first(), Some(10));
    assert::eq(v.last(), Some(30));
}

#[test]
fn test_vec_first_last_empty() {
    val v: List<Int> = [];
    assert::eq(v.first(), None);
    assert::eq(v.last(), None);
}

#[test]
fn test_vec_iteration_order() {
    val v = [1, 2, 3];
    var acc = "".to_string();
    for x in v {
        acc = string::format("{}{}", acc, x);
    }
    assert::eq(acc, "123");
}

// === Map ===

#[test]
fn test_hashmap_empty() {
    val m: Map<String, Int> = Map::new();
    assert::eq(m.len(), 0);
}

#[test]
fn test_hashmap_insert_get() {
    var m: Map<String, Int> = Map::new();
    m.insert("a".to_string(), 1);
    m.insert("b".to_string(), 2);
    assert::eq(m.get("a"), Some(1));
    assert::eq(m.get("b"), Some(2));
    assert::eq(m.get("c"), None);
}

#[test]
fn test_hashmap_overwrite() {
    var m: Map<String, Int> = Map::new();
    m.insert("k".to_string(), 1);
    m.insert("k".to_string(), 2);
    assert::eq(m.get("k"), Some(2));
    assert::eq(m.len(), 1);
}

#[test]
fn test_hashmap_remove() {
    var m: Map<String, Int> = Map::new();
    m.insert("k".to_string(), 5);
    val removed = m.remove("k");
    assert::eq(removed, Some(5));
    assert::eq(m.get("k"), None);
}

#[test]
fn test_hashmap_contains_key() {
    var m: Map<String, Int> = Map::new();
    m.insert("k".to_string(), 0);
    assert::eq(m.contains_key("k"), true);
    assert::eq(m.contains_key("nope"), false);
}

#[test]
fn test_hashmap_iteration_count() {
    var m: Map<String, Int> = Map::new();
    m.insert("a".to_string(), 1);
    m.insert("b".to_string(), 2);
    m.insert("c".to_string(), 3);
    var total = 0;
    for (_k, v) in m {
        total = total + v;
    }
    assert::eq(total, 6);
}

// === Set ===

#[test]
fn test_hashset_empty() {
    val s: Set<Int> = Set::new();
    assert::eq(s.len(), 0);
}

#[test]
fn test_hashset_insert_contains() {
    var s: Set<Int> = Set::new();
    s.insert(1);
    s.insert(2);
    assert::eq(s.contains(1), true);
    assert::eq(s.contains(99), false);
}

#[test]
fn test_hashset_dedup() {
    var s: Set<Int> = Set::new();
    s.insert(1);
    s.insert(1);
    s.insert(1);
    assert::eq(s.len(), 1);
}

#[test]
fn test_hashset_remove() {
    var s: Set<Int> = Set::new();
    s.insert(5);
    val removed = s.remove(5);
    assert::eq(removed, true);
    assert::eq(s.contains(5), false);
}

// === Fixed-size arrays ===

#[test]
fn test_array_literal() {
    val a = [1, 2, 3, 4, 5];
    assert::eq(a[0], 1);
    assert::eq(a[4], 5);
}

#[test]
fn test_array_for_in() {
    val a = [10, 20, 30];
    var sum = 0;
    for x in a {
        sum = sum + x;
    }
    assert::eq(sum, 60);
}

#[test]
fn test_array_typed() {
    val a: [Int; 3] = [7, 8, 9];
    assert::eq(a[1], 8);
}
