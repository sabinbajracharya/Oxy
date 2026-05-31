// === Problem: Reverse Linked List (LeetCode #206) ===
// Given the head of a singly linked list, reverse the list in-place
// and return the new head.
//
// === Pattern: Linked List (Iterative Reversal) ===
// Use three pointers: prev, curr, next. At each step, redirect curr.next
// to prev, then advance all three pointers. prev becomes the new head.
//
// === Intuition ===
// Walk through the list, reversing each pointer one at a time.
// Before we overwrite curr.next, save it as `next` so we can advance.
// After the loop, prev is the last node we processed = new head.
//
// === Pattern Recognition ===
// - "Reverse linked list" → prev/curr/next three-pointer dance
// - In-place linked list modification → pointer manipulation
// - Recursive alternative: reverse from the tail up
//
// === Tips ===
// - Save curr.next before overwriting it
// - Empty list → return None
// - Single node list → return itself

struct ListNode {
    val: Int,
    next: Option,
}

fn main() {
    let mut n1 = ListNode::new(1);
    let mut n2 = ListNode::new(2);
    let n3 = ListNode::new(3);
    n2.next = Some(n3);
    n1.next = Some(n2);

    let head = reverse_list(Some(n1));
    print_list(head);
}

fn print_list(head: Option) {
    let mut curr = head;
    while let Some(node) = curr {
        print("{} ", node.val);
        curr = node.next;
    }
    println("");
}

fn reverse_list(head: Option) -> Option {
    let mut prev = None;
    let mut curr = head;
    while let Some(node) = curr {
        let mut node = node;
        let next = node.next;
        node.next = prev;
        prev = Some(node);
        curr = next;
    }
    prev
}

#[test]
fn test_reverse_empty() {
    assert(reverse_list(None).is_none());
}

#[test]
fn test_reverse_single() {
    let n = ListNode::new(42);
    let result = reverse_list(Some(n));
    assert_eq(result.unwrap().val, 42);
    assert(result.unwrap().next.is_none());
}

#[test]
fn test_reverse_multiple() {
    let mut n1 = ListNode::new(1);
    let mut n2 = ListNode::new(2);
    let n3 = ListNode::new(3);
    n2.next = Some(n3);
    n1.next = Some(n2);
    let result = reverse_list(Some(n1));
    assert_eq(result.unwrap().val, 3);
    let r2 = result.unwrap().next.unwrap();
    assert_eq(r2.val, 2);
}
