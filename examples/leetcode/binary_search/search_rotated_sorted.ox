// === Problem: Search in Rotated Sorted Array (LeetCode #33) ===
// A sorted array was rotated at an unknown pivot. Given a target, return
// its index or -1. Must be O(log n). No duplicates.
//
// === Pattern: Binary Search (Modified) ===
// A rotated sorted array has one property: at least one half is always
// sorted. Determine which half is sorted by comparing nums[left] with
// nums[mid]. Then check if the target lies in the sorted half.
//
// === Intuition ===
// Standard binary search assumes a fully sorted array. After rotation,
// if nums[left] <= nums[mid], the left half is sorted. Otherwise, the
// right half is sorted. Check the target against the sorted half's bounds.
//
// === Pattern Recognition ===
// - "Rotated sorted array" + "O(log n)" → modified binary search
// - Key insight: one half is always sorted
// - "Find in nearly sorted" → adapt binary search
//
// === Tips ===
// - No duplicates makes the nums[left] <= nums[mid] check reliable
// - With duplicates, worst case degrades to O(n)
// - The pivot is never at index 0 if the array was actually rotated

fn main() {
    let nums = vec![4, 5, 6, 7, 0, 1, 2];
    println!("{}", search(nums, 0));
    println!("{}", search(nums, 3));
}

fn search(nums: Vec, target: int) -> int {
    let mut left = 0;
    let mut right = nums.len() - 1;
    while left <= right {
        let mid = left + (right - left) / 2;
        if nums[mid] == target {
            return mid;
        }
        // Left half is sorted
        if nums[left] <= nums[mid] {
            if nums[left] <= target && target < nums[mid] {
                right = mid - 1;
            } else {
                left = mid + 1;
            }
        } else {
            // Right half is sorted
            if nums[mid] < target && target <= nums[right] {
                left = mid + 1;
            } else {
                right = mid - 1;
            }
        }
    }
    -1
}

#[test]
fn test_found() {
    let nums = vec![4, 5, 6, 7, 0, 1, 2];
    assert_eq!(search(nums, 0), 4);
    assert_eq!(search(nums, 6), 2);
}

#[test]
fn test_not_found() {
    let nums = vec![4, 5, 6, 7, 0, 1, 2];
    assert_eq!(search(nums, 3), -1);
}

#[test]
fn test_single_element() {
    assert_eq!(search(vec![1], 0), -1);
    assert_eq!(search(vec![1], 1), 0);
}
