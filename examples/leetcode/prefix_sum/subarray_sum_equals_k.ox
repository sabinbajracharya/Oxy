// === Problem: Subarray Sum Equals K (LeetCode #560) ===
// Given an array of integers and an integer k, return the total number
// of contiguous subarrays that sum to k.
//
// === Pattern: Prefix Sum + Map ===
// Let prefix[i] = sum of nums[0..i]. Then subarray sum(i,j] = prefix[j] - prefix[i].
// We want prefix[j] - prefix[i] = k, i.e., prefix[i] = prefix[j] - k.
// Count how many times each prefix sum has been seen.
//
// === Intuition ===
// As we scan, maintain a running sum. At each step, check if (sum - k)
// exists in the Map of previously seen prefix sums. If it's been
// seen count times, that many subarrays ending here sum to k.
//
// === Pattern Recognition ===
// - "Subarray sum equals K" → prefix sum + Map
// - "Count subarrays with property" → track cumulative sums
// - Works with negative numbers (unlike sliding window)
//
// === Tips ===
// - Initialize Map with {0: 1} for subarrays starting at index 0
// - Works with negative numbers — prefix sum handles it
// - O(n) time, O(n) space

fn main() {
    val nums = [1, 1, 1];
    io::println("{}", subarray_sum(nums, 2));
}

fn subarray_sum(nums: List, k: Int) -> Int {
    var count = 0;
    var sum = 0;
    var seen = Map::new();
    seen.insert(0, 1);
    for num in nums {
        sum = sum + num;
        val target = sum - k;
        count = count + seen.get(target).unwrap_or(0);
        val existing = seen.get(sum).unwrap_or(0);
        seen.insert(sum, existing + 1);
    }
    count
}

#[test]
fn test_example() {
    assert::eq(subarray_sum([1, 1, 1], 2), 2);
}

#[test]
fn test_negative() {
    assert::eq(subarray_sum([1, -1, 0], 0), 3);
}

#[test]
fn test_single_element() {
    assert::eq(subarray_sum([5], 5), 1);
    assert::eq(subarray_sum([5], 3), 0);
}
