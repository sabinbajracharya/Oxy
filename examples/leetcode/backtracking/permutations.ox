// === Problem: Permutations (LeetCode #46) ===
// Given an array of distinct integers, return all possible permutations.
//
// === Pattern: Backtracking ===
// Build permutations by choosing one unused element at a time. Track which
// elements are used with a Vec<bool>. With shared mutable collections (Rc),
// passing by value shares the underlying data — mutations propagate.
//
// === Intuition ===
// For each unused element, pick it, mark it used, recurse on remaining
// positions, then unmark (backtrack). Snapshot current with .clone()
// when storing a complete permutation.
//
// === Pattern Recognition ===
// - "All permutations" → backtracking with visited tracking
// - Shared mutable collections let us use classic push/pop/recurse/undo

fn main() {
    let nums = vec![1, 2, 3];
    let result = permute(nums);
    for p in result {
        println!("{:?}", p);
    }
}

fn backtrack(nums: Vec, current: Vec, used: Vec, result: Vec) {
    if current.len() == nums.len() {
        result.push(current.clone());
        return;
    }
    let mut i = 0;
    while i < nums.len() {
        if !used[i] {
            used[i] = true;
            current.push(nums[i]);
            backtrack(nums, current, used, result);
            current.pop();
            used[i] = false;
        }
        i = i + 1;
    }
}

fn permute(nums: Vec) -> Vec {
    let n = nums.len();
    let mut used = vec![];
    let mut i = 0;
    while i < n {
        used.push(false);
        i = i + 1;
    }
    let result = vec![];
    let current = vec![];
    backtrack(nums, current, used, result);
    result
}

#[test]
fn test_example() {
    let result = permute(vec![1, 2, 3]);
    assert_eq!(result.len(), 6);
}

#[test]
fn test_single() {
    let result = permute(vec![1]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], vec![1]);
}
