// === Problem: Container With Most Water (LeetCode #11) ===
// Given an array of heights, find two lines that together with the x-axis
// form a container that holds the most water. Return the max area.
//
// === Pattern: Two Pointers ===
// Start at both ends. The area is width × min(height[left], height[right]).
// Move the pointer with the SHORTER height inward (because the shorter
// line is the bottleneck — moving the taller one can never increase area).
//
// === Intuition ===
// Width decreases as we move inward, so the only way to increase area is
// to find a taller line to replace the shorter one. Always move the
// shorter line inward — this is the key insight.
//
// === Pattern Recognition ===
// - "Container" / "max area between two elements" → two pointers
// - "Move the bottleneck" — always move the shorter line
// - Optimal solution is greedy two-pointer sweep
//
// === Tips ===
// - Area = (right - left) × min(height[left], height[right])
// - Only move the smaller height pointer
// - O(n) time, O(1) space

fn main() {
    let heights = vec(1, 8, 6, 2, 5, 4, 8, 3, 7);
    println("{}", max_area(heights));
}

fn max_area(height: Vec) -> int {
    let mut left = 0;
    let mut right = height.len() - 1;
    let mut max_water = 0;
    while left < right {
        let h_left = height[left];
        let h_right = height[right];
        let h = if h_left < h_right { h_left } else { h_right };
        let area = (right - left) * h;
        if area > max_water {
            max_water = area;
        }
        if h_left < h_right {
            left = left + 1;
        } else {
            right = right - 1;
        }
    }
    max_water
}

#[test]
fn test_example() {
    assert_eq(max_area(vec(1, 8, 6, 2, 5, 4, 8, 3, 7)), 49);
}

#[test]
fn test_two_elements() {
    assert_eq(max_area(vec(1, 1)), 1);
}

#[test]
fn test_descending() {
    assert_eq(max_area(vec(5, 4, 3, 2, 1)), 6);
}
