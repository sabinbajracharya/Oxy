// === Problem: Longest Consecutive Sequence (LeetCode #128) ===
// Given an unsorted array of integers nums, return the length of the
// longest consecutive elements sequence. Must run in O(n).
//
// === Pattern: Hash Set for O(1) Lookup ===
// Put all numbers in a Set. Only start counting from a number that
// has NO predecessor (num - 1 not in set). This ensures each streak is
// counted exactly once, achieving O(n).
//
// === Intuition ===
// If we start a streak at every number, we get O(n²). Key insight:
// only start at the beginning of a streak. A number n starts a streak
// iff n - 1 is not in the set.
//
// === Pattern Recognition ===
// - "Longest consecutive" → Set + streak expansion
// - "O(n)" with nested lookups → amortized by set membership check
// - "Sequence" without order requirement → set eliminates sorting
//
// === Tips ===
// - Set::contains() is O(1)
// - Don't sort — that's O(n log n)
// - The while loop runs at most n times total across all iterations

fn main() {
    let nums = [100, 4, 200, 1, 3, 2];
    println("{}", longest_consecutive(nums));
}

fn longest_consecutive(nums: List) -> Int {
    let mut set = Set::new();
    for n in nums {
        set.insert(n);
    }
    let mut longest = 0;
    for n in set.to_vec() {
        if !set.contains(n - 1) {
            let mut current = n;
            let mut streak = 1;
            while set.contains(current + 1) {
                current = current + 1;
                streak = streak + 1;
            }
            if streak > longest {
                longest = streak;
            }
        }
    }
    longest
}

#[test]
fn test_example() {
    let nums = [100, 4, 200, 1, 3, 2];
    assert_eq(longest_consecutive(nums), 4);
}

#[test]
fn test_empty() {
    assert_eq(longest_consecutive([]), 0);
}

#[test]
fn test_single() {
    assert_eq(longest_consecutive([5]), 1);
}

#[test]
fn test_duplicates() {
    let nums = [1, 2, 0, 1, 3];
    assert_eq(longest_consecutive(nums), 4);
}
