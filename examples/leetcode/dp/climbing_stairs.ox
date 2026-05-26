// === Problem: Climbing Stairs (LeetCode #70) ===
// You can climb 1 or 2 steps at a time. How many distinct ways to
// climb n stairs?
//
// === Pattern: Dynamic Programming (Fibonacci) ===
// ways(n) = ways(n-1) + ways(n-2). This is the Fibonacci sequence.
// Base cases: ways(1) = 1, ways(2) = 2.
//
// === Intuition ===
// To reach step n, your last move was either 1 step from n-1 or 2 steps
// from n-2. So total ways = ways(n-1) + ways(n-2). This is a classic
// bottom-up DP problem.
//
// === Pattern Recognition ===
// - "Count ways to reach N with steps of size X, Y" → 1D DP
// - Overlapping subproblems → DP or memoization
// - Linear recurrence with constant coefficients → iterative optimal
//
// === Tips ===
// - Use three variables (prev2, prev1, current) for O(1) space
// - This is Fibonacci: f(1)=1, f(2)=2, f(3)=3, f(4)=5
// - O(n) time, O(1) space

fn main() {
    println!("{}", climb_stairs(5));
}

fn climb_stairs(n: int) -> int {
    if n <= 2 {
        return n;
    }
    let mut prev2 = 1;
    let mut prev1 = 2;
    for _i in 3..=n {
        let current = prev1 + prev2;
        prev2 = prev1;
        prev1 = current;
    }
    prev1
}

#[test]
fn test_small() {
    assert_eq!(climb_stairs(2), 2);
    assert_eq!(climb_stairs(3), 3);
}

#[test]
fn test_medium() {
    assert_eq!(climb_stairs(4), 5);
    assert_eq!(climb_stairs(5), 8);
}
