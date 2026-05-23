// === Problem: Insert Interval (LeetCode #57) ===
// Given non-overlapping intervals sorted by start, insert a new interval
// and merge if necessary. Return the resulting intervals.
//
// === Pattern: Linear Scan with Three Phases ===
// Three phases: (1) add all intervals that end before new starts,
// (2) merge all overlapping intervals, (3) add remaining intervals.
//
// === Intuition ===
// Since the input is already sorted, we don't need to re-sort. Walk through:
// - Before overlap: interval.end < new.start → push as-is
// - During overlap: interval.start <= new.end → extend new interval bounds
// - After overlap: push the merged interval, then push all remaining
//
// === Pattern Recognition ===
// - "Insert into sorted intervals" → three-phase linear scan
// - Already sorted → no need to resort
// - Overlap while interval.start <= new_end
//
// === Tips ===
// - New interval's bounds expand during the merge phase
// - Push the merged interval once, then remaining intervals
// - Empty input → just return [new_interval]

fn main() {
    let intervals = vec![vec![1, 3], vec![6, 9]];
    let new_interval = vec![2, 5];
    let result = insert(intervals, new_interval);
    for r in result {
        println!("{:?}", r);
    }
}

fn insert(intervals: Vec, new_interval: Vec) -> Vec {
    let mut result = vec![];
    let n = intervals.len();
    let mut i = 0;
    // Phase 1: add all before overlap
    while i < n && intervals[i][1] < new_interval[0] {
        result.push(intervals[i]);
        i = i + 1;
    }
    // Phase 2: merge overlapping
    let mut merged = new_interval;
    while i < n && intervals[i][0] <= merged[1] {
        if intervals[i][0] < merged[0] {
            merged[0] = intervals[i][0];
        }
        if intervals[i][1] > merged[1] {
            merged[1] = intervals[i][1];
        }
        i = i + 1;
    }
    result.push(merged);
    // Phase 3: add remaining
    while i < n {
        result.push(intervals[i]);
        i = i + 1;
    }
    result
}

#[test]
fn test_example() {
    let intervals = vec![vec![1, 3], vec![6, 9]];
    let result = insert(intervals, vec![2, 5]);
    assert_eq!(result, vec![vec![1, 5], vec![6, 9]]);
}

#[test]
fn test_no_overlap() {
    let intervals = vec![vec![1, 2], vec![5, 6]];
    let result = insert(intervals, vec![3, 4]);
    assert_eq!(result, vec![vec![1, 2], vec![3, 4], vec![5, 6]]);
}

#[test]
fn test_merge_all() {
    let intervals = vec![vec![1, 3], vec![4, 6]];
    let result = insert(intervals, vec![2, 5]);
    assert_eq!(result, vec![vec![1, 6]]);
}
