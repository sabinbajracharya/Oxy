// === Problem: Combination Sum (LeetCode #39) ===
// Given distinct integers and a target, return all unique combinations
// that sum to target. Each number can be used unlimited times.
//
// === Pattern: Backtracking ===
// For each index i, either use nums[i] (stay at same index for unlimited
// reuse) or skip to next. With shared mutable collections, we use
// push/pop/recurse/undo for classic backtracking.
//
// === Intuition ===
// At each state, we track remaining target. If target == 0, snapshot current.
// For each index from start: if candidate <= target, push, recurse, pop.
//
// === Pattern Recognition ===
// - "Combinations that sum to target" → backtracking with remaining target
// - "Unlimited use" → recurse at same index
// - Classic push/pop with shared mutable state

fn main() {
    val candidates = [2, 3, 6, 7];
    val result = combination_sum(candidates, 7);
    for c in result {
        println("{:?}", c);
    }
}

fn backtrack(candidates: List, start: Int, target: Int, current: List, result: List) {
    if target == 0 {
        result.push(current.clone());
        return;
    }
    var i = start;
    while i < candidates.len() {
        val val = candidates[i];
        if val <= target {
            current.push(val);
            backtrack(candidates, i, target - val, current, result);
            current.pop();
        }
        i = i + 1;
    }
}

fn combination_sum(candidates: List, target: Int) -> List {
    val result = [];
    val current = [];
    backtrack(candidates, 0, target, current, result);
    result
}

#[test]
fn test_example() {
    val result = combination_sum([2, 3, 6, 7], 7);
    assert(result.len() > 0);
}

#[test]
fn test_no_solution() {
    val result = combination_sum([2], 1);
    assert_eq(result.len(), 0);
}
