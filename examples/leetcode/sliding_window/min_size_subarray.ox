// === Problem: Minimum Size Subarray Sum (LeetCode #209) ===
// Given an array of positive integers and a target sum, find the minimal
// length of a contiguous subarray whose sum >= target. Return 0 if none.
//
// === Pattern: Sliding Window (Variable Size) ===
// Expand right until sum >= target, then shrink from left to minimize
// while still meeting the condition. Think of it like a caterpillar:
// expand to eat enough, then contract to the minimal satisfying length.
//
// === Intuition ===
// Add nums[right]. While sum >= target, record length and subtract
// nums[left], then advance left. This finds the minimal window ending
// at each right.
//
// === Pattern Recognition ===
// - "Minimum subarray with sum >= K" → variable sliding window
// - All positive numbers → sum always increases expanding right
// - "Contiguous subarray" → window is contiguous, no gaps
//
// === Tips ===
// - All numbers are positive, so sum is monotonic as we expand
// - min_len starts at a large sentinel value
// - Return 0 if min_len was never updated

fn main() {
    let nums = vec![2, 3, 1, 2, 4, 3];
    println!("{}", min_sub_array_len(7, nums));
}

fn min_sub_array_len(target: int, nums: Vec) -> int {
    let n = nums.len();
    let mut left = 0i64;
    let mut sum = 0i64;
    let mut min_len = n + 1; // sentinel
    let mut right = 0i64;
    while right < n {
        sum = sum + nums[right];
        while sum >= target {
            let len = right - left + 1;
            if len < min_len {
                min_len = len;
            }
            sum = sum - nums[left];
            left = left + 1;
        }
        right = right + 1;
    }
    if min_len > n { 0 } else { min_len }
}

#[test]
fn test_example() {
    assert_eq!(min_sub_array_len(7, vec![2, 3, 1, 2, 4, 3]), 2);
}

#[test]
fn test_no_solution() {
    assert_eq!(min_sub_array_len(100, vec![1, 2, 3]), 0);
}

#[test]
fn test_exact_single() {
    assert_eq!(min_sub_array_len(4, vec![1, 4, 4]), 1);
}

#[test]
fn test_entire_array() {
    assert_eq!(min_sub_array_len(15, vec![1, 2, 3, 4, 5]), 5);
}
