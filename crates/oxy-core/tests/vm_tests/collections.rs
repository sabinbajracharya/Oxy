//! Vec, HashMap, HashSet, BinaryHeap, VecDeque, collect.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_array_literal() {
    let output = run_and_capture("fn main() { let a = [1, 2, 3]; println!(\"{:?}\", a); }");
    assert_eq!(output, vec!["[1, 2, 3]\n"]);
}

#[test]
fn test_empty_array() {
    let output = run_and_capture("fn main() { let a = []; println!(\"{:?}\", a); }");
    assert_eq!(output, vec!["[]\n"]);
}

#[test]
fn test_vec_macro() {
    let output = run_and_capture("fn main() { let v = vec![10, 20, 30]; println!(\"{:?}\", v); }");
    assert_eq!(output, vec!["[10, 20, 30]\n"]);
}

#[test]
fn test_vec_index() {
    let output = run_and_capture("fn main() { let v = vec![10, 20, 30]; println!(\"{}\", v[1]); }");
    assert_eq!(output, vec!["20\n"]);
}

#[test]
fn test_vec_push() {
    let output = run_and_capture(
        r#"fn main() {
let mut v = vec![1, 2];
v.push(3);
println!("{:?}", v);
}"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n"]);
}

#[test]
fn test_vec_pop() {
    let output = run_and_capture(
        r#"fn main() {
let mut v = vec![1, 2, 3];
let x = v.pop();
println!("{:?} {:?}", x, v);
}"#,
    );
    assert_eq!(output, vec!["Some(3) [1, 2]\n"]);
}

#[test]
fn test_vec_len() {
    let output = run_and_capture("fn main() { let v = vec![1, 2, 3]; println!(\"{}\", v.len()); }");
    assert_eq!(output, vec!["3\n"]);
}

#[test]
fn test_vec_is_empty() {
    let output = run_and_capture(
        r#"fn main() {
let a = [];
let b = vec![1];
println!("{} {}", a.is_empty(), b.is_empty());
}"#,
    );
    assert_eq!(output, vec!["true false\n"]);
}

#[test]
fn test_vec_contains() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![1, 2, 3];
println!("{} {}", v.contains(2), v.contains(5));
}"#,
    );
    assert_eq!(output, vec!["true false\n"]);
}

#[test]
fn test_vec_index_assign() {
    let output = run_and_capture(
        r#"fn main() {
let mut v = vec![1, 2, 3];
v[1] = 99;
println!("{:?}", v);
}"#,
    );
    assert_eq!(output, vec!["[1, 99, 3]\n"]);
}

#[test]
fn test_vec_iteration() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![10, 20, 30];
let mut sum = 0;
for x in v {
    sum += x;
}
println!("{}", sum);
}"#,
    );
    assert_eq!(output, vec!["60\n"]);
}

#[test]
fn test_vec_join() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec!["a", "b", "c"];
println!("{}", v.join(", "));
}"#,
    );
    assert_eq!(output, vec!["a, b, c\n"]);
}

#[test]
fn test_tuple_literal() {
    let output = run_and_capture("fn main() { let t = (1, 2, 3); println!(\"{:?}\", t); }");
    assert_eq!(output, vec!["(1, 2, 3)\n"]);
}

#[test]
fn test_tuple_index() {
    let output = run_and_capture(
        r#"fn main() {
let t = (10, "hello", true);
println!("{} {} {}", t.0, t.1, t.2);
}"#,
    );
    assert_eq!(output, vec!["10 hello true\n"]);
}

#[test]
fn test_empty_tuple() {
    let output = run_and_capture("fn main() { let t = (); println!(\"{:?}\", t); }");
    assert_eq!(output, vec!["()\n"]);
}

#[test]
fn test_single_element_tuple() {
    let output = run_and_capture("fn main() { let t = (42,); println!(\"{:?}\", t); }");
    assert_eq!(output, vec!["(42,)\n"]);
}

#[test]
fn test_string_len() {
    let output = run_and_capture(r#"fn main() { let s = "hello"; println!("{}", s.len()); }"#);
    assert_eq!(output, vec!["5\n"]);
}

#[test]
fn test_string_contains() {
    let output = run_and_capture(
        r#"fn main() {
let s = "hello world";
println!("{} {}", s.contains("world"), s.contains("xyz"));
}"#,
    );
    assert_eq!(output, vec!["true false\n"]);
}

#[test]
fn test_string_to_uppercase() {
    let output =
        run_and_capture(r#"fn main() { let s = "hello"; println!("{}", s.to_uppercase()); }"#);
    assert_eq!(output, vec!["HELLO\n"]);
}

#[test]
fn test_string_to_lowercase() {
    let output =
        run_and_capture(r#"fn main() { let s = "HELLO"; println!("{}", s.to_lowercase()); }"#);
    assert_eq!(output, vec!["hello\n"]);
}

#[test]
fn test_string_trim() {
    let output =
        run_and_capture(r#"fn main() { let s = "  hello  "; println!(">{}<", s.trim()); }"#);
    assert_eq!(output, vec![">hello<\n"]);
}

#[test]
fn test_string_starts_with() {
    let output = run_and_capture(
        r#"fn main() {
let s = "hello world";
println!("{} {}", s.starts_with("hello"), s.starts_with("world"));
}"#,
    );
    assert_eq!(output, vec!["true false\n"]);
}

#[test]
fn test_string_ends_with() {
    let output = run_and_capture(
        r#"fn main() {
let s = "hello world";
println!("{} {}", s.ends_with("world"), s.ends_with("hello"));
}"#,
    );
    assert_eq!(output, vec!["true false\n"]);
}

#[test]
fn test_string_replace() {
    let output = run_and_capture(
        r#"fn main() {
let s = "hello world";
println!("{}", s.replace("world", "oxy"));
}"#,
    );
    assert_eq!(output, vec!["hello oxy\n"]);
}

#[test]
fn test_string_split() {
    let output = run_and_capture(
        r#"fn main() {
let s = "a,b,c";
let parts = s.split(",");
println!("{:?}", parts);
}"#,
    );
    assert_eq!(output, vec!["[\"a\", \"b\", \"c\"]\n"]);
}

#[test]
fn test_string_chars() {
    let output = run_and_capture(
        r#"fn main() {
let s = "hi";
let chars = s.chars();
println!("{:?}", chars);
}"#,
    );
    assert_eq!(output, vec!["['h', 'i']\n"]);
}

#[test]
fn test_string_repeat() {
    let output = run_and_capture(r#"fn main() { println!("{}", "ab".repeat(3)); }"#);
    assert_eq!(output, vec!["ababab\n"]);
}

#[test]
fn test_string_iteration() {
    let output = run_and_capture(
        r#"fn main() {
for c in "abc" {
    println!("{}", c);
}
}"#,
    );
    assert_eq!(output, vec!["a\n", "b\n", "c\n"]);
}

#[test]
fn test_vec_first_last() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![10, 20, 30];
println!("{:?} {:?}", v.first(), v.last());
}"#,
    );
    assert_eq!(output, vec!["Some(10) Some(30)\n"]);
}

#[test]
fn test_vec_reverse() {
    let output = run_and_capture(
        r#"fn main() {
let mut v = vec![1, 2, 3];
v.reverse();
println!("{:?}", v);
}"#,
    );
    assert_eq!(output, vec!["[3, 2, 1]\n"]);
}

#[test]
fn test_nested_vec() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec![vec![1, 2], vec![3, 4]];
println!("{}", v[0][1]);
println!("{:?}", v);
}"#,
    );
    assert_eq!(output, vec!["2\n", "[[1, 2], [3, 4]]\n"]);
}

#[test]
fn test_debug_format_collections() {
    let output = run_and_capture(
        r#"fn main() {
let v = vec!["hello", "world"];
println!("{:?}", v);
let t = (1, "two", true);
println!("{:?}", t);
}"#,
    );
    assert_eq!(
        output,
        vec!["[\"hello\", \"world\"]\n", "(1, \"two\", true)\n"]
    );
}

#[test]
fn test_index_out_of_bounds() {
    let result = run("fn main() { let v = vec![1, 2]; let x = v[5]; }");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("out of bounds"), "actual error: {err}");
}

#[test]
fn test_tuple_index_out_of_bounds() {
    let result = run("fn main() { let t = (1, 2); let x = t.5; }");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("out of bounds"), "actual error: {err}");
}

#[test]
fn test_hashmap_new_and_insert() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("a", 1);
    m.insert("b", 2);
    println!("{}", m.len());
}
"#,
    );
    assert_eq!(output, vec!["2\n"]);
}

#[test]
fn test_hashmap_get() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("key", 42);
    let val = m.get("key");
    println!("{}", val.unwrap());
}
"#,
    );
    assert_eq!(output, vec!["42\n"]);
}

#[test]
fn test_hashmap_get_missing() {
    let output = run_and_capture(
        r#"
fn main() {
    let m = HashMap::new();
    let val = m.get("nope");
    println!("{}", val.is_none());
}
"#,
    );
    assert_eq!(output, vec!["true\n"]);
}

#[test]
fn test_hashmap_contains_key() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("x", 1);
    println!("{}", m.contains_key("x"));
    println!("{}", m.contains_key("y"));
}
"#,
    );
    assert_eq!(output, vec!["true\n", "false\n"]);
}

#[test]
fn test_hashmap_remove() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("a", 10);
    let removed = m.remove("a");
    println!("{}", removed.unwrap());
    println!("{}", m.is_empty());
}
"#,
    );
    assert_eq!(output, vec!["10\n", "true\n"]);
}

#[test]
fn test_hashmap_keys_values() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("b", 2);
    m.insert("a", 1);
    println!("{:?}", m.keys());
    println!("{:?}", m.values());
}
"#,
    );
    assert_eq!(output, vec!["[\"a\", \"b\"]\n", "[1, 2]\n"]);
}

#[test]
fn test_hashmap_debug_format() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("x", 1);
    println!("{:?}", m);
}
"#,
    );
    assert_eq!(output, vec!["{\"x\": 1}\n"]);
}

#[test]
fn test_hashmap_iteration() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut m = HashMap::new();
    m.insert("a", 1);
    m.insert("b", 2);
    for (k, v) in m {
        println!("{}: {}", k, v);
    }
}
"#,
    );
    assert_eq!(output, vec!["a: 1\n", "b: 2\n"]);
}

#[test]
fn test_hashmap_is_empty() {
    let output = run_and_capture(
        r#"
fn main() {
    let m = HashMap::new();
    println!("{}", m.is_empty());
}
"#,
    );
    assert_eq!(output, vec!["true\n"]);
}

#[test]
fn test_hashset_new_and_insert() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut s = HashSet::new();
    s.insert(1);
    s.insert(2);
    s.insert(1);
    println!("{}", s.len());
}
"#,
    );
    assert_eq!(output, vec!["2\n"]);
}

#[test]
fn test_hashset_contains() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut s = HashSet::new();
    s.insert("a");
    s.insert("b");
    println!("{}", s.contains("a"));
    println!("{}", s.contains("c"));
}
"#,
    );
    assert_eq!(output, vec!["true\n", "false\n"]);
}

#[test]
fn test_hashset_remove() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut s = HashSet::new();
    s.insert(1);
    s.insert(2);
    println!("{}", s.remove(1));
    println!("{}", s.len());
    println!("{}", s.remove(3));
}
"#,
    );
    assert_eq!(output, vec!["true\n", "1\n", "false\n"]);
}

#[test]
fn test_hashset_is_empty() {
    let output = run_and_capture(
        r#"
fn main() {
    let s = HashSet::new();
    println!("{}", s.is_empty());
}
"#,
    );
    assert_eq!(output, vec!["true\n"]);
}

#[test]
fn test_hashset_union() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut a = HashSet::new();
    a.insert(1);
    a.insert(2);
    let mut b = HashSet::new();
    b.insert(2);
    b.insert(3);
    let c = a.union(b);
    println!("{}", c.len());
    println!("{}", c.contains(1));
    println!("{}", c.contains(3));
}
"#,
    );
    assert_eq!(output, vec!["3\n", "true\n", "true\n"]);
}

#[test]
fn test_hashset_intersection() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut a = HashSet::new();
    a.insert(1);
    a.insert(2);
    let mut b = HashSet::new();
    b.insert(2);
    b.insert(3);
    let c = a.intersection(b);
    println!("{}", c.len());
    println!("{}", c.contains(2));
    println!("{}", c.contains(1));
}
"#,
    );
    assert_eq!(output, vec!["1\n", "true\n", "false\n"]);
}

#[test]
fn test_hashset_difference() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut a = HashSet::new();
    a.insert(1);
    a.insert(2);
    let mut b = HashSet::new();
    b.insert(2);
    b.insert(3);
    let c = a.difference(b);
    println!("{}", c.len());
    println!("{}", c.contains(1));
    println!("{}", c.contains(2));
}
"#,
    );
    assert_eq!(output, vec!["1\n", "true\n", "false\n"]);
}

#[test]
fn test_hashset_to_vec() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut s = HashSet::new();
    s.insert(3);
    s.insert(1);
    s.insert(2);
    let v = s.to_vec();
    println!("{}", v.len());
    // to_vec returns sorted elements
    println!("{}", v[0]);
    println!("{}", v[2]);
}
"#,
    );
    assert_eq!(output, vec!["3\n", "1\n", "3\n"]);
}

#[test]
fn test_hashset_clone() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut s = HashSet::new();
    s.insert(1);
    let c = s.clone();
    println!("{}", c.len());
    println!("{}", c.contains(1));
}
"#,
    );
    assert_eq!(output, vec!["1\n", "true\n"]);
}

#[test]
fn test_hashset_iteration() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut s = HashSet::new();
    s.insert(1);
    s.insert(2);
    for x in s {
        println!("{}", x);
    }
}
"#,
    );
    // iteration yields sorted elements
    assert_eq!(output, vec!["1\n", "2\n"]);
}

#[test]
fn test_hashset_string_elements() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut s = HashSet::new();
    s.insert("hello");
    s.insert("world");
    println!("{}", s.contains("hello"));
    println!("{}", s.len());
}
"#,
    );
    assert_eq!(output, vec!["true\n", "2\n"]);
}

#[test]
fn test_binary_heap_new_and_push() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut h = BinaryHeap::new();
    h.push(3);
    h.push(1);
    h.push(2);
    println!("{}", h.len());
}
"#,
    );
    assert_eq!(output, vec!["3\n"]);
}

#[test]
fn test_binary_heap_peek() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut h = BinaryHeap::new();
    h.push(1);
    h.push(5);
    h.push(3);
    // peek returns max
    println!("{}", h.peek().unwrap());
}
"#,
    );
    assert_eq!(output, vec!["5\n"]);
}

#[test]
fn test_binary_heap_pop_order() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut h = BinaryHeap::new();
    h.push(1);
    h.push(3);
    h.push(2);
    // pop returns max each time
    println!("{}", h.pop().unwrap());
    println!("{}", h.pop().unwrap());
    println!("{}", h.pop().unwrap());
    println!("{}", h.pop().is_none());
}
"#,
    );
    assert_eq!(output, vec!["3\n", "2\n", "1\n", "true\n"]);
}

#[test]
fn test_binary_heap_pop_empty() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut h = BinaryHeap::new();
    println!("{}", h.pop().is_none());
}
"#,
    );
    assert_eq!(output, vec!["true\n"]);
}

#[test]
fn test_binary_heap_min_heap_via_negation() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut h = BinaryHeap::new();
    h.push(-1);
    h.push(-3);
    h.push(-2);
    // max-heap on negated values = min-heap on original values
    println!("{}", -(h.pop().unwrap()));
    println!("{}", -(h.pop().unwrap()));
    println!("{}", -(h.pop().unwrap()));
}
"#,
    );
    assert_eq!(output, vec!["1\n", "2\n", "3\n"]);
}

#[test]
fn test_binary_heap_to_vec() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut h = BinaryHeap::new();
    h.push(3);
    h.push(1);
    h.push(2);
    let v = h.to_vec();
    // into_sorted_vec returns ascending order
    println!("{}", v[0]);
    println!("{}", v[2]);
}
"#,
    );
    assert_eq!(output, vec!["1\n", "3\n"]);
}

#[test]
fn test_vec_deque_new_and_push() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut d = VecDeque::new();
    d.push_back(1);
    d.push_back(2);
    d.push_front(0);
    println!("{}", d.len());
}
"#,
    );
    assert_eq!(output, vec!["3\n"]);
}

#[test]
fn test_vec_deque_front_back() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut d = VecDeque::new();
    d.push_back(1);
    d.push_back(3);
    println!("{}", d.front());
    println!("{}", d.back());
}
"#,
    );
    assert_eq!(output, vec!["1\n", "3\n"]);
}

#[test]
fn test_vec_deque_pop() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut d = VecDeque::new();
    d.push_back(1);
    d.push_back(2);
    d.push_back(3);
    println!("{:?}", d.pop_front());
    println!("{:?}", d.pop_back());
    println!("{}", d.len());
}
"#,
    );
    assert_eq!(output, vec!["Some(1)\n", "Some(3)\n", "1\n"]);
}

#[test]
fn test_vec_deque_to_vec() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut d = VecDeque::new();
    d.push_back(1);
    d.push_back(2);
    d.push_back(3);
    let v = d.to_vec();
    println!("{}", v[0]);
    println!("{}", v[2]);
}
"#,
    );
    assert_eq!(output, vec!["1\n", "3\n"]);
}

#[test]
fn test_turbofish_collect_vec() {
    let output = run_and_capture(
        r#"
            fn main() {
                let v = vec![1, 2, 3];
                let doubled = v.iter().map(|x| x * 2).collect::<Vec>();
                println!("{:?}", doubled);
            }
            "#,
    );
    assert_eq!(output, vec!["[2, 4, 6]\n"]);
}
