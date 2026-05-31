// === Problem: Maximum Depth of Binary Tree (LeetCode #104) ===
// Given the root of a binary tree, return its maximum depth.
//
// === Pattern: Tree DFS (Recursive) ===
// The depth of a tree is 1 + max(depth(left), depth(right)).
// Base case: None → depth 0.
//
// === Intuition ===
// Depth = 1 + max(left_depth, right_depth). This is naturally recursive.
// Each node's depth is one more than the deeper of its two children.
//
// === Pattern Recognition ===
// - "Depth/height of tree" → recursive DFS
// - "Max/min/check property" → post-order traversal
// - Tree problems are usually recursive unless level-order is needed
//
// === Tips ===
// - Empty tree has depth 0
// - Recursion limit of 1024 is enough for LeetCode trees (max ~10^4 nodes)
// - DFS uses O(h) stack space, BFS would use O(w) queue space

struct TreeNode {
    val: Int,
    left: Option,
    right: Option,
}

fn main() {
    let mut root = TreeNode::new(3);
    let l = TreeNode::new(9);
    let mut r = TreeNode::new(20);
    r.left = Some(TreeNode::new(15));
    r.right = Some(TreeNode::new(7));
    root.left = Some(l);
    root.right = Some(r);
    println("{}", max_depth(Some(root)));
}

fn max_depth(root: Option) -> Int {
    if let Some(node) = root {
        let left = max_depth(node.left);
        let right = max_depth(node.right);
        1 + (if left > right { left } else { right })
    } else {
        0
    }
}

#[test]
fn test_example() {
    let mut root = TreeNode::new(3);
    let l = TreeNode::new(9);
    let mut r = TreeNode::new(20);
    r.left = Some(TreeNode::new(15));
    r.right = Some(TreeNode::new(7));
    root.left = Some(l);
    root.right = Some(r);
    assert_eq(max_depth(Some(root)), 3);
}

#[test]
fn test_empty() {
    assert_eq(max_depth(None), 0);
}

#[test]
fn test_single_node() {
    assert_eq(max_depth(Some(TreeNode::new(1))), 1);
}
