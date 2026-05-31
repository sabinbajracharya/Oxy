// === Problem: Valid Parentheses (LeetCode #20) ===
// Given a string containing '()', '{}', '[]', return true if all
// brackets are properly matched and nested.
//
// === Pattern: Stack ===
// Last-opened bracket must be closed first (LIFO). Push opening brackets
// onto a stack; when a closing bracket appears, the top of the stack
// must be its matching opener.
//
// === Intuition ===
// Walk through the string. Push each opener. For each closer, pop the
// top of the stack and check if it matches. If the stack is empty at
// a closer, or non-empty at the end, the string is invalid.
//
// === Pattern Recognition ===
// - "Matching pairs" + "nested" → stack
// - "Recently opened must close first" → LIFO = stack
// - Valid parentheses, HTML tags, XML validation all use this pattern
//
// === Tips ===
// - List::pop() returns an Option
// - Early return on size mismatch or wrong closer
// - Empty string is valid

fn main() {
    println("{}", is_valid("()[]{}"));
    println("{}", is_valid("([)]"));
}

fn is_valid(s: String) -> bool {
    let mut stack = list();
    for ch in s {
        if ch == '(' || ch == '[' || ch == '{' {
            stack.push(ch);
        } else {
            let popped = stack.pop();
            let ok = match ch {
                ')' => popped == Some('('),
                ']' => popped == Some('['),
                '}' => popped == Some('{'),
                _ => false,
            };
            if !ok {
                return false;
            }
        }
    }
    stack.is_empty()
}

#[test]
fn test_valid() {
    assert(is_valid("()"));
    assert(is_valid("()[]{}"));
    assert(is_valid("{[]}"));
}

#[test]
fn test_invalid() {
    assert(!is_valid("(]"));
    assert(!is_valid("([)]"));
}

#[test]
fn test_empty() {
    assert(is_valid(""));
}
