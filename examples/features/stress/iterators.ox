// === STRESS: iterators, adapters, consumers ===

// --- map / filter / collect ---
#[test]
fn test_map_squares() {
    val r: List<Int> = [1, 2, 3, 4].iter().map(|x| x * x).collect();
    assert::eq(r, [1, 4, 9, 16]);
}

#[test]
fn test_filter_evens() {
    val r: List<Int> = [1, 2, 3, 4, 5, 6].iter().filter(|x| x % 2 == 0).collect();
    assert::eq(r, [2, 4, 6]);
}

// --- chained map + filter ---
#[test]
fn test_map_filter_collect() {
    val r: List<Int> = [1, 2, 3, 4, 5, 6, 7, 8]
        .iter()
        .map(|x| x * 2)
        .filter(|x| x > 5)
        .collect();
    assert::eq(r, [6, 8, 10, 12, 14, 16]);
}

// --- fold / reduce ---
#[test]
fn test_fold_sum() {
    val s = [1, 2, 3, 4, 5].iter().fold(0, |a, b| a + b);
    assert::eq(s, 15);
}

#[test]
fn test_fold_product() {
    val p = [1, 2, 3, 4, 5].iter().fold(1, |a, b| a * b);
    assert::eq(p, 120);
}

#[test]
fn test_fold_string_concat() {
    val r = ["a".to_string(), "b".to_string(), "c".to_string()]
        .iter()
        .fold("".to_string(), |acc, x| string::format("{}{}", acc, x));
    assert::eq(r, "abc");
}

// --- sum / product ---
#[test]
fn test_iter_sum() {
    val s: Int = [1, 2, 3, 4, 5].iter().sum();
    assert::eq(s, 15);
}

#[test]
fn test_iter_product() {
    val p: Int = [1, 2, 3, 4].iter().product();
    assert::eq(p, 24);
}

// --- count / max / min ---
#[test]
fn test_iter_count() {
    val n = [10, 20, 30].iter().count();
    assert::eq(n, 3);
}

#[test]
fn test_iter_max() {
    val m = [3, 1, 4, 1, 5, 9, 2, 6].iter().max();
    assert::eq(m, Some(9));
}

#[test]
fn test_iter_min() {
    val m = [3, 1, 4, 1, 5, 9, 2, 6].iter().min();
    assert::eq(m, Some(1));
}

#[test]
fn test_iter_max_empty() {
    val v: List<Int> = [];
    val m = v.iter().max();
    assert::eq(m, None);
}

// --- enumerate ---
#[test]
fn test_iter_enumerate() {
    var acc = "".to_string();
    for (i, v) in ["a", "b", "c"].iter().enumerate() {
        acc = string::format("{}{}{}", acc, i, v);
    }
    assert::eq(acc, "0a1b2c");
}

// --- zip ---
#[test]
fn test_iter_zip() {
    val a = [1, 2, 3];
    val b = ["a", "b", "c"];
    var out = "".to_string();
    for (x, y) in a.iter().zip(b.iter()) {
        out = string::format("{}{}-{} ", out, x, y);
    }
    assert::eq(out, "1-a 2-b 3-c ");
}

// --- take / skip ---
#[test]
fn test_iter_take() {
    val r: List<Int> = [1, 2, 3, 4, 5].iter().take(3).collect();
    assert::eq(r, [1, 2, 3]);
}

#[test]
fn test_iter_skip() {
    val r: List<Int> = [1, 2, 3, 4, 5].iter().skip(2).collect();
    assert::eq(r, [3, 4, 5]);
}

// --- rev ---
#[test]
fn test_iter_rev() {
    val r: List<Int> = [1, 2, 3].iter().rev().collect();
    assert::eq(r, [3, 2, 1]);
}

// --- find / position ---
#[test]
fn test_iter_find() {
    val r = [1, 2, 3, 4].iter().find(|x| x > 2);
    assert::eq(r, Some(3));
}

#[test]
fn test_iter_find_none() {
    val r = [1, 2, 3].iter().find(|x| x > 99);
    assert::eq(r, None);
}

#[test]
fn test_iter_position() {
    val r = [10, 20, 30, 40].iter().position(|x| x == 30);
    assert::eq(r, Some(2));
}

// --- any / all ---
#[test]
fn test_iter_any() {
    assert::eq([1, 2, 3].iter().any(|x| x > 2), true);
    assert::eq([1, 2, 3].iter().any(|x| x > 99), false);
}

#[test]
fn test_iter_all() {
    assert::eq([1, 2, 3].iter().all(|x| x > 0), true);
    assert::eq([1, 2, 3].iter().all(|x| x > 2), false);
}

// --- range iteration ---
#[test]
fn test_range_collect() {
    val r: List<Int> = (0..5).collect();
    assert::eq(r, [0, 1, 2, 3, 4]);
}

#[test]
fn test_range_inclusive_collect() {
    val r: List<Int> = (0..=4).collect();
    assert::eq(r, [0, 1, 2, 3, 4]);
}

#[test]
fn test_range_sum() {
    val s: Int = (1..=100).sum();
    assert::eq(s, 5050);
}

// --- nested iterators ---
#[test]
fn test_iter_in_iter() {
    val v: List<List<Int>> = [[1, 2], [3, 4], [5, 6]];
    val total: Int = v.iter().fold(0, |acc, inner| acc + inner.iter().sum::<Int>());
    assert::eq(total, 21);
}
