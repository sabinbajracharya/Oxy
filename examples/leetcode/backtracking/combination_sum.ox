// === Problem: Combination Sum (LeetCode #39) ===
// Given distinct integers and a target, return all unique combinations
// that sum to target. Each number can be used unlimited times.
//
// === Pattern: Backtracking ===
// For each index i, either skip nums[i] or use it (stay at same index
// for unlimited reuse). Return accumulated results.
//
// === Intuition ===
// At index i: skip branch returns combinations without nums[i].
// Use branch: take nums[i], recurse with reduced target. Prefix each
// from the use branch with nums[i]. Combine both branches.
//
// === Pattern Recognition ===
// - "Combinations that sum to target" → backtracking with index
// - "Unlimited use" → stay at same index
// - Return-based for value semantics

fn main() {
    let candidates = vec![2, 3, 6, 7];
    let result = combination_sum(candidates, 7);
    for c in result {
        println!("{:?}", c);
    }
}

fn backtrack(candidates: Vec, start: i64, target: i64) -> Vec {
    if target == 0 {
        return vec![vec![]];
    }
    if start >= candidates.len() || target < 0 {
        return vec![];
    }
    let val = candidates[start];
    let mut result = vec![];
    // Skip this candidate
    let skip = backtrack(candidates, start + 1, target);
    for s in skip {
        result.push(s);
    }
    // Use this candidate
    let use_results = backtrack(candidates, start, target - val);
    for u in use_results {
        let mut combo = u;
        combo.push(val);
        result.push(combo);
    }
    result
}

fn combination_sum(candidates: Vec, target: i64) -> Vec {
    backtrack(candidates, 0, target)
}

#[test]
fn test_example() {
    let result = combination_sum(vec![2, 3, 6, 7], 7);
    assert!(result.len() > 0);
}

#[test]
fn test_no_solution() {
    let result = combination_sum(vec![2], 1);
    assert_eq!(result.len(), 0);
}
