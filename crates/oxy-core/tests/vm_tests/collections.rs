//! Vec, HashMap, HashSet, BinaryHeap, VecDeque, collect.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_array_literal() {
    let output = run_and_capture("fn main() { val a = [1, 2, 3]; io::println(\"{:?}\", a); }");
    assert_eq!(output, vec!["[1, 2, 3]\n"]);
}

#[test]
fn test_empty_array() {
    let output = run_and_capture("fn main() { val a = []; io::println(\"{:?}\", a); }");
    assert_eq!(output, vec!["[]\n"]);
}

#[test]
fn test_vec_macro() {
    let output = run_and_capture("fn main() { val v = [10, 20, 30]; io::println(\"{:?}\", v); }");
    assert_eq!(output, vec!["[10, 20, 30]\n"]);
}

#[test]
fn test_vec_index() {
    let output = run_and_capture("fn main() { val v = [10, 20, 30]; io::println(\"{}\", v[1]); }");
    assert_eq!(output, vec!["20\n"]);
}

#[test]
fn test_vec_push() {
    let output = run_and_capture(
        r#"fn main() {
var v = [1, 2];
v.push(3);
io::println("{:?}", v);
}"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n"]);
}

#[test]
fn test_vec_pop() {
    let output = run_and_capture(
        r#"fn main() {
var v = [1, 2, 3];
val x = v.pop();
io::println("{:?} {:?}", x, v);
}"#,
    );
    assert_eq!(output, vec!["Some(3) [1, 2]\n"]);
}

#[test]
fn test_vec_len() {
    let output = run_and_capture("fn main() { val v = [1, 2, 3]; io::println(\"{}\", v.len()); }");
    assert_eq!(output, vec!["3\n"]);
}

#[test]
fn test_vec_is_empty() {
    let output = run_and_capture(
        r#"fn main() {
val a = [];
val b = [1];
io::println("{} {}", a.is_empty(), b.is_empty());
}"#,
    );
    assert_eq!(output, vec!["true false\n"]);
}

#[test]
fn test_vec_contains() {
    let output = run_and_capture(
        r#"fn main() {
val v = [1, 2, 3];
io::println("{} {}", v.contains(2), v.contains(5));
}"#,
    );
    assert_eq!(output, vec!["true false\n"]);
}

#[test]
fn test_vec_index_assign() {
    let output = run_and_capture(
        r#"fn main() {
var v = [1, 2, 3];
v[1] = 99;
io::println("{:?}", v);
}"#,
    );
    assert_eq!(output, vec!["[1, 99, 3]\n"]);
}

#[test]
fn test_vec_iteration() {
    let output = run_and_capture(
        r#"fn main() {
val v = [10, 20, 30];
var sum = 0;
for x in v {
    sum += x;
}
io::println("{}", sum);
}"#,
    );
    assert_eq!(output, vec!["60\n"]);
}

#[test]
fn test_vec_join() {
    let output = run_and_capture(
        r#"fn main() {
val v = ["a", "b", "c"];
io::println("{}", v.join(", "));
}"#,
    );
    assert_eq!(output, vec!["a, b, c\n"]);
}

#[test]
fn test_tuple_literal() {
    let output = run_and_capture("fn main() { val t = (1, 2, 3); io::println(\"{:?}\", t); }");
    assert_eq!(output, vec!["(1, 2, 3)\n"]);
}

#[test]
fn test_tuple_index() {
    let output = run_and_capture(
        r#"fn main() {
val t = (10, "hello", true);
io::println("{} {} {}", t.0, t.1, t.2);
}"#,
    );
    assert_eq!(output, vec!["10 hello true\n"]);
}

#[test]
fn test_empty_tuple() {
    let output = run_and_capture("fn main() { val t = (); io::println(\"{:?}\", t); }");
    assert_eq!(output, vec!["()\n"]);
}

#[test]
fn test_single_element_tuple() {
    let output = run_and_capture("fn main() { val t = (42,); io::println(\"{:?}\", t); }");
    assert_eq!(output, vec!["(42,)\n"]);
}

#[test]
fn test_string_len() {
    let output = run_and_capture(r#"fn main() { val s = "hello"; io::println("{}", s.len()); }"#);
    assert_eq!(output, vec!["5\n"]);
}

#[test]
fn test_string_contains() {
    let output = run_and_capture(
        r#"fn main() {
val s = "hello world";
io::println("{} {}", s.contains("world"), s.contains("xyz"));
}"#,
    );
    assert_eq!(output, vec!["true false\n"]);
}

#[test]
fn test_string_to_uppercase() {
    let output =
        run_and_capture(r#"fn main() { val s = "hello"; io::println("{}", s.to_uppercase()); }"#);
    assert_eq!(output, vec!["HELLO\n"]);
}

#[test]
fn test_string_to_lowercase() {
    let output =
        run_and_capture(r#"fn main() { val s = "HELLO"; io::println("{}", s.to_lowercase()); }"#);
    assert_eq!(output, vec!["hello\n"]);
}

#[test]
fn test_string_trim() {
    let output =
        run_and_capture(r#"fn main() { val s = "  hello  "; io::println(">{}<", s.trim()); }"#);
    assert_eq!(output, vec![">hello<\n"]);
}

#[test]
fn test_string_starts_with() {
    let output = run_and_capture(
        r#"fn main() {
val s = "hello world";
io::println("{} {}", s.starts_with("hello"), s.starts_with("world"));
}"#,
    );
    assert_eq!(output, vec!["true false\n"]);
}

#[test]
fn test_string_ends_with() {
    let output = run_and_capture(
        r#"fn main() {
val s = "hello world";
io::println("{} {}", s.ends_with("world"), s.ends_with("hello"));
}"#,
    );
    assert_eq!(output, vec!["true false\n"]);
}

#[test]
fn test_string_replace() {
    let output = run_and_capture(
        r#"fn main() {
val s = "hello world";
io::println("{}", s.replace("world", "oxy"));
}"#,
    );
    assert_eq!(output, vec!["hello oxy\n"]);
}

#[test]
fn test_string_split() {
    let output = run_and_capture(
        r#"fn main() {
val s = "a,b,c";
val parts = s.split(",");
io::println("{:?}", parts);
}"#,
    );
    assert_eq!(output, vec!["[\"a\", \"b\", \"c\"]\n"]);
}

#[test]
fn test_string_chars() {
    let output = run_and_capture(
        r#"fn main() {
val s = "hi";
val chars = s.chars();
io::println("{:?}", chars);
}"#,
    );
    assert_eq!(output, vec!["['h', 'i']\n"]);
}

#[test]
fn test_string_repeat() {
    let output = run_and_capture(r#"fn main() { io::println("{}", "ab".repeat(3)); }"#);
    assert_eq!(output, vec!["ababab\n"]);
}

#[test]
fn test_string_iteration() {
    let output = run_and_capture(
        r#"fn main() {
for c in "abc" {
    io::println("{}", c);
}
}"#,
    );
    assert_eq!(output, vec!["a\n", "b\n", "c\n"]);
}

#[test]
fn test_vec_first_last() {
    let output = run_and_capture(
        r#"fn main() {
val v = [10, 20, 30];
io::println("{:?} {:?}", v.first(), v.last());
}"#,
    );
    assert_eq!(output, vec!["Some(10) Some(30)\n"]);
}

#[test]
fn test_vec_reverse() {
    let output = run_and_capture(
        r#"fn main() {
var v = [1, 2, 3];
v.reverse();
io::println("{:?}", v);
}"#,
    );
    assert_eq!(output, vec!["[3, 2, 1]\n"]);
}

#[test]
fn test_nested_list() {
    let output = run_and_capture(
        r#"fn main() {
val v = [[1, 2], [3, 4]];
io::println("{}", v[0][1]);
io::println("{:?}", v);
}"#,
    );
    assert_eq!(output, vec!["2\n", "[[1, 2], [3, 4]]\n"]);
}

#[test]
fn test_debug_format_collections() {
    let output = run_and_capture(
        r#"fn main() {
val v = ["hello", "world"];
io::println("{:?}", v);
val t = (1, "two", true);
io::println("{:?}", t);
}"#,
    );
    assert_eq!(
        output,
        vec!["[\"hello\", \"world\"]\n", "(1, \"two\", true)\n"]
    );
}

#[test]
fn test_index_out_of_bounds() {
    let result = run_compiled("fn main() { val v = [1, 2]; val x = v[5]; }");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("out of bounds"), "actual error: {err}");
}

#[test]
fn test_tuple_index_out_of_bounds() {
    let result = run_compiled("fn main() { val t = (1, 2); val x = t.5; }");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("out of bounds"), "actual error: {err}");
}

#[test]
fn test_hashmap_new_and_insert() {
    let output = run_and_capture(
        r#"
fn main() {
    var m = Map::new();
    m.insert("a", 1);
    m.insert("b", 2);
    io::println("{}", m.len());
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
    var m = Map::new();
    m.insert("key", 42);
    val value = m.get("key");
    io::println("{}", value.unwrap());
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
    val m = Map::new();
    val value = m.get("nope");
    io::println("{}", value.is_none());
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
    var m = Map::new();
    m.insert("x", 1);
    io::println("{}", m.contains_key("x"));
    io::println("{}", m.contains_key("y"));
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
    var m = Map::new();
    m.insert("a", 10);
    val removed = m.remove("a");
    io::println("{}", removed.unwrap());
    io::println("{}", m.is_empty());
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
    var m = Map::new();
    m.insert("b", 2);
    m.insert("a", 1);
    io::println("{:?}", m.keys());
    io::println("{:?}", m.values());
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
    var m = Map::new();
    m.insert("x", 1);
    io::println("{:?}", m);
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
    var m = Map::new();
    m.insert("a", 1);
    m.insert("b", 2);
    for (k, v) in m {
        io::println("{}: {}", k, v);
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
    val m = Map::new();
    io::println("{}", m.is_empty());
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
    var s = Set::new();
    s.insert(1);
    s.insert(2);
    s.insert(1);
    io::println("{}", s.len());
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
    var s = Set::new();
    s.insert("a");
    s.insert("b");
    io::println("{}", s.contains("a"));
    io::println("{}", s.contains("c"));
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
    var s = Set::new();
    s.insert(1);
    s.insert(2);
    io::println("{}", s.remove(1));
    io::println("{}", s.len());
    io::println("{}", s.remove(3));
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
    val s = Set::new();
    io::println("{}", s.is_empty());
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
    var a = Set::new();
    a.insert(1);
    a.insert(2);
    var b = Set::new();
    b.insert(2);
    b.insert(3);
    val c = a.union(b);
    io::println("{}", c.len());
    io::println("{}", c.contains(1));
    io::println("{}", c.contains(3));
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
    var a = Set::new();
    a.insert(1);
    a.insert(2);
    var b = Set::new();
    b.insert(2);
    b.insert(3);
    val c = a.intersection(b);
    io::println("{}", c.len());
    io::println("{}", c.contains(2));
    io::println("{}", c.contains(1));
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
    var a = Set::new();
    a.insert(1);
    a.insert(2);
    var b = Set::new();
    b.insert(2);
    b.insert(3);
    val c = a.difference(b);
    io::println("{}", c.len());
    io::println("{}", c.contains(1));
    io::println("{}", c.contains(2));
}
"#,
    );
    assert_eq!(output, vec!["1\n", "true\n", "false\n"]);
}

#[test]
fn test_hashset_to_list() {
    let output = run_and_capture(
        r#"
fn main() {
    var s = Set::new();
    s.insert(3);
    s.insert(1);
    s.insert(2);
    val v = s.to_vec();
    io::println("{}", v.len());
    // to_vec returns sorted elements
    io::println("{}", v[0]);
    io::println("{}", v[2]);
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
    var s = Set::new();
    s.insert(1);
    val c = s.clone();
    io::println("{}", c.len());
    io::println("{}", c.contains(1));
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
    var s = Set::new();
    s.insert(1);
    s.insert(2);
    for x in s {
        io::println("{}", x);
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
    var s = Set::new();
    s.insert("hello");
    s.insert("world");
    io::println("{}", s.contains("hello"));
    io::println("{}", s.len());
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
    var h = BinaryHeap::new();
    h.push(3);
    h.push(1);
    h.push(2);
    io::println("{}", h.len());
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
    var h = BinaryHeap::new();
    h.push(1);
    h.push(5);
    h.push(3);
    // peek returns max
    io::println("{}", h.peek().unwrap());
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
    var h = BinaryHeap::new();
    h.push(1);
    h.push(3);
    h.push(2);
    // pop returns max each time
    io::println("{}", h.pop().unwrap());
    io::println("{}", h.pop().unwrap());
    io::println("{}", h.pop().unwrap());
    io::println("{}", h.pop().is_none());
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
    var h = BinaryHeap::new();
    io::println("{}", h.pop().is_none());
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
    var h = BinaryHeap::new();
    h.push(-1);
    h.push(-3);
    h.push(-2);
    // max-heap on negated values = min-heap on original values
    io::println("{}", -(h.pop().unwrap()));
    io::println("{}", -(h.pop().unwrap()));
    io::println("{}", -(h.pop().unwrap()));
}
"#,
    );
    assert_eq!(output, vec!["1\n", "2\n", "3\n"]);
}

#[test]
fn test_binary_heap_to_list() {
    let output = run_and_capture(
        r#"
fn main() {
    var h = BinaryHeap::new();
    h.push(3);
    h.push(1);
    h.push(2);
    val v = h.to_vec();
    // into_sorted_vec returns ascending order
    io::println("{}", v[0]);
    io::println("{}", v[2]);
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
    var d = VecDeque::new();
    d.push_back(1);
    d.push_back(2);
    d.push_front(0);
    io::println("{}", d.len());
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
    var d = VecDeque::new();
    d.push_back(1);
    d.push_back(3);
    io::println("{}", d.front());
    io::println("{}", d.back());
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
    var d = VecDeque::new();
    d.push_back(1);
    d.push_back(2);
    d.push_back(3);
    io::println("{:?}", d.pop_front());
    io::println("{:?}", d.pop_back());
    io::println("{}", d.len());
}
"#,
    );
    assert_eq!(output, vec!["Some(1)\n", "Some(3)\n", "1\n"]);
}

#[test]
fn test_vec_deque_to_list() {
    let output = run_and_capture(
        r#"
fn main() {
    var d = VecDeque::new();
    d.push_back(1);
    d.push_back(2);
    d.push_back(3);
    val v = d.to_vec();
    io::println("{}", v[0]);
    io::println("{}", v[2]);
}
"#,
    );
    assert_eq!(output, vec!["1\n", "3\n"]);
}

#[test]
fn test_turbofish_collect_list() {
    let output = run_and_capture(
        r#"
            fn main() {
                val v = [1, 2, 3];
                val doubled = v.iter().map(|x| x * 2).collect::<List>();
                io::println("{:?}", doubled);
            }
            "#,
    );
    assert_eq!(output, vec!["[2, 4, 6]\n"]);
}
