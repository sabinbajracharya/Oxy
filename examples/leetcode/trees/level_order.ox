// === Problem: Binary Tree Level Order Traversal (LeetCode #102) ===
// Return the level order traversal (BFS) of a binary tree's node values.
// Each level is a separate Vec.
//
// === Pattern: BFS with Queue ===
// Use a VecDeque as a queue. Process nodes level by level. For each level,
// record the current queue size (number of nodes at that level), drain
// that many nodes, and enqueue their children.
//
// === Intuition ===
// BFS processes nodes level by level. Use a queue: enqueue root, then
// while the queue is not empty, process all nodes at the current level
// (by tracking queue size before the inner loop).
//
// === Pattern Recognition ===
// - "Level order" → BFS with queue
// - "Process tree by level" → queue + level size tracking
// - Alternative: DFS with depth tracking
//
// === Tips ===
// - VecDeque::new() creates an empty deque
// - push_back to enqueue, pop_front to dequeue
// - Track level_size before processing each level

struct TreeNode {
    val: int,
    left: Option,
    right: Option,
}

fn main() {
    let mut root = TreeNode::new(3);
    root.left = Some(TreeNode::new(9));
    let mut r = TreeNode::new(20);
    r.left = Some(TreeNode::new(15));
    r.right = Some(TreeNode::new(7));
    root.right = Some(r);
    let levels = level_order(Some(root));
    for level in levels {
        println!("{:?}", level);
    }
}

fn level_order(root: Option) -> Vec {
    let mut result = vec![];
    if root.is_none() {
        return result;
    }
    let mut queue = VecDeque::new();
    queue.push_back(root.unwrap());
    while !queue.is_empty() {
        let level_size = queue.len();
        let mut level = vec![];
        for _i in 0..level_size {
            let mut node = queue.pop_front().unwrap();
            level.push(node.val);
            if let Some(left) = node.left {
                queue.push_back(left);
            }
            if let Some(right) = node.right {
                queue.push_back(right);
            }
        }
        result.push(level);
    }
    result
}

#[test]
fn test_example() {
    let mut root = TreeNode::new(3);
    root.left = Some(TreeNode::new(9));
    let mut r = TreeNode::new(20);
    r.left = Some(TreeNode::new(15));
    r.right = Some(TreeNode::new(7));
    root.right = Some(r);
    let result = level_order(Some(root));
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], vec![3]);
    assert_eq!(result[1], vec![9, 20]);
    assert_eq!(result[2], vec![15, 7]);
}

#[test]
fn test_empty() {
    assert_eq!(level_order(None).len(), 0);
}
