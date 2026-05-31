// === Problem: Two Sum (LeetCode #1) ===
// Given an array of integers `nums` and an integer `target`, return the
// indices of the two numbers that add up to `target`.
//
// Constraints: exactly one solution, no duplicate use of same index.
//
// === Pattern: Hash Map ===
// Whenever you hear "find two elements that sum to X" or "find a complement,"
// think Map. A map turns an O(n²) nested scan into O(n) by memorizing
// every value you've seen.
//
// === Intuition ===
// For each element A at index i, check if (target - A) is already in the map.
// If yes, you've found the pair. If no, store A → i and keep going.
//
// === Pattern Recognition ===
// Look for these clues:
//   - "Find two numbers that..." → complement pattern
//   - O(n²) brute force exists → can we memoize?
//   - The value you're looking for depends on the current element
//
// === Tips ===
// - Use `Map::new()` with integer keys — no hashing issues
// - Return `Option<(Int, Int)>` so the caller can handle "not found"
// - The map stores value → index so we can retrieve positions directly

fn main() {
    val nums = [2, 7, 11, 15];
    val target = 9;
    match two_sum(nums, target) {
        Some((i, j)) => println("Found: indices {} and {}", i, j),
        None => println("No solution"),
    }
}

fn two_sum(nums: List, target: Int) -> Option {
    var seen = Map::new();
    for (i, num) in nums.iter().enumerate() {
        val complement = target - num;
        if val Some(j) = seen.get(complement) {
            return Some((j, i));
        }
        seen.insert(num, i);
    }
    None
}

// --- Tests ---

#[test]
fn test_basic_case() {
    val nums = [2, 7, 11, 15];
    val result = two_sum(nums, 9);
    assert_eq(result, Some((0, 1)));
}

#[test]
fn test_reversed_order() {
    val nums = [3, 3, 4, 1];
    val result = two_sum(nums, 6);
    assert_eq(result, Some((0, 1)));
}

#[test]
fn test_no_solution() {
    val nums = [1, 2, 3];
    val result = two_sum(nums, 10);
    assert(result.is_none());
}

#[test]
fn test_negative_numbers() {
    val nums = [-3, 4, 3, 90];
    val result = two_sum(nums, 0);
    assert_eq(result, Some((0, 2)));
}
