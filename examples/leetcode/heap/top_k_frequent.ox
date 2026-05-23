// === Problem: Top K Frequent Elements (LeetCode #347) ===
// Given an integer array and an integer k, return the k most frequent
// elements. Answer is guaranteed to be unique.
//
// === Pattern: Heap (Priority Queue) ===
// Count frequencies with a HashMap, then use a min-heap of size k to
// track the top-k elements by frequency. Alternatively, sort by frequency
// and take the top k.
//
// === Intuition ===
// First pass: count frequencies (O(n)). Second pass: find top k.
// A min-heap of size k ensures we only keep the k most frequent elements.
// When a new element has higher frequency than the heap minimum, swap.
//
// === Pattern Recognition ===
// - "Top K" / "K most frequent" → heap or bucket sort
// - "K largest/smallest by some metric" → min-heap/max-heap of size k
// - Heap of size k gives O(n log k) instead of O(n log n) for full sort
//
// === Tips ===
// - BinaryHeap is a max-heap; negate values for min-heap behavior
// - Or sort by frequency and take last k
// - HashMap for counting: .get_or(key, 0) pattern

fn main() {
    let nums = vec![1, 1, 1, 2, 2, 3];
    println!("{:?}", top_k_frequent(nums, 2));
}

fn top_k_frequent(nums: Vec, k: int) -> Vec {
    // Count frequencies
    let mut counts = HashMap::new();
    for num in nums {
        let count = counts.get(num).unwrap_or(0i64);
        counts.insert(num, count + 1);
    }
    // Collect as (freq, num) pairs and sort by frequency descending
    let mut pairs = vec![];
    for (num, freq) in counts {
        pairs.push((freq, num));
    }
    pairs.sort_by(|a, b| {
        let (fa, _) = a;
        let (fb, _) = b;
        if fa > fb { -1 } else if fa < fb { 1 } else { 0 }
    });
    let mut result = vec![];
    let mut i = 0i64;
    while i < k && i < pairs.len() {
        let (_, num) = pairs[i];
        result.push(num);
        i = i + 1;
    }
    result
}

#[test]
fn test_example() {
    let nums = vec![1, 1, 1, 2, 2, 3];
    let result = top_k_frequent(nums, 2);
    assert_eq!(result.len(), 2);
    assert!(result.contains(1));
    assert!(result.contains(2));
}

#[test]
fn test_single() {
    assert_eq!(top_k_frequent(vec![1], 1), vec![1]);
}
