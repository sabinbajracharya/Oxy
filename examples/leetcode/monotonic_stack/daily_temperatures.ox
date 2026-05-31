// === Problem: Daily Temperatures (LeetCode #739) ===
// Given an array of daily temperatures, return an array where answer[i]
// is the number of days you must wait until a warmer temperature.
// If no warmer day exists, answer[i] = 0.
//
// === Pattern: Monotonic Stack (Decreasing) ===
// Maintain a stack of indices with decreasing temperatures. When a
// warmer day arrives, it resolves all colder days on the stack.
//
// === Intuition ===
// Walk right-to-left, maintaining a stack of "warmer days ahead".
// For each day, pop the stack until we find a warmer day or it empties.
// If found, distance = (warmer_idx - current_idx). Push current.
//
// Alternative (left-to-right): push indices. When a warmer temperature
// comes, pop all colder ones and set their answer.
//
// === Pattern Recognition ===
// - "Next greater/smaller element" → monotonic stack
// - "Days until warmer" = "next greater to the right"
// - Stack stores indices waiting for a greater element
//
// === Tips ===
// - Iterate left to right, push indices onto stack
// - When t[i] > t[stack.top()], resolve all colder days
// - Default answer is 0 (List initialized with zeros)

fn main() {
    val temps = [73, 74, 75, 71, 69, 72, 76, 73];
    val result = daily_temperatures(temps);
    println("{:?}", result);
}

fn daily_temperatures(temps: List) -> List {
    val n = temps.len();
    var answer = [];
    for i in 0..n {
        answer.push(0);
    }
    var stack = [];
    for i in 0..n {
        while !stack.is_empty() {
            val prev_idx = stack.last().unwrap();
            if temps[i] > temps[prev_idx] {
                answer[prev_idx] = i - prev_idx;
                stack.pop();
            } else {
                break;
            }
        }
        stack.push(i);
    }
    answer
}

#[test]
fn test_example() {
    val temps = [73, 74, 75, 71, 69, 72, 76, 73];
    assert_eq(
        daily_temperatures(temps),
        [1, 1, 4, 2, 1, 1, 0, 0]
    );
}

#[test]
fn test_all_decreasing() {
    val temps = [5, 4, 3, 2, 1];
    assert_eq(daily_temperatures(temps), [0, 0, 0, 0, 0]);
}

#[test]
fn test_all_increasing() {
    val temps = [1, 2, 3, 4];
    assert_eq(daily_temperatures(temps), [1, 1, 1, 0]);
}
