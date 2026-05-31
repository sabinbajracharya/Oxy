// === Problem: Subsets (LeetCode #78) ===
// Given an array of unique integers, return all possible subsets (the power set).
//
// === Pattern: Backtracking ===
// For each element, branch: include it or skip. With shared mutable collections,
// we use classic push/pop/recurse/undo to explore the decision tree.
//
// === Intuition ===
// Start with empty current. At each index i:
//   - Skip: recurse to i+1 without changing current
//   - Include: push nums[i], recurse to i+1, pop (backtrack)
// Snapshot current with .clone() when adding to result.
//
// === Pattern Recognition ===
// - "All subsets" / "power set" → include/exclude backtracking
// - Shared mutable state for classic push/pop/recurse/undo

fn main() {
    let nums = vec(1, 2, 3);
    let result = subsets(nums);
    for s in result {
        println("{:?}", s);
    }
}

fn backtrack(nums: Vec, start: int, current: Vec, result: Vec) {
    result.push(current.clone());
    let mut i = start;
    while i < nums.len() {
        current.push(nums[i]);
        backtrack(nums, i + 1, current, result);
        current.pop();
        i = i + 1;
    }
}

fn subsets(nums: Vec) -> Vec {
    let result = vec();
    let current = vec();
    backtrack(nums, 0, current, result);
    result
}

#[test]
fn test_example() {
    let result = subsets(vec(1, 2, 3));
    assert_eq(result.len(), 8);
}

#[test]
fn test_empty() {
    let result = subsets(vec());
    assert_eq(result.len(), 1);
    assert_eq(result[0], vec());
}
