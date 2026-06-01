// === Problem: Coin Change (LeetCode #322) ===
// Given coins of different denominations and an amount, find the fewest
// number of coins needed to make up that amount. Return -1 if impossible.
//
// === Pattern: Dynamic Programming (Unbounded Knapsack) ===
// dp[i] = min coins needed for amount i.
// dp[i] = min(dp[i], 1 + dp[i - coin]) for each coin where i >= coin.
// Initialize dp[0] = 0, others = INF.
//
// === Intuition ===
// For each amount from 1 to amount, try subtracting each coin value.
// The minimum coins for amount i is 1 more than the minimum for amount
// (i - coin). This is bottom-up: build up from 0.
//
// === Pattern Recognition ===
// - "Minimum coins to make amount" → unbounded knapsack / coin change DP
// - Each coin can be used unlimited times → iterate coins inside amount loop
// - "Fewest number" → minimize over subproblems
//
// === Tips ===
// - dp[0] = 0 (0 coins to make amount 0)
// - Initialize dp with a sentinel (amount + 1 or MAX)
// - Return -1 if dp[amount] is still the sentinel

fn main() {
    val coins = [1, 2, 5];
    io::println("{}", coin_change(coins, 11));
}

fn coin_change(coins: List, amount: Int) -> Int {
    val inf = amount + 1;
    var dp = [0];
    for _i in 0..amount {
        dp.push(inf);
    }
    for a in 1..=amount {
        for coin in coins {
            if coin <= a {
                val prev = dp[a - coin];
                if prev != inf {
                    val candidate = prev + 1;
                    if candidate < dp[a] {
                        dp[a] = candidate;
                    }
                }
            }
        }
    }
    if dp[amount] == inf { -1 } else { dp[amount] }
}

#[test]
fn test_example() {
    assert::eq(coin_change([1, 2, 5], 11), 3);
}

#[test]
fn test_impossible() {
    assert::eq(coin_change([2], 3), -1);
}

#[test]
fn test_zero() {
    assert::eq(coin_change([1], 0), 0);
}
