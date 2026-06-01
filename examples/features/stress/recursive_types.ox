// === STRESS: recursive types and recursive methods ===

// --- recursive enum: linked list ---
enum List {
    Empty,
    Node(Int, List<List>),
}

fn list_len(l: List) -> Int {
    match l {
        List::Empty => 0,
        List::Node(_, rest) => 1 + list_len(rest[0].clone()),
    }
}

fn list_sum(l: List) -> Int {
    match l {
        List::Empty => 0,
        List::Node(v, rest) => v + list_sum(rest[0].clone()),
    }
}

#[test]
fn test_linked_list_empty() {
    val l = List::Empty;
    assert::eq(list_len(l), 0);
}

#[test]
fn test_linked_list_three_nodes() {
    val l = List::Node(1, [List::Node(2, [List::Node(3, [List::Empty])])]);
    assert::eq(list_sum(l), 6);
}

// --- recursive enum: binary tree ---
enum Tree {
    Leaf,
    Node(Int, List<Tree>),
}

fn tree_sum(t: Tree) -> Int {
    match t {
        Tree::Leaf => 0,
        Tree::Node(v, children) => {
            var total = v;
            for c in children {
                total = total + tree_sum(c);
            }
            total
        }
    }
}

#[test]
fn test_tree_single_node() {
    val t = Tree::Node(5, []);
    assert::eq(tree_sum(t), 5);
}

#[test]
fn test_tree_with_children() {
    val t = Tree::Node(1, [Tree::Node(2, []), Tree::Node(3, [])]);
    assert::eq(tree_sum(t), 6);
}

#[test]
fn test_tree_deep() {
    val t = Tree::Node(1, [
        Tree::Node(2, [
            Tree::Node(4, []),
            Tree::Node(5, []),
        ]),
        Tree::Node(3, [
            Tree::Node(6, []),
        ]),
    ]);
    assert::eq(tree_sum(t), 21);
}

// --- recursive fn: factorial ---
fn factorial(n: Int) -> Int {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

#[test]
fn test_factorial_zero() { assert::eq(factorial(0), 1); }
#[test]
fn test_factorial_one() { assert::eq(factorial(1), 1); }
#[test]
fn test_factorial_five() { assert::eq(factorial(5), 120); }
#[test]
fn test_factorial_ten() { assert::eq(factorial(10), 3628800); }

// --- mutual recursion ---
fn is_even(n: Int) -> bool {
    if n == 0 { true } else { is_odd(n - 1) }
}
fn is_odd(n: Int) -> bool {
    if n == 0 { false } else { is_even(n - 1) }
}

#[test]
fn test_mutual_recursion_even() {
    assert::eq(is_even(0), true);
    assert::eq(is_even(4), true);
    assert::eq(is_even(7), false);
}
#[test]
fn test_mutual_recursion_odd() {
    assert::eq(is_odd(0), false);
    assert::eq(is_odd(3), true);
    assert::eq(is_odd(6), false);
}

// --- recursive fn with accumulator ---
fn sum_to(n: Int) -> Int {
    fn helper(n: Int, acc: Int) -> Int {
        if n <= 0 { acc } else { helper(n - 1, acc + n) }
    }
    helper(n, 0)
}

#[test]
fn test_sum_to_ten() { assert::eq(sum_to(10), 55); }
#[test]
fn test_sum_to_zero() { assert::eq(sum_to(0), 0); }
#[test]
fn test_sum_to_hundred() { assert::eq(sum_to(100), 5050); }

// --- deep recursion (small enough to fit on the stack) ---
fn countdown(n: Int) -> Int {
    if n <= 0 { 0 } else { countdown(n - 1) + 1 }
}

#[test]
fn test_countdown() {
    assert::eq(countdown(100), 100);
}

// --- recursive struct via List (no direct self-ref allowed) ---
struct TreeNode {
    value: Int,
    children: List<TreeNode>,
}

fn count_nodes(t: TreeNode) -> Int {
    var n = 1;
    for c in t.children {
        n = n + count_nodes(c);
    }
    n
}

#[test]
fn test_tree_node_count() {
    val t = TreeNode {
        value: 1,
        children: [
            TreeNode { value: 2, children: [] },
            TreeNode { value: 3, children: [
                TreeNode { value: 4, children: [] },
            ]},
        ],
    };
    assert::eq(count_nodes(t), 4);
}
