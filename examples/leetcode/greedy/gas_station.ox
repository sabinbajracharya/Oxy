// === Problem: Gas Station (LeetCode #134) ===
// There are n gas stations along a circular route. At station i you get
// gas[i] and spend cost[i] to reach the next station. Return the starting
// station index that lets you complete the circuit, or -1.
//
// === Pattern: Greedy ===
// If total gas >= total cost, a solution exists. When the tank goes
// negative, reset the start to the next station. The greedy insight:
// if you can't reach station j from i, no station between i and j works.
//
// === Intuition ===
// Track running tank. When tank < 0, reset tank to 0 and set start = i+1.
// This works because any station between start and the failure point
// would have an even smaller tank at that point.
//
// === Pattern Recognition ===
// - "Circular route with costs" → cumulative sum / greedy
// - If total_gas >= total_cost, answer exists (invariant)
// - When stuck, the next station is the only candidate for start
//
// === Tips ===
// - Check total gas vs total cost first
// - Tank goes negative → reset and try next station
// - Return -1 if total cost > total gas

fn main() {
    let gas = [1, 2, 3, 4, 5];
    let cost = [3, 4, 5, 1, 2];
    println("{}", can_complete_circuit(gas, cost));
}

fn can_complete_circuit(gas: List, cost: List) -> Int {
    let n = gas.len();
    let mut total = 0;
    let mut tank = 0;
    let mut start = 0;
    let mut i = 0;
    while i < n {
        let net = gas[i] - cost[i];
        total = total + net;
        tank = tank + net;
        if tank < 0 {
            start = i + 1;
            tank = 0;
        }
        i = i + 1;
    }
    if total >= 0 { start } else { -1 }
}

#[test]
fn test_possible() {
    let gas = [1, 2, 3, 4, 5];
    let cost = [3, 4, 5, 1, 2];
    assert_eq(can_complete_circuit(gas, cost), 3);
}

#[test]
fn test_impossible() {
    let gas = [2, 3, 4];
    let cost = [3, 4, 3];
    assert_eq(can_complete_circuit(gas, cost), -1);
}
