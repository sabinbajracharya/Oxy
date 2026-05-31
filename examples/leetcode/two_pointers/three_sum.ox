// === Problem: 3Sum (LeetCode #15) ===
// Given an integer array nums, return all unique triplets [nums[i], nums[j], nums[k]]
// such that i != j != k and nums[i] + nums[j] + nums[k] == 0.
//
// === Pattern: Two Pointers (Sorted) ===
// Sort the array first. For each index i, use two pointers (left, right) on the
// subarray after i to find pairs that sum to -nums[i]. Skip duplicates to avoid
// duplicate triplets.
//
// === Intuition ===
// O(n³) brute force → O(n²) with sort + two pointers.
// Sort makes duplicates adjacent (easy to skip) and lets us use two-pointer
// technique on the inner loop. For each i, the problem reduces to Two Sum II
// (sorted) with target = -nums[i].
//
// === Pattern Recognition ===
// - "Find k numbers summing to target" → sort + k-2 nested loops + two pointers
// - "No duplicates" → sort + skip adjacent equal values
// - Three sum → fix one, two-sum the rest
//
// === Tips ===
// - sort_by with Int ordering
// - Skip duplicate i values
// - Skip duplicate left/right values after finding a match

fn main() {
    let nums = list(-1, 0, 1, 2, -1, -4);
    let triplets = three_sum(nums);
    for t in triplets {
        println("{:?}", t);
    }
}

fn three_sum(nums: List) -> List {
    let mut sorted = nums;
    sorted.sort_by(|a, b| {
        if a < b { -1 } else if a > b { 1 } else { 0 }
    });
    let mut result = list();
    let n = sorted.len();
    let mut i = 0;
    while i < n - 2 {
        let a = sorted[i];
        if a > 0 {
            break; // Can't sum to 0 if smallest > 0
        }
        // Skip duplicate starting values
        if i > 0 && a == sorted[i - 1] {
            i = i + 1;
            continue;
        }
        let mut left = i + 1;
        let mut right = n - 1;
        while left < right {
            let sum = a + sorted[left] + sorted[right];
            if sum < 0 {
                left = left + 1;
            } else if sum > 0 {
                right = right - 1;
            } else {
                result.push(list(a, sorted[left], sorted[right]));
                // Skip duplicates
                while left < right && sorted[left] == sorted[left + 1] {
                    left = left + 1;
                }
                while left < right && sorted[right] == sorted[right - 1] {
                    right = right - 1;
                }
                left = left + 1;
                right = right - 1;
            }
        }
        i = i + 1;
    }
    result
}

#[test]
fn test_example() {
    let nums = list(-1, 0, 1, 2, -1, -4);
    let result = three_sum(nums);
    assert_eq(result.len(), 2);
}

#[test]
fn test_no_solution() {
    let nums = list(1, 2, 3);
    let result = three_sum(nums);
    assert_eq(result.len(), 0);
}

#[test]
fn test_all_zeros() {
    let nums = list(0, 0, 0, 0);
    let result = three_sum(nums);
    assert_eq(result.len(), 1);
}
