// === Problem: Longest Substring Without Repeating Characters (LeetCode #3) ===
// Given a string s, find the length of the longest substring without
// repeating characters.
//
// === Pattern: Sliding Window ===
// Maintain a window [left, right] with all unique characters. When a
// duplicate enters, shrink left until it's unique again. Track the max
// window size.
//
// === Intuition ===
// A sliding window expands right one character at a time. If the new
// character is already in the window, shrink from the left until it's gone.
// The window is always valid (all unique). Use a HashMap to store each
// character's last position for O(1) "shrink to" jumping.
//
// === Pattern Recognition ===
// - "Longest substring with property X" → sliding window
// - "Without repeating" → HashSet or HashMap of window contents
// - Window validity check O(1) → remove leftmost char on shrink
//
// === Tips ===
// - HashMap<char, int> for char → index mapping
// - When duplicate found, jump left past the previous occurrence
// - max_len = max(max_len, right - left + 1)

fn main() {
    println!("{}", length_of_longest_substring("abcabcbb"));
    println!("{}", length_of_longest_substring("bbbbb"));
}

fn length_of_longest_substring(s: String) -> int {
    let mut seen = HashMap::new();
    let mut left = 0;
    let mut max_len = 0;
    let mut right = 0;
    let n = s.len();
    while right < n {
        let ch = s.char_at(right);
        if let Some(prev_idx) = seen.get(ch) && prev_idx >= left {
            left = prev_idx + 1;
        }
        seen.insert(ch, right);
        let len = right - left + 1;
        if len > max_len {
            max_len = len;
        }
        right = right + 1;
    }
    max_len
}

#[test]
fn test_example() {
    assert_eq!(length_of_longest_substring("abcabcbb"), 3);
}

#[test]
fn test_all_same() {
    assert_eq!(length_of_longest_substring("bbbbb"), 1);
}

#[test]
fn test_empty() {
    assert_eq!(length_of_longest_substring(""), 0);
}

#[test]
fn test_single() {
    assert_eq!(length_of_longest_substring("a"), 1);
}

#[test]
fn test_pwwkew() {
    assert_eq!(length_of_longest_substring("pwwkew"), 3);
}
