// === Problem: Single Number (LeetCode #136) ===
// Given a non-empty array where every element appears twice except one,
// find that single element. O(n) time, O(1) space.
//
// === Pattern: Bit Manipulation (XOR) ===
// XOR of two identical numbers is 0. XOR of a number with 0 is the number.
// XOR is commutative and associative. Therefore, XORing all elements
// cancels pairs, leaving the single number.
//
// === Intuition ===
// a ^ a = 0, a ^ 0 = a, a ^ b ^ a = b.
// XOR every element. Duplicates cancel out. The result is the single number.
//
// === Pattern Recognition ===
// - "Every element twice except one" → XOR
// - "Find the odd one out" where others appear in pairs → XOR
// - XOR properties: commutative, associative, self-inverse
//
// === Tips ===
// - The bitwise XOR operator in Oxy is `^` (same as Rust)
// - No extra space needed — use a single accumulator
// - Works for any number of duplicates as long as count is even

fn main() {
    val nums = [4, 1, 2, 1, 2];
    println("{}", single_number(nums));
}

fn single_number(nums: List) -> Int {
    var result = 0;
    for num in nums {
        result = result ^ num;
    }
    result
}

#[test]
fn test_example() {
    assert_eq(single_number([2, 2, 1]), 1);
    assert_eq(single_number([4, 1, 2, 1, 2]), 4);
}

#[test]
fn test_single_element() {
    assert_eq(single_number([1]), 1);
}
