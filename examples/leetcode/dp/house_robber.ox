// === Problem: House Robber (LeetCode #198) ===
// You cannot rob two adjacent houses. Given an array of house values,
// return the maximum amount you can rob tonight.
//
// === Pattern: Dynamic Programming (1D, Non-Adjacent) ===
// dp[i] = max(dp[i-1], dp[i-2] + nums[i]).
// At each house, either skip it (take dp[i-1]) or rob it (take nums[i] + dp[i-2]).
//
// === Intuition ===
// Two choices at each house: skip or rob. If you rob house i, you can't
// rob i-1, so the best you can do is nums[i] + dp[i-2]. If you skip,
// the best is dp[i-1]. Take the max of these two.
//
// === Pattern Recognition ===
// - "Maximum sum with no adjacent selections" → classic DP
// - Two states per element → skip or take
// - Linear scan with two rolling variables
//
// === Tips ===
// - Use two variables instead of full array for O(1) space
// - prev2 = best up to i-2, prev1 = best up to i-1
// - Base: empty → 0, single → nums[0]

fn main() {
    val nums = [1, 2, 3, 1];
    io::println("{}", rob(nums));
}

fn rob(nums: List) -> Int {
    val n = nums.len();
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return nums[0];
    }
    var prev2 = nums[0];
    var prev1 = if nums[0] > nums[1] { nums[0] } else { nums[1] };
    for i in 2..n {
        val current = if prev1 > (prev2 + nums[i]) { prev1 } else { prev2 + nums[i] };
        prev2 = prev1;
        prev1 = current;
    }
    prev1
}

#[test]
fn test_example() {
    assert::eq(rob([1, 2, 3, 1]), 4);
}

#[test]
fn test_two_houses() {
    assert::eq(rob([2, 1]), 2);
}

#[test]
fn test_empty() {
    assert::eq(rob([]), 0);
}
