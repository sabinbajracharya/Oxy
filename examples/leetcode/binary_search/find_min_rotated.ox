// === Problem: Find Minimum in Rotated Sorted Array (LeetCode #153) ===
// Given a rotated sorted array with unique elements, return the minimum
// element in O(log n).
//
// === Pattern: Binary Search (Pivot Detection) ===
// The minimum element is the only one smaller than its left neighbor
// (or the first element if never rotated). Binary search to find the
// "inflection point" where the array wraps around.
//
// === Intuition ===
// Compare nums[mid] with nums[right]. If nums[mid] > nums[right], the
// minimum is in the right half. Otherwise it's in the left half (or is
// mid itself).
//
// === Pattern Recognition ===
// - "Minimum in rotated sorted array" → binary search for inflection
// - Compare with rightmost element to determine which half is unsorted
// - The unsorted half contains the pivot
//
// === Tips ===
// - If nums[mid] > nums[right], min is to the right
// - If nums[mid] < nums[right], min is at or to the left of mid
// - Loop until left == right

fn main() {
    let nums = vec![3, 4, 5, 1, 2];
    println!("{}", find_min(nums));
}

fn find_min(nums: Vec) -> i64 {
    let mut left = 0i64;
    let mut right = nums.len() - 1;
    while left < right {
        let mid = left + (right - left) / 2;
        if nums[mid] > nums[right] {
            left = mid + 1;
        } else {
            right = mid;
        }
    }
    nums[left]
}

#[test]
fn test_example() {
    assert_eq!(find_min(vec![3, 4, 5, 1, 2]), 1);
}

#[test]
fn test_not_rotated() {
    assert_eq!(find_min(vec![1, 2, 3, 4, 5]), 1);
}

#[test]
fn test_two_elements() {
    assert_eq!(find_min(vec![2, 1]), 1);
}
