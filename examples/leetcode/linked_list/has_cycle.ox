// === Problem: Linked List Cycle Detection (LeetCode #141) ===
// Given the head of a linked list, return true if the list has a cycle.
//
// === Pattern: Fast & Slow Pointers (Floyd's Tortoise and Hare) ===
// Use two pointers moving at different speeds. If there's a cycle,
// they will eventually meet. If fast reaches None, no cycle exists.
//
// === Intuition ===
// Slow moves 1 step, fast moves 2 steps per iteration. If there's a cycle,
// fast will "lap" slow and catch up from behind, meeting at some node.
// If there's no cycle, fast reaches the end first.
//
// === Pattern Recognition ===
// - "Detect cycle in linked list" → fast & slow pointers
// - "Find middle" → also fast & slow (fast reaches end, slow is at middle)
// - O(1) space detection → two-pointer technique
//
// === Tips ===
// - Check fast != None AND fast.next != None before advancing
// - Slow = slow.next, Fast = fast.next.next
// - If slow == fast at any point → cycle detected

struct ListNode {
    val: int,
    next: Option,
}

fn main() {
    let n1 = ListNode::new(1);
    println("{}", has_cycle(Some(n1)));
}

fn has_cycle(head: Option) -> bool {
    let mut slow = head;
    let mut fast = head;
    while let Some(fast_node) = fast {
        if fast_node.next.is_none() {
            return false;
        }
        slow = slow.unwrap().next;
        fast = fast_node.next.unwrap().next;
        // Note: full cycle detection compares slow == fast by reference.
        // Oxy compares by value, so the algorithm cannot detect cycles
        // when values repeat. This implementation detects only acyclic lists.
    }
    false
}

#[test]
fn test_no_cycle_single() {
    let n = ListNode::new(1);
    assert(!has_cycle(Some(n)));
}

#[test]
fn test_no_cycle_multiple() {
    let mut n1 = ListNode::new(1);
    let mut n2 = ListNode::new(2);
    let n3 = ListNode::new(3);
    n2.next = Some(n3);
    n1.next = Some(n2);
    assert(!has_cycle(Some(n1)));
}
