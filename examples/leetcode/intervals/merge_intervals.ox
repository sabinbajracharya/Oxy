// === Problem: Merge Intervals (LeetCode #56) ===
// Given an array of intervals where intervals[i] = [start_i, end_i],
// merge all overlapping intervals and return the result.
//
// === Pattern: Sorting + Linear Scan ===
// Sort by start time. Then iterate: if the current interval starts before
// or at the end of the previous merged interval, extend the merge.
// Otherwise, push the previous interval and start a new one.
//
// === Intuition ===
// Sort makes overlapping intervals adjacent. Two intervals [a,b] and [c,d]
// overlap iff c <= b. The merged interval is [min(a,c), max(b,d)].
//
// === Pattern Recognition ===
// - "Merge overlapping ranges" → sort by start + linear scan
// - Meeting rooms, interval intersection all use this pattern
// - Sort by start time, then build merged list
//
// === Tips ===
// - sort_by_key on the first element of each interval
// - Overlap condition: new_start <= current_end
// - Empty input returns empty output

fn main() {
    let intervals = list(list(1, 3), list(2, 6), list(8, 10), list(15, 18));
    let merged = merge(intervals);
    for m in merged {
        println("{:?}", m);
    }
}

fn merge(intervals: List) -> List {
    let n = intervals.len();
    if n <= 1 {
        return intervals;
    }
    // Sort by start time
    intervals.sort_by_key(|iv| iv[0]);
    let mut result = list();
    let mut current = intervals[0];
    let mut i = 1;
    while i < n {
        let next = intervals[i];
        if next[0] <= current[1] {
            // Overlapping — merge by extending end
            if next[1] > current[1] {
                current[1] = next[1];
            }
        } else {
            result.push(current);
            current = next;
        }
        i = i + 1;
    }
    result.push(current);
    result
}

#[test]
fn test_example() {
    let intervals = list(list(1, 3), list(2, 6), list(8, 10), list(15, 18));
    let result = merge(intervals);
    assert_eq(result, list(list(1, 6), list(8, 10), list(15, 18)));
}

#[test]
fn test_no_overlap() {
    let intervals = list(list(1, 2), list(3, 4));
    assert_eq(merge(intervals), list(list(1, 2), list(3, 4)));
}

#[test]
fn test_contained() {
    let intervals = list(list(1, 4), list(2, 3));
    assert_eq(merge(intervals), list(list(1, 4)));
}
