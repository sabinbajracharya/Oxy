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
    value: Int,
    next: Option,
}

fn main() {
    var n1 = ListNode::new(1);
    var n2 = ListNode::new(2);
    val n3 = ListNode::new(3);
    n2.next = Some(n3);
    n1.next = Some(n2);

    val head = reverse_list(Some(n1));
    print_list(head);
}

fn print_list(head: Option) {
    var curr = head;
    while val Some(node) = curr {
        io::print("{} ", node.value);
        curr = node.next;
    }
    io::println("");
}

fn reverse_list(head: Option) -> Option {
    var prev = None;
    var curr = head;
    while val Some(node) = curr {
        var node = node;
        val next = node.next;
        node.next = prev;
        prev = Some(node);
        curr = next;
    }
    prev
}

#[test]
fn test_reverse_empty() {
    assert::true(reverse_list(None).is_none());
}

#[test]
fn test_reverse_single() {
    val n = ListNode::new(42);
    val result = reverse_list(Some(n));
    assert::eq(result.unwrap().value, 42);
    assert::true(result.unwrap().next.is_none());
}

#[test]
fn test_reverse_multiple() {
    var n1 = ListNode::new(1);
    var n2 = ListNode::new(2);
    val n3 = ListNode::new(3);
    n2.next = Some(n3);
    n1.next = Some(n2);
    val result = reverse_list(Some(n1));
    assert::eq(result.unwrap().value, 3);
    val r2 = result.unwrap().next.unwrap();
    assert::eq(r2.value, 2);
}
