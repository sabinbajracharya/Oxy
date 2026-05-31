// === STRESS: iterators, adapters, consumers ===

// --- map / filter / collect ---
#[test]
fn test_map_squares() {
    let r: List<Int> = [1, 2, 3, 4].iter().map(|x| x * x).collect();
    assert_eq(r, [1, 4, 9, 16]);
}

#[test]
fn test_filter_evens() {
    let r: List<Int> = [1, 2, 3, 4, 5, 6].iter().filter(|x| x % 2 == 0).collect();
    assert_eq(r, [2, 4, 6]);
}

// --- chained map + filter ---
#[test]
fn test_map_filter_collect() {
    let r: List<Int> = [1, 2, 3, 4, 5, 6, 7, 8]
        .iter()
        .map(|x| x * 2)
        .filter(|x| x > 5)
        .collect();
    assert_eq(r, [6, 8, 10, 12, 14, 16]);
}

// --- fold / reduce ---
#[test]
fn test_fold_sum() {
    let s = [1, 2, 3, 4, 5].iter().fold(0, |a, b| a + b);
    assert_eq(s, 15);
}

#[test]
fn test_fold_product() {
    let p = [1, 2, 3, 4, 5].iter().fold(1, |a, b| a * b);
    assert_eq(p, 120);
}

#[test]
fn test_fold_string_concat() {
    let r = ["a".to_string(), "b".to_string(), "c".to_string()]
        .iter()
        .fold("".to_string(), |acc, x| format("{}{}", acc, x));
    assert_eq(r, "abc");
}

// --- sum / product ---
#[test]
fn test_iter_sum() {
    let s: Int = [1, 2, 3, 4, 5].iter().sum();
    assert_eq(s, 15);
}

#[test]
fn test_iter_product() {
    let p: Int = [1, 2, 3, 4].iter().product();
    assert_eq(p, 24);
}

// --- count / max / min ---
#[test]
fn test_iter_count() {
    let n = [10, 20, 30].iter().count();
    assert_eq(n, 3);
}

#[test]
fn test_iter_max() {
    let m = [3, 1, 4, 1, 5, 9, 2, 6].iter().max();
    assert_eq(m, Some(9));
}

#[test]
fn test_iter_min() {
    let m = [3, 1, 4, 1, 5, 9, 2, 6].iter().min();
    assert_eq(m, Some(1));
}

#[test]
fn test_iter_max_empty() {
    let v: List<Int> = [];
    let m = v.iter().max();
    assert_eq(m, None);
}

// --- enumerate ---
#[test]
fn test_iter_enumerate() {
    let mut acc = "".to_string();
    for (i, v) in ["a", "b", "c"].iter().enumerate() {
        acc = format("{}{}{}", acc, i, v);
    }
    assert_eq(acc, "0a1b2c");
}

// --- zip ---
#[test]
fn test_iter_zip() {
    let a = [1, 2, 3];
    let b = ["a", "b", "c"];
    let mut out = "".to_string();
    for (x, y) in a.iter().zip(b.iter()) {
        out = format("{}{}-{} ", out, x, y);
    }
    assert_eq(out, "1-a 2-b 3-c ");
}

// --- take / skip ---
#[test]
fn test_iter_take() {
    let r: List<Int> = [1, 2, 3, 4, 5].iter().take(3).collect();
    assert_eq(r, [1, 2, 3]);
}

#[test]
fn test_iter_skip() {
    let r: List<Int> = [1, 2, 3, 4, 5].iter().skip(2).collect();
    assert_eq(r, [3, 4, 5]);
}

// --- rev ---
#[test]
fn test_iter_rev() {
    let r: List<Int> = [1, 2, 3].iter().rev().collect();
    assert_eq(r, [3, 2, 1]);
}

// --- find / position ---
#[test]
fn test_iter_find() {
    let r = [1, 2, 3, 4].iter().find(|x| x > 2);
    assert_eq(r, Some(3));
}

#[test]
fn test_iter_find_none() {
    let r = [1, 2, 3].iter().find(|x| x > 99);
    assert_eq(r, None);
}

#[test]
fn test_iter_position() {
    let r = [10, 20, 30, 40].iter().position(|x| x == 30);
    assert_eq(r, Some(2));
}

// --- any / all ---
#[test]
fn test_iter_any() {
    assert_eq([1, 2, 3].iter().any(|x| x > 2), true);
    assert_eq([1, 2, 3].iter().any(|x| x > 99), false);
}

#[test]
fn test_iter_all() {
    assert_eq([1, 2, 3].iter().all(|x| x > 0), true);
    assert_eq([1, 2, 3].iter().all(|x| x > 2), false);
}

// --- range iteration ---
#[test]
fn test_range_collect() {
    let r: List<Int> = (0..5).collect();
    assert_eq(r, [0, 1, 2, 3, 4]);
}

#[test]
fn test_range_inclusive_collect() {
    let r: List<Int> = (0..=4).collect();
    assert_eq(r, [0, 1, 2, 3, 4]);
}

#[test]
fn test_range_sum() {
    let s: Int = (1..=100).sum();
    assert_eq(s, 5050);
}

// --- nested iterators ---
#[test]
fn test_iter_in_iter() {
    let v: List<List<Int>> = [[1, 2], [3, 4], [5, 6]];
    let total: Int = v.iter().fold(0, |acc, inner| acc + inner.iter().sum::<Int>());
    assert_eq(total, 21);
}
