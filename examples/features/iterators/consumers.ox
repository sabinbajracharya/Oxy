// === Iterator Consumers ===
// fold, sum, product, count, any, all, find, position, for_each, next, nth

// --- fold ---

#[test]
fn test_fold_sum() {
    let v = list(1, 2, 3, 4, 5);
    let result = v.iter().fold(0, |acc, x| acc + x);
    assert_eq(result, 15);
}

#[test]
fn test_fold_product() {
    let v = list(1, 2, 3, 4, 5);
    let result = v.iter().fold(1, |acc, x| acc * x);
    assert_eq(result, 120);
}

#[test]
fn test_fold_max() {
    let v = list(3, 1, 4, 1, 5, 9, 2, 6);
    let result = v.iter().fold(0, |acc, x| if x > acc { x } else { acc });
    assert_eq(result, 9);
}

#[test]
fn test_fold_string_concat() {
    let v = list("a", "b", "c");
    let result = v.iter().fold("", |acc, x| {
        if acc == "" { x } else { acc + x }
    });
    assert_eq(result, "abc");
}

#[test]
fn test_fold_empty() {
    let v: List<Int> = list();
    let result = v.iter().fold(42, |acc, x| acc + x);
    assert_eq(result, 42);
}

#[test]
fn test_fold_count_positives() {
    let v = list(-3, 1, -2, 4, 5, -1);
    let result = v.iter().fold(0, |acc, x| if x > 0 { acc + 1 } else { acc });
    assert_eq(result, 3);
}

// --- sum ---

#[test]
fn test_sum_basic() {
    let v = list(1, 2, 3, 4, 5);
    let result = v.iter().sum();
    assert_eq(result, 15);
}

#[test]
fn test_sum_empty() {
    let v: List<Int> = list();
    let result = v.iter().sum();
    assert_eq(result, 0);
}

#[test]
fn test_sum_single() {
    let v = list(42);
    let result = v.iter().sum();
    assert_eq(result, 42);
}

#[test]
fn test_sum_negatives() {
    let v = list(-1, -2, -3, 10);
    let result = v.iter().sum();
    assert_eq(result, 4);
}

#[test]
fn test_sum_after_map() {
    let v = list(1, 2, 3, 4, 5);
    let result = v.iter().map(|x| x * x).iter().sum();
    assert_eq(result, 55);  // 1+4+9+16+25
}

// --- product ---

#[test]
fn test_product_basic() {
    let v = list(1, 2, 3, 4, 5);
    let result = v.iter().product();
    assert_eq(result, 120);
}

#[test]
fn test_product_empty() {
    let v: List<Int> = list();
    let result = v.iter().product();
    assert_eq(result, 1);
}

#[test]
fn test_product_with_zero() {
    let v = list(1, 2, 0, 4, 5);
    let result = v.iter().product();
    assert_eq(result, 0);
}

#[test]
fn test_product_single() {
    let v = list(7);
    let result = v.iter().product();
    assert_eq(result, 7);
}

// --- count ---

#[test]
fn test_count_basic() {
    let v = list(1, 2, 3, 4, 5);
    let result = v.iter().count();
    assert_eq(result, 5);
}

#[test]
fn test_count_empty() {
    let v: List<Int> = list();
    let result = v.iter().count();
    assert_eq(result, 0);
}

#[test]
fn test_count_after_filter() {
    let v = list(1, 2, 3, 4, 5, 6);
    let result = v.iter().filter(|x| x % 2 == 0).iter().count();
    assert_eq(result, 3);
}

#[test]
fn test_count_after_take() {
    let v = list(1, 2, 3, 4, 5);
    let result = v.iter().take(3).count();
    assert_eq(result, 3);
}

// --- any ---

#[test]
fn test_any_true() {
    let v = list(1, 2, 3, 4, 5);
    let result = v.iter().any(|x| x > 4);
    assert(result);
}

#[test]
fn test_any_false() {
    let v = list(1, 2, 3, 4, 5);
    let result = v.iter().any(|x| x > 10);
    assert(!result);
}

#[test]
fn test_any_empty() {
    let v: List<Int> = list();
    let result = v.iter().any(|x| x > 0);
    assert(!result);
}

#[test]
fn test_any_first_matches() {
    let v = list(10, 1, 2, 3);
    let result = v.iter().any(|x| x > 5);
    assert(result);
}

#[test]
fn test_any_last_matches() {
    let v = list(1, 2, 3, 10);
    let result = v.iter().any(|x| x > 5);
    assert(result);
}

// --- all ---

#[test]
fn test_all_true() {
    let v = list(2, 4, 6, 8);
    let result = v.iter().all(|x| x % 2 == 0);
    assert(result);
}

#[test]
fn test_all_false() {
    let v = list(2, 4, 5, 8);
    let result = v.iter().all(|x| x % 2 == 0);
    assert(!result);
}

#[test]
fn test_all_empty() {
    // vacuously true
    let v: List<Int> = list();
    let result = v.iter().all(|x| x > 100);
    assert(result);
}

#[test]
fn test_all_single_true() {
    let v = list(4);
    let result = v.iter().all(|x| x % 2 == 0);
    assert(result);
}

#[test]
fn test_all_single_false() {
    let v = list(3);
    let result = v.iter().all(|x| x % 2 == 0);
    assert(!result);
}

// --- find ---

#[test]
fn test_find_found() {
    let v = list(1, 2, 3, 4, 5);
    let result = v.iter().find(|x| x > 3);
    assert_eq(result.unwrap(), 4);
}

#[test]
fn test_find_not_found() {
    let v = list(1, 2, 3);
    let result = v.iter().find(|x| x > 10);
    assert(result.is_none());
}

#[test]
fn test_find_first_match() {
    let v = list(5, 10, 15, 20);
    let result = v.iter().find(|x| x % 10 == 0);
    assert_eq(result.unwrap(), 10);
}

#[test]
fn test_find_empty() {
    let v: List<Int> = list();
    let result = v.iter().find(|x| x > 0);
    assert(result.is_none());
}

#[test]
fn test_find_string() {
    let v = list("apple", "banana", "cherry");
    let result = v.iter().find(|s| s.starts_with("b"));
    assert_eq(result.unwrap(), "banana");
}

// --- position ---

#[test]
fn test_position_found() {
    let v = list(10, 20, 30, 40);
    let result = v.iter().position(|x| x == 30);
    assert_eq(result.unwrap(), 2);
}

#[test]
fn test_position_not_found() {
    let v = list(1, 2, 3);
    let result = v.iter().position(|x| x == 99);
    assert(result.is_none());
}

#[test]
fn test_position_first_element() {
    let v = list(42, 1, 2, 3);
    let result = v.iter().position(|x| x == 42);
    assert_eq(result.unwrap(), 0);
}

#[test]
fn test_position_last_element() {
    let v = list(1, 2, 3, 42);
    let result = v.iter().position(|x| x == 42);
    assert_eq(result.unwrap(), 3);
}

#[test]
fn test_position_empty() {
    let v: List<Int> = list();
    let result = v.iter().position(|x| x > 0);
    assert(result.is_none());
}

// --- for_each ---

#[test]
fn test_for_each_side_effect() {
    let v = list(1, 2, 3, 4, 5);
    let mut total = 0;
    v.iter().for_each(|x| { total = total + x; });
    assert_eq(total, 15);
}

#[test]
fn test_for_each_empty() {
    let v: List<Int> = list();
    let mut count = 0;
    v.iter().for_each(|_x| { count = count + 1; });
    assert_eq(count, 0);
}

#[test]
fn test_for_each_collect_into_list() {
    let v = list(1, 2, 3);
    let mut out: List<Int> = list();
    v.iter().for_each(|x| { out.push(x * 2); });
    assert_eq(out, list(2, 4, 6));
}

// --- next ---
// NOTE: next() only works correctly when the iterator is consumed inline.
// Storing an iterator in a variable and calling next() repeatedly does NOT
// advance the stored state (Box<IteratorState> is value-typed, not Rc<RefCell>).
// Use nth(), for loops, or consumers (fold/sum/collect) for multi-step iteration.

#[test]
fn test_next_inline_first_element() {
    let v = list(1, 2, 3);
    let result = v.iter().next();
    assert_eq(result.unwrap(), 1);
}

#[test]
fn test_next_empty() {
    let v: List<Int> = list();
    let result = v.iter().next();
    assert(result.is_none());
}

#[test]
fn test_next_after_skip() {
    let v = list(1, 2, 3);
    let result = v.iter().skip(2).next();
    assert_eq(result.unwrap(), 3);
}

// --- nth ---

#[test]
fn test_nth_basic() {
    let v = list(10, 20, 30, 40, 50);
    let result = v.iter().nth(2);
    assert_eq(result.unwrap(), 30);
}

#[test]
fn test_nth_first() {
    let v = list(10, 20, 30);
    let result = v.iter().nth(0);
    assert_eq(result.unwrap(), 10);
}

#[test]
fn test_nth_out_of_bounds() {
    let v = list(1, 2, 3);
    let result = v.iter().nth(5);
    assert(result.is_none());
}

#[test]
fn test_nth_empty() {
    let v: List<Int> = list();
    let result = v.iter().nth(0);
    assert(result.is_none());
}

// --- min / max ---

#[test]
fn test_min_basic() {
    let v = list(3, 1, 4, 1, 5, 9, 2, 6);
    let result = v.iter().min();
    assert_eq(result.unwrap(), 1);
}

#[test]
fn test_max_basic() {
    let v = list(3, 1, 4, 1, 5, 9, 2, 6);
    let result = v.iter().max();
    assert_eq(result.unwrap(), 9);
}

#[test]
fn test_min_empty() {
    let v: List<Int> = list();
    let result = v.iter().min();
    assert(result.is_none());
}

#[test]
fn test_max_empty() {
    let v: List<Int> = list();
    let result = v.iter().max();
    assert(result.is_none());
}

#[test]
fn test_min_single() {
    let v = list(7);
    assert_eq(v.iter().min().unwrap(), 7);
}

#[test]
fn test_max_single() {
    let v = list(7);
    assert_eq(v.iter().max().unwrap(), 7);
}

// --- AoC-style patterns ---

#[test]
fn test_aoc_count_lines_over_threshold() {
    let values = list(199, 200, 208, 210, 200, 207, 240, 269, 260, 263);
    // AoC 2021 Day 1: count increases
    let mut increases = 0;
    let mut prev = values[0];
    for x in values.iter().skip(1).collect() {
        if x > prev {
            increases = increases + 1;
        }
        prev = x;
    }
    assert_eq(increases, 7);
}

#[test]
fn test_aoc_fold_to_find_first_duplicate_sum() {
    let v = list(1, 2, 3, 4, 5);
    let total = v.iter().fold(0, |acc, x| acc + x);
    assert_eq(total, 15);
}

#[test]
fn test_aoc_any_all_validation() {
    let passwords = list("abc", "bcd", "aaa");
    let any_start_a = passwords.iter().any(|p| p.starts_with("a"));
    let all_three_chars = passwords.iter().all(|p| p.len() == 3);
    assert(any_start_a);
    assert(all_three_chars);
}
