//! Destructuring (let/for) and assorted syntax-gap regression tests.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_for_destructure_vec_of_tuples() {
    let output = run_and_capture(
        r#"
fn main() {
    val pairs = [(1, "a"), (2, "b")];
    for (num, letter) in pairs {
        println("{} {}", num, letter);
    }
}
"#,
    );
    assert_eq!(output, vec!["1 a\n", "2 b\n"]);
}

#[test]
fn test_let_tuple_destructure() {
    let output = run_and_capture(
        r#"fn main() {
            val t = (1, 2, 3);
            val (a, b, c) = t;
            println("{} {} {}", a, b, c);
            }"#,
    );
    assert_eq!(output, vec!["1 2 3\n"]);
}

#[test]
fn test_let_slice_destructure() {
    let output = run_and_capture(
        r#"fn main() {
            val v = [10, 20];
            val [x, y] = v;
            println("{} {}", x, y);
            }"#,
    );
    assert_eq!(output, vec!["10 20\n"]);
}

#[test]
fn test_vec_empty_macro() {
    let output = run_and_capture(
        r#"
            fn main() {
                var v = [];
                println("{}", v.len());
                v.push(42);
                println("{}", v.len());
            }
            "#,
    );
    assert_eq!(output, vec!["0\n", "1\n"]);
}

#[test]
fn test_use_import_shortcut() {
    let output = run_and_capture(
        r#"
            use std::env;
            fn main() {
                val vars = env::vars();
                println("{}", vars.len() >= 0);
            }
            "#,
    );
    assert_eq!(output, vec!["true\n"]);
}

#[test]
fn test_range_slicing_list() {
    let output = run_and_capture(
        r#"
            fn main() {
                val v = [10, 20, 30, 40, 50];
                val a = v[1..4];
                println("{} {} {}", a[0], a[1], a[2]);
                val b = v[..2];
                println("{} {}", b[0], b[1]);
                val c = v[3..];
                println("{} {}", c[0], c[1]);
            }
            "#,
    );
    assert_eq!(output, vec!["20 30 40\n", "10 20\n", "40 50\n"]);
}

#[test]
fn test_range_slicing_string() {
    let output = run_and_capture(
        r#"
            fn main() {
                val s = "hello world";
                println("{}", s[..5]);
                println("{}", s[6..]);
                println("{}", s[2..8]);
            }
            "#,
    );
    assert_eq!(output, vec!["hello\n", "world\n", "llo wo\n"]);
}

#[test]
fn test_clone_list() {
    let output = run_and_capture(
        r#"
            fn main() {
                val a = [1, 2, 3];
                var b = a.clone();
                b.push(4);
                // .clone() is a deep copy — mutations don't propagate
                println("{} {}", a.len(), b.len());
            }
            "#,
    );
    assert_eq!(output, vec!["3 4\n"]);
}

#[test]
fn test_vec_shared_mutation() {
    let output = run_and_capture(
        r#"
            fn main() {
                val a = [1, 2, 3];
                var b = a;        // shared via Rc — no deep copy
                b.push(4);            // mutation visible through both
                println("{} {}", a.len(), b.len());
            }
            "#,
    );
    assert_eq!(output, vec!["4 4\n"]);
}

#[test]
fn test_clone_tuple() {
    let output = run_and_capture(
        r#"
            fn main() {
                val t = (1, "hello", true);
                val t2 = t.clone();
                println("{} {}", t.0, t2.1);
            }
            "#,
    );
    assert_eq!(output, vec!["1 hello\n"]);
}

#[test]
fn test_hashmap_index_access() {
    let output = run_and_capture(
        r#"
            fn main() {
                var m = Map::new();
                m.insert("name", "Oxy");
                m.insert("version", "0.1");
                println("{}", m["name"]);
                println("{}", m["version"]);
            }
            "#,
    );
    assert_eq!(output, vec!["Oxy\n", "0.1\n"]);
}

#[test]
fn test_use_group_std() {
    let output = run_and_capture(
        r#"
            use std::{env, fs};
            fn main() {
                val vars = env::vars();
                println("{}", vars.len() > 0);
            }
            "#,
    );
    assert_eq!(output, vec!["true\n"]);
}

#[test]
fn test_match_guard() {
    let output = run_and_capture(
        r#"
            fn main() {
                val x = 5;
                val result = match x {
                    n if n < 0 => "negative",
                    n if n == 0 => "zero",
                    n if n > 0 => "positive",
                    _ => "unknown",
                };
                println("{}", result);
            }
            "#,
    );
    assert_eq!(output, vec!["positive\n"]);
}

#[test]
fn test_match_guard_with_binding() {
    let output = run_and_capture(
        r#"
            fn main() {
                val values = [1, -2, 3, -4, 5];
                var pos = 0;
                var neg = 0;
                for v in values {
                    match v {
                        n if n > 0 => pos = pos + n,
                        n if n < 0 => neg = neg + n,
                        _ => {},
                    }
                }
                println("{} {}", pos, neg);
            }
            "#,
    );
    assert_eq!(output, vec!["9 -6\n"]);
}

#[test]
fn test_operator_overload_add() {
    let output = run_and_capture(
        r#"
            struct PoInt { x: Int, y: Int }

            trait Add {
                fn add(self, other: PoInt) -> PoInt;
            }

            impl Add for PoInt {
                fn add(self, other: PoInt) -> PoInt {
                    PoInt { x: self.x + other.x, y: self.y + other.y }
                }
            }

            fn main() {
                val a = PoInt { x: 1, y: 2 };
                val b = PoInt { x: 3, y: 4 };
                val c = a + b;
                println("{} {}", c.x, c.y);
            }
            "#,
    );
    assert_eq!(output, vec!["4 6\n"]);
}

#[test]
fn test_impl_display() {
    let output = run_and_capture(
        r#"
            struct PoInt { x: Int, y: Int }

            trait Display {
                fn fmt(self) -> String;
            }

            impl Display for PoInt {
                fn fmt(self) -> String {
                    format("({}, {})", self.x, self.y)
                }
            }

            fn main() {
                val p = PoInt { x: 3, y: 4 };
                println("PoInt is: {}", p);
            }
            "#,
    );
    assert_eq!(output, vec!["PoInt is: (3, 4)\n"]);
}

#[test]
fn test_enum_methods() {
    let output = run_and_capture(
        r#"
            enum Direction {
                North,
                South,
                East,
                West,
            }

            impl Direction {
                fn is_horizontal(self) -> bool {
                    match self {
                        Direction::East => true,
                        Direction::West => true,
                        _ => false,
                    }
                }
            }

            fn main() {
                val d = Direction::East;
                println("{}", d.is_horizontal());
                val d2 = Direction::North;
                println("{}", d2.is_horizontal());
            }
            "#,
    );
    assert_eq!(output, vec!["true\n", "false\n"]);
}
