// === Problem: Kth Largest Element in an Array (LeetCode #215) ===
// Given an integer array nums and an integer k, return the kth largest
// element (not the kth distinct element).
//
// === Pattern: Heap (Priority Queue) / Quickselect ===
// Use a BinaryHeap (max-heap) to extract the kth largest. Push all
// elements in, pop k times. Or use quickselect for O(n) average.
//
// === Intuition ===
// A max-heap keeps the largest element at the top. Pop k-1 times to
// reach the kth largest. Alternatively, use a min-heap of size k:
// if an element is larger than the heap's minimum, push it and pop
// the smallest.
//
// === Pattern Recognition ===
// - "Kth largest/smallest" → heap or quickselect
// - Heap: O(n log k) with min-heap of size k
// - Quickselect: O(n) average, O(n²) worst
//
// === Tips ===
// - BinaryHeap::new() creates a max-heap
// - Use .push() to add, .pop() to remove max
// - For min-heap behavior, negate values

fn main() {
    val nums = [3, 2, 1, 5, 6, 4];
    println("{}", find_kth_largest(nums, 2));
}

fn find_kth_largest(nums: List, k: Int) -> Int {
    var heap = BinaryHeap::new();
    for num in nums {
        heap.push(num);
    }
    var i = 1;
    while i < k {
        heap.pop();
        i = i + 1;
    }
    heap.pop().unwrap_or(-1)
}

#[test]
fn test_example() {
    assert_eq(find_kth_largest([3, 2, 1, 5, 6, 4], 2), 5);
}

#[test]
fn test_single_element() {
    assert_eq(find_kth_largest([1], 1), 1);
}

#[test]
fn test_all_same() {
    assert_eq(find_kth_largest([7, 7, 7, 7], 2), 7);
}
