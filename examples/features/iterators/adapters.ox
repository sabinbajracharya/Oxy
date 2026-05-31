// === Iterator Adapters ===
// map, filter, take, skip, rev, chain, enumerate, zip, flat_map, flatten, collect

// --- map ---

#[test]
fn test_map_double() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().map(|x| x * 2);
    assert_eq(result, [2, 4, 6, 8, 10]);
}

#[test]
fn test_map_to_string() {
    val v = [1, 2, 3];
    val result = v.iter().map(|x| x.to_string());
    assert_eq(result[0], "1");
    assert_eq(result[1], "2");
    assert_eq(result[2], "3");
}

#[test]
fn test_map_empty() {
    val v: List<Int> = [];
    val result = v.iter().map(|x| x * 2);
    assert_eq(result.len(), 0);
}

#[test]
fn test_map_strings() {
    val v = ["hello", "world"];
    val result = v.iter().map(|s| s.to_uppercase());
    assert_eq(result[0], "HELLO");
    assert_eq(result[1], "WORLD");
}

// --- filter ---

#[test]
fn test_filter_even() {
    val v = [1, 2, 3, 4, 5, 6];
    val result = v.iter().filter(|x| x % 2 == 0);
    assert_eq(result, [2, 4, 6]);
}

#[test]
fn test_filter_none_match() {
    val v = [1, 3, 5];
    val result = v.iter().filter(|x| x % 2 == 0);
    assert_eq(result.len(), 0);
}

#[test]
fn test_filter_all_match() {
    val v = [2, 4, 6];
    val result = v.iter().filter(|x| x % 2 == 0);
    assert_eq(result.len(), 3);
}

#[test]
fn test_filter_empty() {
    val v: List<Int> = [];
    val result = v.iter().filter(|x| x > 0);
    assert_eq(result.len(), 0);
}

#[test]
fn test_filter_strings() {
    val v = ["foo", "bar", "baz", "qux"];
    val result = v.iter().filter(|s| s.starts_with("b"));
    assert_eq(result.len(), 2);
    assert_eq(result[0], "bar");
    assert_eq(result[1], "baz");
}

// --- map + filter chaining ---

#[test]
fn test_filter_then_map() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().filter(|x| x % 2 == 0).iter().map(|x| x * x);
    assert_eq(result, [4, 16]);
}

#[test]
fn test_map_then_filter() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().map(|x| x * 2).iter().filter(|x| x > 4);
    assert_eq(result, [6, 8, 10]);
}

// --- take ---

#[test]
fn test_take_basic() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().take(3).collect();
    assert_eq(result, [1, 2, 3]);
}

#[test]
fn test_take_more_than_len() {
    val v = [1, 2, 3];
    val result = v.iter().take(10).collect();
    assert_eq(result, [1, 2, 3]);
}

#[test]
fn test_take_zero() {
    val v = [1, 2, 3];
    val result = v.iter().take(0).collect();
    assert_eq(result.len(), 0);
}

#[test]
fn test_take_exact_len() {
    val v = [1, 2, 3];
    val result = v.iter().take(3).collect();
    assert_eq(result, [1, 2, 3]);
}

// --- skip ---

#[test]
fn test_skip_basic() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().skip(2).collect();
    assert_eq(result, [3, 4, 5]);
}

#[test]
fn test_skip_all() {
    val v = [1, 2, 3];
    val result = v.iter().skip(10).collect();
    assert_eq(result.len(), 0);
}

#[test]
fn test_skip_zero() {
    val v = [1, 2, 3];
    val result = v.iter().skip(0).collect();
    assert_eq(result, [1, 2, 3]);
}

#[test]
fn test_skip_exact_len() {
    val v = [1, 2, 3];
    val result = v.iter().skip(3).collect();
    assert_eq(result.len(), 0);
}

// --- take + skip ---

#[test]
fn test_skip_then_take() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().skip(1).take(3).collect();
    assert_eq(result, [2, 3, 4]);
}

#[test]
fn test_take_then_skip() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().take(4).skip(1).collect();
    assert_eq(result, [2, 3, 4]);
}

// --- rev ---

#[test]
fn test_rev_basic() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().rev();
    assert_eq(result, [5, 4, 3, 2, 1]);
}

#[test]
fn test_rev_single() {
    val v = [42];
    val result = v.iter().rev();
    assert_eq(result, [42]);
}

#[test]
fn test_rev_empty() {
    val v: List<Int> = [];
    val result = v.iter().rev();
    assert_eq(result.len(), 0);
}

#[test]
fn test_rev_strings() {
    val v = ["a", "b", "c"];
    val result = v.iter().rev();
    assert_eq(result[0], "c");
    assert_eq(result[1], "b");
    assert_eq(result[2], "a");
}

// --- chain ---

#[test]
fn test_chain_two_vecs() {
    val a = [1, 2, 3];
    val b = [4, 5, 6];
    val result = a.iter().chain(b.iter()).collect();
    assert_eq(result, [1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_chain_empty_first() {
    val a: List<Int> = [];
    val b = [1, 2, 3];
    val result = a.iter().chain(b.iter()).collect();
    assert_eq(result, [1, 2, 3]);
}

#[test]
fn test_chain_empty_second() {
    val a = [1, 2, 3];
    val b: List<Int> = [];
    val result = a.iter().chain(b.iter()).collect();
    assert_eq(result, [1, 2, 3]);
}

#[test]
fn test_chain_both_empty() {
    val a: List<Int> = [];
    val b: List<Int> = [];
    val result = a.iter().chain(b.iter()).collect();
    assert_eq(result.len(), 0);
}

#[test]
fn test_chain_then_map() {
    val a = [1, 2];
    val b = [3, 4];
    val result = a.iter().chain(b.iter()).collect().iter().map(|x| x * 10);
    assert_eq(result, [10, 20, 30, 40]);
}

// --- enumerate ---

#[test]
fn test_enumerate_basic() {
    val v = [10, 20, 30];
    val pairs = v.iter().enumerate().collect();
    assert_eq(pairs.len(), 3);
    val (i0, v0) = pairs[0];
    val (i1, v1) = pairs[1];
    val (i2, v2) = pairs[2];
    assert_eq(i0, 0);
    assert_eq(i1, 1);
    assert_eq(i2, 2);
    assert_eq(v0, 10);
    assert_eq(v1, 20);
    assert_eq(v2, 30);
}

#[test]
fn test_enumerate_strings() {
    val v = ["a", "b", "c"];
    val pairs = v.iter().enumerate().collect();
    val (i, s) = pairs[0];
    assert_eq(i, 0);
    assert_eq(s, "a");
}

#[test]
fn test_enumerate_empty() {
    val v: List<Int> = [];
    val pairs = v.iter().enumerate().collect();
    assert_eq(pairs.len(), 0);
}

#[test]
fn test_enumerate_index_in_loop() {
    val v = [100, 200, 300];
    var sum_indices = 0;
    var sum_values = 0;
    for (i, x) in v.iter().enumerate().collect() {
        sum_indices = sum_indices + i;
        sum_values = sum_values + x;
    }
    assert_eq(sum_indices, 3);   // 0+1+2
    assert_eq(sum_values, 600);  // 100+200+300
}

// --- zip ---

#[test]
fn test_zip_basic() {
    val a = [1, 2, 3];
    val b = [10, 20, 30];
    val pairs = a.iter().zip(b.iter()).collect();
    assert_eq(pairs.len(), 3);
    val (a0, b0) = pairs[0];
    assert_eq(a0, 1);
    assert_eq(b0, 10);
}

#[test]
fn test_zip_mixed_types() {
    val nums = [1, 2, 3];
    val strs = ["a", "b", "c"];
    val pairs = nums.iter().zip(strs.iter()).collect();
    val (n, s) = pairs[1];
    assert_eq(n, 2);
    assert_eq(s, "b");
}

#[test]
fn test_zip_stops_at_shorter() {
    val a = [1, 2, 3, 4, 5];
    val b = [10, 20];
    val pairs = a.iter().zip(b.iter()).collect();
    assert_eq(pairs.len(), 2);
}

#[test]
fn test_zip_empty() {
    val a: List<Int> = [];
    val b = [1, 2, 3];
    val pairs = a.iter().zip(b.iter()).collect();
    assert_eq(pairs.len(), 0);
}

#[test]
fn test_zip_sum_of_products() {
    val a = [1, 2, 3];
    val b = [4, 5, 6];
    val pairs = a.iter().zip(b.iter()).collect();
    var total = 0;
    for (x, y) in pairs {
        total = total + x * y;
    }
    assert_eq(total, 32);  // 1*4 + 2*5 + 3*6 = 4+10+18
}

// --- flat_map ---

#[test]
fn test_flat_map_expand() {
    val v = [1, 2, 3];
    val result = v.iter().flat_map(|x| [x, x * 10]).collect();
    assert_eq(result, [1, 10, 2, 20, 3, 30]);
}

#[test]
fn test_flat_map_filter_like() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().flat_map(|x| if x % 2 == 0 { [x] } else { [] }).collect();
    assert_eq(result, [2, 4]);
}

#[test]
fn test_flat_map_empty_input() {
    val v: List<Int> = [];
    val result = v.iter().flat_map(|x| [x, x + 1]).collect();
    assert_eq(result.len(), 0);
}

#[test]
fn test_flat_map_all_empty() {
    val v = [1, 2, 3];
    val result = v.iter().flat_map(|_x| []).collect();
    assert_eq(result.len(), 0);
}

// --- flatten ---

#[test]
fn test_flatten_basic() {
    val v = [[1, 2], [3, 4], [5, 6]];
    val result = v.iter().flatten();
    assert_eq(result, [1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_flatten_with_empty_inner() {
    val v = [[1, 2], [], [3, 4]];
    val result = v.iter().flatten();
    assert_eq(result, [1, 2, 3, 4]);
}

#[test]
fn test_flatten_all_empty() {
    val v: List<List<Int>> = [[], [], []];
    val result = v.iter().flatten();
    assert_eq(result.len(), 0);
}

#[test]
fn test_flatten_single_inner() {
    val v = [[42]];
    val result = v.iter().flatten();
    assert_eq(result, [42]);
}

// --- collect ---

#[test]
fn test_collect_roundtrip() {
    val v = [1, 2, 3];
    val v2 = v.iter().collect();
    assert_eq(v2, [1, 2, 3]);
}

#[test]
fn test_collect_after_take() {
    val v = [1, 2, 3, 4, 5];
    val result = v.iter().take(2).collect();
    assert_eq(result.len(), 2);
    assert_eq(result[0], 1);
    assert_eq(result[1], 2);
}

// --- AoC-style patterns ---

#[test]
fn test_aoc_parse_line_of_numbers() {
    val line = "3 1 4 1 5 9 2 6";
    val nums = line.split_whitespace().iter().map(|s| s.parse_int().unwrap());
    assert_eq(nums.len(), 8);
    assert_eq(nums[0], 3);
    assert_eq(nums[7], 6);
}

#[test]
fn test_aoc_sum_of_doubled_evens() {
    val v = [1, 2, 3, 4, 5, 6, 7, 8];
    val result = v.iter().filter(|x| x % 2 == 0).iter().map(|x| x * 2).iter().fold(0, |acc, x| acc + x);
    assert_eq(result, 40);  // (2+4+6+8)*2 = 40
}

#[test]
fn test_aoc_index_of_first_over_threshold() {
    val v = [10, 20, 30, 40, 50];
    val pairs = v.iter().enumerate().collect();
    var found_idx = -1;
    for (i, x) in pairs {
        if x > 25 && found_idx == -1 {
            found_idx = i;
        }
    }
    assert_eq(found_idx, 2);
}

// --- Shared state through lazy adapters ---
// A lazy adapter (take/skip/chain/zip/enumerate) shares state with its source
// iterator: advancing the adapter advances the underlying iterator too.

#[test]
fn test_take_shares_source_state() {
    val v = [1, 2, 3, 4, 5];
    val it = v.iter();
    val t = it.take(2);
    assert_eq(t.next().unwrap(), 1);
    assert_eq(t.next().unwrap(), 2);
    assert(t.next().is_none());
    // Source advanced through `t`, so `it` resumes at element 3.
    assert_eq(it.next().unwrap(), 3);
    assert_eq(it.next().unwrap(), 4);
}

#[test]
fn test_skip_shares_source_state() {
    val v = [1, 2, 3, 4, 5];
    val it = v.iter();
    val s = it.skip(2);
    assert_eq(s.next().unwrap(), 3);
    // Skip consumed 1, 2, 3 from source; `it` resumes at 4.
    assert_eq(it.next().unwrap(), 4);
}

#[test]
fn test_enumerate_shares_source_state() {
    val v = [10, 20, 30];
    val it = v.iter();
    val e = it.enumerate();
    val pair = e.next().unwrap();
    assert_eq(pair.0, 0);
    assert_eq(pair.1, 10);
    assert_eq(it.next().unwrap(), 20);
}
