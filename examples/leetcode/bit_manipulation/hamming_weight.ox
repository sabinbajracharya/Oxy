// === Problem: Number of 1 Bits (LeetCode #191) ===
// Given an unsigned integer, return the number of '1' bits in its binary
// representation (Hamming weight).
//
// === Pattern: Bit Manipulation (Population Count) ===
// Repeatedly check the lowest bit (n & 1) and right-shift (n >> 1).
// Or use the trick: n & (n - 1) clears the lowest set bit.
//
// === Intuition ===
// n & 1 returns the lowest bit. Add it to count, then n = n >> 1.
// Repeat until n == 0. The n & (n - 1) trick is faster: it clears the
// lowest set bit in one operation, iterating only once per set bit.
//
// === Pattern Recognition ===
// - "Count set bits" → bit manipulation loop
// - n & 1 isolates the LSB; n >> 1 shifts right
// - n & (n - 1) clears the lowest 1-bit (Brian Kernighan's algorithm)
//
// === Tips ===
// - Use n & 1 to check LSB, n >> 1 to shift
// - The `&` and `>>` operators work the same as in Rust
// - Handle 0 — returns 0

fn main() {
    println!("{}", hamming_weight(11));
    println!("{}", hamming_weight(128));
}

fn hamming_weight(n: i64) -> i64 {
    let mut count = 0i64;
    let mut x = n;
    while x != 0 {
        count = count + (x & 1);
        x = x >> 1;
    }
    count
}

#[test]
fn test_example() {
    assert_eq!(hamming_weight(11), 3); // 1011
}

#[test]
fn test_power_of_two() {
    assert_eq!(hamming_weight(128), 1); // 10000000
}

#[test]
fn test_zero() {
    assert_eq!(hamming_weight(0), 0);
}
