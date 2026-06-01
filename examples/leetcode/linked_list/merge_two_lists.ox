// === Problem: Merge Two Sorted Lists (LeetCode #21) ===
// Given the heads of two sorted linked lists, merge them into one
// sorted linked list and return its head.
//
// === Pattern: Linked List (Two-Pointer Merge) ===
// Compare the two current nodes. Take the smaller one, advance that list,
// and append to the result. Works exactly like the merge step of merge sort.
//
// === Intuition ===
// Compare l1.value and l2.value. The smaller node becomes part of the result.
// Recursively merge the rest. This avoids the dummy-head copy issue.
//
// === Pattern Recognition ===
// - "Merge two sorted lists" → two-pointer merge (like merge sort)
// - Recursive approach: pick smaller head, merge the rest
// - "Sorted" is the key that enables O(n+m) merging
//
// === Tips ===
// - Base case: if one list is None, return the other
// - Compare val, pick smaller, recursively merge rest
// - Simple and avoids state management issues

struct ListNode {
    value: Int,
    next: Option,
}

fn main() {
    var l1 = ListNode::new(1);
    var n2 = ListNode::new(2);
    val n4 = ListNode::new(4);
    n2.next = Some(n4);
    l1.next = Some(n2);

    var l2 = ListNode::new(1);
    var n3 = ListNode::new(3);
    val n4b = ListNode::new(4);
    n3.next = Some(n4b);
    l2.next = Some(n3);

    val merged = merge_two_lists(Some(l1), Some(l2));
    print_list(merged);
}

fn print_list(head: Option) {
    var curr = head;
    while val Some(node) = curr {
        io::print("{} ", node.value);
        curr = node.next;
    }
    io::println("");
}

fn merge_two_lists(l1: Option, l2: Option) -> Option {
    if l1.is_none() {
        return l2;
    }
    if l2.is_none() {
        return l1;
    }
    var node_a = l1.unwrap();
    var node_b = l2.unwrap();
    if node_a.value <= node_b.value {
        val rest_a = node_a.next;
        node_a.next = merge_two_lists(rest_a, Some(node_b));
        Some(node_a)
    } else {
        val rest_b = node_b.next;
        node_b.next = merge_two_lists(Some(node_a), rest_b);
        Some(node_b)
    }
}

#[test]
fn test_merge_basic() {
    var l1 = ListNode::new(1);
    val n2 = ListNode::new(3);
    l1.next = Some(n2);

    var l2 = ListNode::new(2);
    val n4 = ListNode::new(4);
    l2.next = Some(n4);

    val result = merge_two_lists(Some(l1), Some(l2));
    assert::eq(result.unwrap().value, 1);
}

#[test]
fn test_merge_one_empty() {
    val l1 = ListNode::new(5);
    val result = merge_two_lists(Some(l1), None);
    assert::eq(result.unwrap().value, 5);
}
