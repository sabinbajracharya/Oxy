// === Problem: Permutations (LeetCode #46) ===
// Given an array of distinct integers, return all possible permutations.
//
// === Pattern: Backtracking ===
// Build permutations by choosing one unused element at a time. Track which
// elements are used with a Vec<bool>. Since Oxy has value semantics, use
// functional recursion: return accumulated results rather than mutating
// a shared result parameter.
//
// === Intuition ===
// For each unused element, pick it, mark it used, recurse on remaining
// positions, then unmark. Accumulate results through returned values.
//
// === Pattern Recognition ===
// - "All permutations" → backtracking with visited tracking
// - Return-based recursion for value-semantics languages

fn main() {
    let nums = vec![1, 2, 3];
    let result = permute(nums);
    for p in result {
        println!("{:?}", p);
    }
}

fn backtrack(nums: Vec, current: Vec, used: Vec) -> Vec {
    if current.len() == nums.len() {
        return vec![current];
    }
    let mut result = vec![];
    let mut i = 0i64;
    while i < nums.len() {
        if !used[i] {
            let mut new_used = used;
            new_used[i] = true;
            let mut new_current = current;
            new_current.push(nums[i]);
            let sub_results = backtrack(nums, new_current, new_used);
            for sub in sub_results {
                result.push(sub);
            }
        }
        i = i + 1;
    }
    result
}

fn permute(nums: Vec) -> Vec {
    let n = nums.len();
    let mut used = vec![];
    let mut i = 0i64;
    while i < n {
        used.push(false);
        i = i + 1;
    }
    backtrack(nums, vec![], used)
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
