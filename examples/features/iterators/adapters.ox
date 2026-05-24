// === Iterator Adapters ===
// map, filter, take, skip, rev, chain, enumerate, zip, flat_map, flatten, collect

// --- map ---

#[test]
fn test_map_double() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().map(|x| x * 2);
    assert_eq!(result, vec![2, 4, 6, 8, 10]);
}

#[test]
fn test_map_to_string() {
    let v = vec![1, 2, 3];
    let result = v.iter().map(|x| x.to_string());
    assert_eq!(result[0], "1");
    assert_eq!(result[1], "2");
    assert_eq!(result[2], "3");
}

#[test]
fn test_map_empty() {
    let v: Vec<int> = vec![];
    let result = v.iter().map(|x| x * 2);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_map_strings() {
    let v = vec!["hello", "world"];
    let result = v.iter().map(|s| s.to_uppercase());
    assert_eq!(result[0], "HELLO");
    assert_eq!(result[1], "WORLD");
}

// --- filter ---

#[test]
fn test_filter_even() {
    let v = vec![1, 2, 3, 4, 5, 6];
    let result = v.iter().filter(|x| x % 2 == 0);
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_filter_none_match() {
    let v = vec![1, 3, 5];
    let result = v.iter().filter(|x| x % 2 == 0);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_filter_all_match() {
    let v = vec![2, 4, 6];
    let result = v.iter().filter(|x| x % 2 == 0);
    assert_eq!(result.len(), 3);
}

#[test]
fn test_filter_empty() {
    let v: Vec<int> = vec![];
    let result = v.iter().filter(|x| x > 0);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_filter_strings() {
    let v = vec!["foo", "bar", "baz", "qux"];
    let result = v.iter().filter(|s| s.starts_with("b"));
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], "bar");
    assert_eq!(result[1], "baz");
}

// --- map + filter chaining ---

#[test]
fn test_filter_then_map() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().filter(|x| x % 2 == 0).iter().map(|x| x * x);
    assert_eq!(result, vec![4, 16]);
}

#[test]
fn test_map_then_filter() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().map(|x| x * 2).iter().filter(|x| x > 4);
    assert_eq!(result, vec![6, 8, 10]);
}

// --- take ---

#[test]
fn test_take_basic() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().take(3).collect();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_take_more_than_len() {
    let v = vec![1, 2, 3];
    let result = v.iter().take(10).collect();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_take_zero() {
    let v = vec![1, 2, 3];
    let result = v.iter().take(0).collect();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_take_exact_len() {
    let v = vec![1, 2, 3];
    let result = v.iter().take(3).collect();
    assert_eq!(result, vec![1, 2, 3]);
}

// --- skip ---

#[test]
fn test_skip_basic() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().skip(2).collect();
    assert_eq!(result, vec![3, 4, 5]);
}

#[test]
fn test_skip_all() {
    let v = vec![1, 2, 3];
    let result = v.iter().skip(10).collect();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_skip_zero() {
    let v = vec![1, 2, 3];
    let result = v.iter().skip(0).collect();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_skip_exact_len() {
    let v = vec![1, 2, 3];
    let result = v.iter().skip(3).collect();
    assert_eq!(result.len(), 0);
}

// --- take + skip ---

#[test]
fn test_skip_then_take() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().skip(1).take(3).collect();
    assert_eq!(result, vec![2, 3, 4]);
}

#[test]
fn test_take_then_skip() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().take(4).skip(1).collect();
    assert_eq!(result, vec![2, 3, 4]);
}

// --- rev ---

#[test]
fn test_rev_basic() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().rev();
    assert_eq!(result, vec![5, 4, 3, 2, 1]);
}

#[test]
fn test_rev_single() {
    let v = vec![42];
    let result = v.iter().rev();
    assert_eq!(result, vec![42]);
}

#[test]
fn test_rev_empty() {
    let v: Vec<int> = vec![];
    let result = v.iter().rev();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_rev_strings() {
    let v = vec!["a", "b", "c"];
    let result = v.iter().rev();
    assert_eq!(result[0], "c");
    assert_eq!(result[1], "b");
    assert_eq!(result[2], "a");
}

// --- chain ---

#[test]
fn test_chain_two_vecs() {
    let a = vec![1, 2, 3];
    let b = vec![4, 5, 6];
    let result = a.iter().chain(b.iter()).collect();
    assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_chain_empty_first() {
    let a: Vec<int> = vec![];
    let b = vec![1, 2, 3];
    let result = a.iter().chain(b.iter()).collect();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_chain_empty_second() {
    let a = vec![1, 2, 3];
    let b: Vec<int> = vec![];
    let result = a.iter().chain(b.iter()).collect();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_chain_both_empty() {
    let a: Vec<int> = vec![];
    let b: Vec<int> = vec![];
    let result = a.iter().chain(b.iter()).collect();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_chain_then_map() {
    let a = vec![1, 2];
    let b = vec![3, 4];
    let result = a.iter().chain(b.iter()).collect().iter().map(|x| x * 10);
    assert_eq!(result, vec![10, 20, 30, 40]);
}

// --- enumerate ---

#[test]
fn test_enumerate_basic() {
    let v = vec![10, 20, 30];
    let pairs = v.iter().enumerate().collect();
    assert_eq!(pairs.len(), 3);
    let (i0, v0) = pairs[0];
    let (i1, v1) = pairs[1];
    let (i2, v2) = pairs[2];
    assert_eq!(i0, 0);
    assert_eq!(i1, 1);
    assert_eq!(i2, 2);
    assert_eq!(v0, 10);
    assert_eq!(v1, 20);
    assert_eq!(v2, 30);
}

#[test]
fn test_enumerate_strings() {
    let v = vec!["a", "b", "c"];
    let pairs = v.iter().enumerate().collect();
    let (i, s) = pairs[0];
    assert_eq!(i, 0);
    assert_eq!(s, "a");
}

#[test]
fn test_enumerate_empty() {
    let v: Vec<int> = vec![];
    let pairs = v.iter().enumerate().collect();
    assert_eq!(pairs.len(), 0);
}

#[test]
fn test_enumerate_index_in_loop() {
    let v = vec![100, 200, 300];
    let mut sum_indices = 0;
    let mut sum_values = 0;
    for (i, x) in v.iter().enumerate().collect() {
        sum_indices = sum_indices + i;
        sum_values = sum_values + x;
    }
    assert_eq!(sum_indices, 3);   // 0+1+2
    assert_eq!(sum_values, 600);  // 100+200+300
}

// --- zip ---

#[test]
fn test_zip_basic() {
    let a = vec![1, 2, 3];
    let b = vec![10, 20, 30];
    let pairs = a.iter().zip(b.iter()).collect();
    assert_eq!(pairs.len(), 3);
    let (a0, b0) = pairs[0];
    assert_eq!(a0, 1);
    assert_eq!(b0, 10);
}

#[test]
fn test_zip_mixed_types() {
    let nums = vec![1, 2, 3];
    let strs = vec!["a", "b", "c"];
    let pairs = nums.iter().zip(strs.iter()).collect();
    let (n, s) = pairs[1];
    assert_eq!(n, 2);
    assert_eq!(s, "b");
}

#[test]
fn test_zip_stops_at_shorter() {
    let a = vec![1, 2, 3, 4, 5];
    let b = vec![10, 20];
    let pairs = a.iter().zip(b.iter()).collect();
    assert_eq!(pairs.len(), 2);
}

#[test]
fn test_zip_empty() {
    let a: Vec<int> = vec![];
    let b = vec![1, 2, 3];
    let pairs = a.iter().zip(b.iter()).collect();
    assert_eq!(pairs.len(), 0);
}

#[test]
fn test_zip_sum_of_products() {
    let a = vec![1, 2, 3];
    let b = vec![4, 5, 6];
    let pairs = a.iter().zip(b.iter()).collect();
    let mut total = 0;
    for (x, y) in pairs {
        total = total + x * y;
    }
    assert_eq!(total, 32);  // 1*4 + 2*5 + 3*6 = 4+10+18
}

// --- flat_map ---

#[test]
fn test_flat_map_expand() {
    let v = vec![1, 2, 3];
    let result = v.iter().flat_map(|x| vec![x, x * 10]).collect();
    assert_eq!(result, vec![1, 10, 2, 20, 3, 30]);
}

#[test]
fn test_flat_map_filter_like() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().flat_map(|x| if x % 2 == 0 { vec![x] } else { vec![] }).collect();
    assert_eq!(result, vec![2, 4]);
}

#[test]
fn test_flat_map_empty_input() {
    let v: Vec<int> = vec![];
    let result = v.iter().flat_map(|x| vec![x, x + 1]).collect();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_flat_map_all_empty() {
    let v = vec![1, 2, 3];
    let result = v.iter().flat_map(|_x| vec![]).collect();
    assert_eq!(result.len(), 0);
}

// --- flatten ---

#[test]
fn test_flatten_basic() {
    let v = vec![vec![1, 2], vec![3, 4], vec![5, 6]];
    let result = v.iter().flatten();
    assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_flatten_with_empty_inner() {
    let v = vec![vec![1, 2], vec![], vec![3, 4]];
    let result = v.iter().flatten();
    assert_eq!(result, vec![1, 2, 3, 4]);
}

#[test]
fn test_flatten_all_empty() {
    let v: Vec<Vec<int>> = vec![vec![], vec![], vec![]];
    let result = v.iter().flatten();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_flatten_single_inner() {
    let v = vec![vec![42]];
    let result = v.iter().flatten();
    assert_eq!(result, vec![42]);
}

// --- collect ---

#[test]
fn test_collect_roundtrip() {
    let v = vec![1, 2, 3];
    let v2 = v.iter().collect();
    assert_eq!(v2, vec![1, 2, 3]);
}

#[test]
fn test_collect_after_take() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().take(2).collect();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], 1);
    assert_eq!(result[1], 2);
}

// --- AoC-style patterns ---

#[test]
fn test_aoc_parse_line_of_numbers() {
    let line = "3 1 4 1 5 9 2 6";
    let nums = line.split_whitespace().iter().map(|s| s.parse_int().unwrap());
    assert_eq!(nums.len(), 8);
    assert_eq!(nums[0], 3);
    assert_eq!(nums[7], 6);
}

#[test]
fn test_aoc_sum_of_doubled_evens() {
    let v = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let result = v.iter().filter(|x| x % 2 == 0).iter().map(|x| x * 2).iter().fold(0, |acc, x| acc + x);
    assert_eq!(result, 40);  // (2+4+6+8)*2 = 40
}

#[test]
fn test_aoc_index_of_first_over_threshold() {
    let v = vec![10, 20, 30, 40, 50];
    let pairs = v.iter().enumerate().collect();
    let mut found_idx = -1;
    for (i, x) in pairs {
        if x > 25 && found_idx == -1 {
            found_idx = i;
        }
    }
    assert_eq!(found_idx, 2);
}
