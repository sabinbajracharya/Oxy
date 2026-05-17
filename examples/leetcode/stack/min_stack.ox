// === Problem: Min Stack (LeetCode #155) ===
// Design a stack that supports push, pop, top, and retrieving the
// minimum element in O(1) time.
//
// === Pattern: Stack with Auxiliary Storage ===
// Maintain two stacks: one for values, one for minimums. When pushing,
// also push the min(current_min, new_val) onto the min stack.
//
// === Intuition ===
// The min stack always stores the minimum value seen so far at each level.
// When pushing 5 onto [3], min stack gets min(3,5)=3.
// When pushing 2 onto [5,3], min stack gets min(3,2)=2.
// Pop both stacks together to maintain sync.
//
// === Pattern Recognition ===
// - "Design a stack with X feature" → auxiliary data structure
// - "O(1) minimum" → maintain running minimum
// - Stack + extra property → second stack or pair elements
//
// === Tips ===
// - Use struct with two Vec fields
// - get_min() on empty stack can return None
// - push/pop update both stacks in sync

struct MinStack {
    data: Vec,
    mins: Vec,
}

impl MinStack {
    fn new() -> Self {
        MinStack { data: vec![], mins: vec![] }
    }

    fn push(&mut self, val: i64) {
        self.data.push(val);
        let min_val = if self.mins.is_empty() || val < self.mins.last().unwrap() {
            val
        } else {
            self.mins.last().unwrap()
        };
        self.mins.push(min_val);
    }

    fn pop(&mut self) {
        self.data.pop();
        self.mins.pop();
    }

    fn top(&self) -> i64 {
        self.data.last().unwrap()
    }

    fn get_min(&self) -> i64 {
        self.mins.last().unwrap()
    }
}

fn main() {
    let mut s = MinStack::new();
    s.push(-2);
    s.push(0);
    s.push(-3);
    println!("min: {}", s.get_min());
    s.pop();
    println!("top: {}", s.top());
    println!("min: {}", s.get_min());
}

#[test]
fn test_min_stack() {
    let mut s = MinStack::new();
    s.push(-2);
    s.push(0);
    s.push(-3);
    assert_eq!(s.get_min(), -3);
    s.pop();
    assert_eq!(s.top(), 0);
    assert_eq!(s.get_min(), -2);
}
