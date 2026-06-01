// === Problem: Top K Frequent Elements (LeetCode #347) ===
// Given an integer array and an integer k, return the k most frequent
// elements. Answer is guaranteed to be unique.
//
// === Pattern: Heap (Priority Queue) ===
// Count frequencies with a Map, then use a min-heap of size k to
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
// - Map for counting: .get_or(key, 0) pattern

fn main() {
    val nums = [1, 1, 1, 2, 2, 3];
    io::println("{:?}", top_k_frequent(nums, 2));
}

fn top_k_frequent(nums: List, k: Int) -> List {
    // Count frequencies
    var counts = Map::new();
    for num in nums {
        val count = counts.get(num).unwrap_or(0);
        counts.insert(num, count + 1);
    }
    // Collect as (freq, num) pairs and sort by frequency descending
    var pairs = [];
    for (num, freq) in counts {
        pairs.push((freq, num));
    }
    pairs.sort_by(|a, b| {
        val (fa, _) = a;
        val (fb, _) = b;
        if fa > fb { -1 } else if fa < fb { 1 } else { 0 }
    });
    var result = [];
    val limit = if k < pairs.len() { k } else { pairs.len() };
    for i in 0..limit {
        val (_, num) = pairs[i];
        result.push(num);
    }
    result
}

#[test]
fn test_example() {
    val nums = [1, 1, 1, 2, 2, 3];
    val result = top_k_frequent(nums, 2);
    assert::eq(result.len(), 2);
    assert::true(result.contains(1));
    assert::true(result.contains(2));
}

#[test]
fn test_single() {
    assert::eq(top_k_frequent([1], 1), [1]);
}
