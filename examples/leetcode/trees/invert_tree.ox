// === Problem: Invert Binary Tree (LeetCode #226) ===
// Given the root of a binary tree, invert it (swap left and right children)
// and return the new root.
//
// === Pattern: Tree DFS (Pre-order) ===
// At each node, swap left and right children, then recursively invert
// both subtrees. Or invert children first, then swap.
//
// === Intuition ===
// Inverting a tree means swapping every node's left and right children.
// This is a mirror image. Process: swap(left, right), then invert(left),
// invert(right).
//
// === Pattern Recognition ===
// - "Invert/mirror tree" → swap children + recurse
// - "Transform tree" → pre-order or post-order traversal
// - Simple recursive pattern
//
// === Tips ===
// - Base case: None → return None
// - Swap, then recurse on both sides
// - Return the node itself (now inverted)

struct TreeNode {
    val: int,
    left: Option,
    right: Option,
}

fn main() {
    let mut root = TreeNode::new(4);
    let mut l = TreeNode::new(2);
    l.left = Some(TreeNode::new(1));
    l.right = Some(TreeNode::new(3));
    let mut r = TreeNode::new(7);
    r.left = Some(TreeNode::new(6));
    r.right = Some(TreeNode::new(9));
    root.left = Some(l);
    root.right = Some(r);
    let inverted = invert_tree(Some(root));
    print_tree(inverted);
}

fn print_tree(root: Option) {
    if let Some(node) = root {
        print!("{} ", node.val);
        print_tree(node.left);
        print_tree(node.right);
    }
}

fn invert_tree(root: Option) -> Option {
    if let Some(node) = root {
        let mut node = node;
        let left = node.left;
        let right = node.right;
        node.left = invert_tree(right);
        node.right = invert_tree(left);
        Some(node)
    } else {
        None
    }
}

#[test]
fn test_invert() {
    let mut root = TreeNode::new(4);
    let mut l = TreeNode::new(2);
    l.left = Some(TreeNode::new(1));
    l.right = Some(TreeNode::new(3));
    let mut r = TreeNode::new(7);
    r.left = Some(TreeNode::new(6));
    r.right = Some(TreeNode::new(9));
    root.left = Some(l);
    root.right = Some(r);
    let result = invert_tree(Some(root));
    let node = result.unwrap();
    assert_eq!(node.val, 4);
    assert_eq!(node.left.unwrap().val, 7);
    assert_eq!(node.right.unwrap().val, 2);
}

#[test]
fn test_empty() {
    assert!(invert_tree(None).is_none());
}
