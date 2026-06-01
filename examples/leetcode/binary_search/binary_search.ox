// === Problem: Binary Search (LeetCode #704) ===
// Given a sorted array of integers and a target, return the index of
// target or -1 if not found. Must run in O(log n).
//
// === Pattern: Binary Search ===
// Repeatedly halve the search space. Compare target with middle element.
// If target < mid, search left half. If target > mid, search right half.
//
// === Intuition ===
// The array is sorted, so each comparison eliminates half the remaining
// elements. Start with the full range [0, n-1], and narrow until you
// find the target or the range is empty.
//
// === Pattern Recognition ===
// - "Sorted array" + "find element" → binary search
// - If there's a monotonic predicate, binary search applies
// - O(log n) on sorted data is almost always binary search
//
// === Tips ===
// - mid = left + (right - left) / 2 to avoid overflow
// - while left <= right (not <), otherwise single-element fails
// - Return -1 for not found

fn main() {
    val nums = [-1, 0, 3, 5, 9, 12];
    io::println("{}", search(nums, 9));
    io::println("{}", search(nums, 2));
}

fn search(nums: List, target: Int) -> Int {
    var left = 0;
    var right = nums.len() - 1;
    while left <= right {
        val mid = left + (right - left) / 2;
        val mid_val = nums[mid];
        if mid_val == target {
            return mid;
        } else if mid_val < target {
            left = mid + 1;
        } else {
            right = mid - 1;
        }
    }
    -1
}

#[test]
fn test_found() {
    val nums = [-1, 0, 3, 5, 9, 12];
    assert::eq(search(nums, 9), 4);
}

#[test]
fn test_not_found() {
    val nums = [-1, 0, 3, 5, 9, 12];
    assert::eq(search(nums, 2), -1);
}

#[test]
fn test_single_element() {
    assert::eq(search([5], 5), 0);
    assert::eq(search([5], 3), -1);
}
