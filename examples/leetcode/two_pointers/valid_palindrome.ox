// === Problem: Valid Palindrome (LeetCode #125) ===
// Given a string s, return true if it's a palindrome after converting
// to lowercase and removing non-alphanumeric characters.
//
// === Pattern: Two Pointers ===
// Start with left=0, right=len-1. Move inward, skipping non-alphanumeric
// chars. Compare at each step. Two pointers works for any problem where
// you compare elements from both ends.
//
// === Intuition ===
// A palindrome reads the same forward and backward. Instead of building
// a cleaned string (O(n) extra space), use two pointers on the original
// string and skip invalid characters.
//
// === Pattern Recognition ===
// - "Palindrome" → two pointers from ends
// - "Compare from both ends" → left/right pointers
// - "Skip certain elements" → advance pointer past invalid
//
// === Tips ===
// - Use char_at() to access string characters
// - is_alphanumeric() on char type
// - to_lowercase() for case-insensitive compare
// - Empty string is a palindrome

fn main() {
    println!("{}", is_palindrome("A man, a plan, a canal: Panama"));
    println!("{}", is_palindrome("race a car"));
}

fn is_palindrome(s: String) -> bool {
    let mut left = 0i64;
    let mut right = s.len() - 1;
    while left < right {
        let lch = s.char_at(left);
        if !lch.is_alphanumeric() {
            left = left + 1;
            continue;
        }
        let rch = s.char_at(right);
        if !rch.is_alphanumeric() {
            right = right - 1;
            continue;
        }
        if lch.to_lowercase() != rch.to_lowercase() {
            return false;
        }
        left = left + 1;
        right = right - 1;
    }
    true
}

#[test]
fn test_valid_palindrome() {
    assert!(is_palindrome("A man, a plan, a canal: Panama"));
}

#[test]
fn test_not_palindrome() {
    assert!(!is_palindrome("race a car"));
}

#[test]
fn test_empty_string() {
    assert!(is_palindrome(""));
}

#[test]
fn test_single_char() {
    assert!(is_palindrome("a"));
}
