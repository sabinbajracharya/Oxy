// === Problem: Longest Common Subsequence (LeetCode #1143) ===
// Given two strings text1 and text2, return the length of their longest
// common subsequence (LCS). A subsequence is a sequence that appears in
// the same relative order but not necessarily contiguous.
//
// === Pattern: DP (2D Grid) ===
// dp[i][j] = LCS length of text1[0..i] and text2[0..j].
// If chars match: dp[i][j] = 1 + dp[i-1][j-1].
// Else: dp[i][j] = max(dp[i-1][j], dp[i][j-1]).
//
// === Intuition ===
// Compare characters. If they match, include the character (1 + diagonal).
// If not, take the max of skipping from text1 or text2.
// Build a 2D table bottom-up.
//
// === Pattern Recognition ===
// - "Longest common subsequence/substring" → 2D DP
// - Two sequences → grid with dimensions m+1 × n+1
// - Match → diagonal, No match → max(left, up)
//
// === Tips ===
// - dp is (m+1) × (n+1) with row 0 and col 0 as zeros
// - Space optimization: only need previous row
// - This is the foundation for diff/merge/edit-distance algorithms

fn main() {
    println!("{}", longest_common_subsequence("abcde", "ace"));
}

fn longest_common_subsequence(text1: String, text2: String) -> int {
    let m = text1.len();
    let n = text2.len();
    // Use two rows for O(n) space
    let mut prev = vec![];
    let mut cur = vec![];
    let mut j = 0;
    while j <= n {
        prev.push(0);
        cur.push(0);
        j = j + 1;
    }
    let mut i = 1;
    while i <= m {
        cur[0] = 0;
        let mut j = 1;
        while j <= n {
            if text1.char_at(i - 1) == text2.char_at(j - 1) {
                cur[j] = prev[j - 1] + 1;
            } else {
                cur[j] = if prev[j] > cur[j - 1] { prev[j] } else { cur[j - 1] };
            }
            j = j + 1;
        }
        // Swap rows
        let tmp = prev;
        prev = cur;
        cur = tmp;
        i = i + 1;
    }
    prev[n]
}

#[test]
fn test_example() {
    assert_eq!(longest_common_subsequence("abcde", "ace"), 3);
}

#[test]
fn test_no_common() {
    assert_eq!(longest_common_subsequence("abc", "def"), 0);
}

#[test]
fn test_empty() {
    assert_eq!(longest_common_subsequence("abc", ""), 0);
}
