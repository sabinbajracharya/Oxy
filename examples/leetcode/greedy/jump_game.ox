// === Problem: Jump Game (LeetCode #55) ===
// Given an array where nums[i] is the max jump length from position i,
// return true if you can reach the last index starting from index 0.
//
// === Pattern: Greedy ===
// Track the farthest index reachable at each step. If at any point
// the current index exceeds the farthest reachable, return false.
//
// === Intuition ===
// At each position i, the farthest we can reach is max(farthest, i + nums[i]).
// If i ever passes farthest, we're stuck. The greedy choice is always to
// take the maximum jump possible.
//
// === Pattern Recognition ===
// - "Can you reach the end?" → track max reachable
// - Each position extends your reach by nums[i]
// - Greedy: the optimal choice is always max jump
//
// === Tips ===
// - farthest = max(farthest, i + nums[i])
// - Return false if i > farthest at any point
// - O(n) time, O(1) space

fn main() {
    io::println("{}", can_jump([2, 3, 1, 1, 4]));
    io::println("{}", can_jump([3, 2, 1, 0, 4]));
}

fn can_jump(nums: List) -> bool {
    var farthest = 0;
    val n = nums.len();
    var i = 0;
    while i < n {
        if i > farthest {
            return false;
        }
        val reach = i + nums[i];
        if reach > farthest {
            farthest = reach;
        }
        i = i + 1;
    }
    true
}

#[test]
fn test_reachable() {
    assert::true(can_jump([2, 3, 1, 1, 4]));
}

#[test]
fn test_unreachable() {
    assert::true(!can_jump([3, 2, 1, 0, 4]));
}

#[test]
fn test_single_element() {
    assert::true(can_jump([0]));
}
