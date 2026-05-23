// === Problem: Subarray Sum Equals K (LeetCode #560) ===
// Given an array of integers and an integer k, return the total number
// of contiguous subarrays that sum to k.
//
// === Pattern: Prefix Sum + HashMap ===
// Let prefix[i] = sum of nums[0..i]. Then subarray sum(i,j] = prefix[j] - prefix[i].
// We want prefix[j] - prefix[i] = k, i.e., prefix[i] = prefix[j] - k.
// Count how many times each prefix sum has been seen.
//
// === Intuition ===
// As we scan, maintain a running sum. At each step, check if (sum - k)
// exists in the HashMap of previously seen prefix sums. If it's been
// seen count times, that many subarrays ending here sum to k.
//
// === Pattern Recognition ===
// - "Subarray sum equals K" → prefix sum + HashMap
// - "Count subarrays with property" → track cumulative sums
// - Works with negative numbers (unlike sliding window)
//
// === Tips ===
// - Initialize HashMap with {0: 1} for subarrays starting at index 0
// - Works with negative numbers — prefix sum handles it
// - O(n) time, O(n) space

fn main() {
    let nums = vec![1, 1, 1];
    println!("{}", subarray_sum(nums, 2));
}

fn subarray_sum(nums: Vec, k: int) -> int {
    let mut count = 0;
    let mut sum = 0;
    let mut seen = HashMap::new();
    seen.insert(0, 1);
    for num in nums {
        sum = sum + num;
        let target = sum - k;
        count = count + seen.get(target).unwrap_or(0);
        let existing = seen.get(sum).unwrap_or(0);
        seen.insert(sum, existing + 1);
    }
    count
}

#[test]
fn test_example() {
    assert_eq!(subarray_sum(vec![1, 1, 1], 2), 2);
}

#[test]
fn test_negative() {
    assert_eq!(subarray_sum(vec![1, -1, 0], 0), 3);
}

#[test]
fn test_single_element() {
    assert_eq!(subarray_sum(vec![5], 5), 1);
    assert_eq!(subarray_sum(vec![5], 3), 0);
}
