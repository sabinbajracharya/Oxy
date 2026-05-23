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
// - Default answer is 0 (Vec initialized with zeros)

fn main() {
    let temps = vec![73, 74, 75, 71, 69, 72, 76, 73];
    let result = daily_temperatures(temps);
    println!("{:?}", result);
}

fn daily_temperatures(temps: Vec) -> Vec {
    let n = temps.len();
    let mut answer = vec![];
    for i in 0..n {
        answer.push(0);
    }
    let mut stack = vec![];
    let mut i = 0;
    while i < n {
        while !stack.is_empty() {
            let prev_idx = stack.last().unwrap();
            if temps[i] > temps[prev_idx] {
                answer[prev_idx] = i - prev_idx;
                stack.pop();
            } else {
                break;
            }
        }
        stack.push(i);
        i = i + 1;
    }
    answer
}

#[test]
fn test_example() {
    let temps = vec![73, 74, 75, 71, 69, 72, 76, 73];
    assert_eq!(
        daily_temperatures(temps),
        vec![1, 1, 4, 2, 1, 1, 0, 0]
    );
}

#[test]
fn test_all_decreasing() {
    let temps = vec![5, 4, 3, 2, 1];
    assert_eq!(daily_temperatures(temps), vec![0, 0, 0, 0, 0]);
}

#[test]
fn test_all_increasing() {
    let temps = vec![1, 2, 3, 4];
    assert_eq!(daily_temperatures(temps), vec![1, 1, 1, 0]);
}
