// === Iterator Consumers ===
// fold, sum, product, count, any, all, find, position, for_each, next, nth

// --- fold ---

#[test]
fn test_fold_sum() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().fold(0, |acc, x| acc + x);
    assert::eq(result, 15);
}

#[test]
fn test_fold_product() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().fold(1, |acc, x| acc * x);
    assert::eq(result, 120);
}

#[test]
fn test_fold_max() {
    val v = [3, 1, 4, 1, 5, 9, 2, 6];
    val result = v.iter().fold(0, |acc, x| if x > acc { x } else { acc });
    assert::eq(result, 9);
}

#[test]
fn test_fold_string_concat() {
    val v = ["a", "b", "c"];
    val result = v.iter().fold("", |acc, x| {
        if acc == "" { x } else { acc + x }
    });
    assert::eq(result, "abc");
}

#[test]
fn test_fold_empty() {
    val v: List<Int> = [];
    val result = v.iter().fold(42, |acc, x| acc + x);
    assert::eq(result, 42);
}

#[test]
fn test_fold_count_positives() {
    val v = [-3, 1, -2, 4, 5, -1];
    val result = v.iter().fold(0, |acc, x| if x > 0 { acc + 1 } else { acc });
    assert::eq(result, 3);
}

// --- sum ---

#[test]
fn test_sum_basic() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().sum();
    assert::eq(result, 15);
}

#[test]
fn test_sum_empty() {
    val v: List<Int> = [];
    val result = v.iter().sum();
    assert::eq(result, 0);
}

#[test]
fn test_sum_single() {
    val v = [42];
    val result = v.iter().sum();
    assert::eq(result, 42);
}

#[test]
fn test_sum_negatives() {
    val v = [-1, -2, -3, 10];
    val result = v.iter().sum();
    assert::eq(result, 4);
}

#[test]
fn test_sum_after_map() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().map(|x| x * x).iter().sum();
    assert::eq(result, 55);  // 1+4+9+16+25
}

// --- product ---

#[test]
fn test_product_basic() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().product();
    assert::eq(result, 120);
}

#[test]
fn test_product_empty() {
    val v: List<Int> = [];
    val result = v.iter().product();
    assert::eq(result, 1);
}

#[test]
fn test_product_with_zero() {
    val v = [1, 2, 0, 4, 5];
    val result = v.iter().product();
    assert::eq(result, 0);
}

#[test]
fn test_product_single() {
    val v = [7];
    val result = v.iter().product();
    assert::eq(result, 7);
}

// --- count ---

#[test]
fn test_count_basic() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().count();
    assert::eq(result, 5);
}

#[test]
fn test_count_empty() {
    val v: List<Int> = [];
    val result = v.iter().count();
    assert::eq(result, 0);
}

#[test]
fn test_count_after_filter() {
    val v = [1, 2, 3, 4, 5, 6];
    val result = v.iter().filter(|x| x % 2 == 0).iter().count();
    assert::eq(result, 3);
}

#[test]
fn test_count_after_take() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().take(3).count();
    assert::eq(result, 3);
}

// --- any ---

#[test]
fn test_any_true() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().any(|x| x > 4);
    assert::true(result);
}

#[test]
fn test_any_false() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().any(|x| x > 10);
    assert::true(!result);
}

#[test]
fn test_any_empty() {
    val v: List<Int> = [];
    val result = v.iter().any(|x| x > 0);
    assert::true(!result);
}

#[test]
fn test_any_first_matches() {
    val v = [10, 1, 2, 3];
    val result = v.iter().any(|x| x > 5);
    assert::true(result);
}

#[test]
fn test_any_last_matches() {
    val v = [1, 2, 3, 10];
    val result = v.iter().any(|x| x > 5);
    assert::true(result);
}

// --- all ---

#[test]
fn test_all_true() {
    val v = [2, 4, 6, 8];
    val result = v.iter().all(|x| x % 2 == 0);
    assert::true(result);
}

#[test]
fn test_all_false() {
    val v = [2, 4, 5, 8];
    val result = v.iter().all(|x| x % 2 == 0);
    assert::true(!result);
}

#[test]
fn test_all_empty() {
    // vacuously true
    val v: List<Int> = [];
    val result = v.iter().all(|x| x > 100);
    assert::true(result);
}

#[test]
fn test_all_single_true() {
    val v = [4];
    val result = v.iter().all(|x| x % 2 == 0);
    assert::true(result);
}

#[test]
fn test_all_single_false() {
    val v = [3];
    val result = v.iter().all(|x| x % 2 == 0);
    assert::true(!result);
}

// --- find ---

#[test]
fn test_find_found() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().find(|x| x > 3);
    assert::eq(result.unwrap(), 4);
}

#[test]
fn test_find_not_found() {
    val v = [1, 2, 3];
    val result = v.iter().find(|x| x > 10);
    assert::true(result.is_none());
}

#[test]
fn test_find_first_match() {
    val v = [5, 10, 15, 20];
    val result = v.iter().find(|x| x % 10 == 0);
    assert::eq(result.unwrap(), 10);
}

#[test]
fn test_find_empty() {
    val v: List<Int> = [];
    val result = v.iter().find(|x| x > 0);
    assert::true(result.is_none());
}

#[test]
fn test_find_string() {
    val v = ["apple", "banana", "cherry"];
    val result = v.iter().find(|s| s.starts_with("b"));
    assert::eq(result.unwrap(), "banana");
}

// --- position ---

#[test]
fn test_position_found() {
    val v = [10, 20, 30, 40];
    val result = v.iter().position(|x| x == 30);
    assert::eq(result.unwrap(), 2);
}

#[test]
fn test_position_not_found() {
    val v = [1, 2, 3];
    val result = v.iter().position(|x| x == 99);
    assert::true(result.is_none());
}

#[test]
fn test_position_first_element() {
    val v = [42, 1, 2, 3];
    val result = v.iter().position(|x| x == 42);
    assert::eq(result.unwrap(), 0);
}

#[test]
fn test_position_last_element() {
    val v = [1, 2, 3, 42];
    val result = v.iter().position(|x| x == 42);
    assert::eq(result.unwrap(), 3);
}

#[test]
fn test_position_empty() {
    val v: List<Int> = [];
    val result = v.iter().position(|x| x > 0);
    assert::true(result.is_none());
}

// --- for_each ---

#[test]
fn test_for_each_side_effect() {
    val v = [1, 2, 3, 4, 5];
    var total = 0;
    v.iter().for_each(|x| { total = total + x; });
    assert::eq(total, 15);
}

#[test]
fn test_for_each_empty() {
    val v: List<Int> = [];
    var count = 0;
    v.iter().for_each(|_x| { count = count + 1; });
    assert::eq(count, 0);
}

#[test]
fn test_for_each_collect_into_list() {
    val v = [1, 2, 3];
    var out: List<Int> = [];
    v.iter().for_each(|x| { out.push(x * 2); });
    assert::eq(out, [2, 4, 6]);
}

// --- next ---
// NOTE: next() only works correctly when the iterator is consumed inline.
// Storing an iterator in a variable and calling next() repeatedly does NOT
// advance the stored state (Box<IteratorState> is value-typed, not Rc<RefCell>).
// Use nth(), for loops, or consumers (fold/sum/collect) for multi-step iteration.

#[test]
fn test_next_inline_first_element() {
    val v = [1, 2, 3];
    val result = v.iter().next();
    assert::eq(result.unwrap(), 1);
}

#[test]
fn test_next_empty() {
    val v: List<Int> = [];
    val result = v.iter().next();
    assert::true(result.is_none());
}

#[test]
fn test_next_after_skip() {
    val v = [1, 2, 3];
    val result = v.iter().skip(2).next();
    assert::eq(result.unwrap(), 3);
}

// --- nth ---

#[test]
fn test_nth_basic() {
    val v = [10, 20, 30, 40, 50];
    val result = v.iter().nth(2);
    assert::eq(result.unwrap(), 30);
}

#[test]
fn test_nth_first() {
    val v = [10, 20, 30];
    val result = v.iter().nth(0);
    assert::eq(result.unwrap(), 10);
}

#[test]
fn test_nth_out_of_bounds() {
    val v = [1, 2, 3];
    val result = v.iter().nth(5);
    assert::true(result.is_none());
}

#[test]
fn test_nth_empty() {
    val v: List<Int> = [];
    val result = v.iter().nth(0);
    assert::true(result.is_none());
}

// --- min / max ---

#[test]
fn test_min_basic() {
    val v = [3, 1, 4, 1, 5, 9, 2, 6];
    val result = v.iter().min();
    assert::eq(result.unwrap(), 1);
}

#[test]
fn test_max_basic() {
    val v = [3, 1, 4, 1, 5, 9, 2, 6];
    val result = v.iter().max();
    assert::eq(result.unwrap(), 9);
}

#[test]
fn test_min_empty() {
    val v: List<Int> = [];
    val result = v.iter().min();
    assert::true(result.is_none());
}

#[test]
fn test_max_empty() {
    val v: List<Int> = [];
    val result = v.iter().max();
    assert::true(result.is_none());
}

#[test]
fn test_min_single() {
    val v = [7];
    assert::eq(v.iter().min().unwrap(), 7);
}

#[test]
fn test_max_single() {
    val v = [7];
    assert::eq(v.iter().max().unwrap(), 7);
}

// --- AoC-style patterns ---

#[test]
fn test_aoc_count_lines_over_threshold() {
    val values = [199, 200, 208, 210, 200, 207, 240, 269, 260, 263];
    // AoC 2021 Day 1: count increases
    var increases = 0;
    var prev = values[0];
    for x in values.iter().skip(1).collect() {
        if x > prev {
            increases = increases + 1;
        }
        prev = x;
    }
    assert::eq(increases, 7);
}

#[test]
fn test_aoc_fold_to_find_first_duplicate_sum() {
    val v = [1, 2, 3, 4, 5];
    val total = v.iter().fold(0, |acc, x| acc + x);
    assert::eq(total, 15);
}

#[test]
fn test_aoc_any_all_validation() {
    val passwords = ["abc", "bcd", "aaa"];
    val any_start_a = passwords.iter().any(|p| p.starts_with("a"));
    val all_three_chars = passwords.iter().all(|p| p.len() == 3);
    assert::true(any_start_a);
    assert::true(all_three_chars);
}
