// === Problem: Subsets (LeetCode #78) ===
// Given an array of distinct integers, return all possible subsets (the power set).
//
// === Pattern: Backtracking ===
// For each element, choose to include or exclude it. Use functional
// recursion: for each position, combine results from both branches.
//
// === Intuition ===
// subsets([1,2,3]) = subsets_from(0) = combos with and without nums[0].
// Base case: i == len → return [[]].
// For each i: exclude = subsets_from(i+1), include = each of exclude + nums[i].
//
// === Pattern Recognition ===
// - "All subsets" → include/exclude backtracking
// - Power set = 2^n combinations

fn main() {
    let nums = vec![1, 2, 3];
    let result = subsets(nums);
    for s in result {
        println!("{:?}", s);
    }
}

fn subsets_from(nums: Vec, start: i64) -> Vec {
    if start == nums.len() {
        return vec![vec![]];
    }
    let without = subsets_from(nums, start + 1);
    let mut result = vec![];
    for sub in without {
        result.push(sub);
        let mut with = sub;
        with.push(nums[start]);
        result.push(with);
    }
    result
}

fn subsets(nums: Vec) -> Vec {
    subsets_from(nums, 0)
}

#[test]
fn test_example() {
    let result = subsets(vec![1, 2, 3]);
    assert_eq!(result.len(), 8);
}

#[test]
fn test_empty() {
    let result = subsets(vec![]);
    assert_eq!(result.len(), 1);
}
