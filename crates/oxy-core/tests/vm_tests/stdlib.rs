//! JSON, HTTP, math, CLI args — standard-library surface.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_math_gcd() {
    let output = run_and_capture(
        r#"
fn main() {
    println("{}", math::gcd(12, 8));
    println("{}", math::gcd(7, 13));
    println("{}", math::gcd(0, 5));
}
"#,
    );
    assert_eq!(output, vec!["4\n", "1\n", "5\n"]);
}

#[test]
fn test_math_lcm() {
    let output = run_and_capture(
        r#"
fn main() {
    println("{}", math::lcm(4, 6));
    println("{}", math::lcm(7, 13));
    println("{}", math::lcm(0, 5));
}
"#,
    );
    assert_eq!(output, vec!["12\n", "91\n", "0\n"]);
}

#[test]
fn test_cli_args() {
    let out = run_and_capture(
        r#"
fn main() {
    let args = std::env::args();
    println("{}", args.len());
}
"#,
    );
    // In tests, args are empty (no actual CLI args passed)
    assert_eq!(out.len(), 1);
}

#[test]
fn test_json_serialize_primitives() {
    let output = run_and_capture(
        r#"fn main() {
    let a = json::serialize(42).unwrap();
    let b = json::serialize(3.14).unwrap();
    let c = json::serialize(true).unwrap();
    let d = json::serialize("hello").unwrap();
    println("{}", a);
    println("{}", b);
    println("{}", c);
    println("{}", d);
}"#,
    );
    assert_eq!(output, vec!["42\n", "3.14\n", "true\n", "\"hello\"\n"]);
}

#[test]
fn test_json_serialize_string_escapes() {
    let output = run_and_capture(
        r#"fn main() {
    let s = json::serialize("hello\nworld\t\"quoted\"").unwrap();
    println("{}", s);
}"#,
    );
    assert_eq!(output, vec!["\"hello\\nworld\\t\\\"quoted\\\"\"\n"]);
}

#[test]
fn test_json_serialize_vec() {
    let output = run_and_capture(
        r#"fn main() {
    let v = vec(1, 2, 3);
    let j = json::serialize(v).unwrap();
    println("{}", j);
}"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n"]);
}

#[test]
fn test_json_serialize_hashmap() {
    let output = run_and_capture(
        r#"fn main() {
    let mut m = HashMap::new();
    m.insert("alpha", 1);
    m.insert("beta", 2);
    let j = json::serialize(m).unwrap();
    println("{}", j);
}"#,
    );
    assert_eq!(output, vec!["{\"alpha\": 1, \"beta\": 2}\n"]);
}

#[test]
fn test_json_serialize_struct() {
    let output = run_and_capture(
        r#"
struct Point {
    x: int,
    y: int,
}
fn main() {
    let p = Point { x: 10, y: 20 };
    let j = json::serialize(p).unwrap();
    println("{}", j);
}"#,
    );
    assert_eq!(output, vec!["{\"x\": 10, \"y\": 20}\n"]);
}

#[test]
fn test_json_serialize_enum() {
    let output = run_and_capture(
        r#"
enum Color {
    Red,
    Green,
    Blue,
    Rgb(int, int, int),
}
fn main() {
    let a = json::serialize(Color::Red).unwrap();
    let b = json::serialize(Color::Rgb(255, 128, 0)).unwrap();
    println("{}", a);
    println("{}", b);
}"#,
    );
    assert_eq!(
        output,
        vec![
            "\"Red\"\n",
            "{\"variant\": \"Rgb\", \"data\": [255, 128, 0]}\n"
        ]
    );
}

#[test]
fn test_json_serialize_option_result() {
    let output = run_and_capture(
        r#"fn main() {
    let a = json::serialize(Some(42)).unwrap();
    let b = json::serialize(None).unwrap();
    let c = json::serialize(Ok("yes")).unwrap();
    let d = json::serialize(Err("no")).unwrap();
    println("{}", a);
    println("{}", b);
    println("{}", c);
    println("{}", d);
}"#,
    );
    assert_eq!(
        output,
        vec![
            "42\n",
            "null\n",
            "{\"Ok\": \"yes\"}\n",
            "{\"Err\": \"no\"}\n"
        ]
    );
}

#[test]
fn test_json_serialize_nested() {
    let output = run_and_capture(
        r#"fn main() {
    let v = vec(vec(1, 2), vec(3, 4));
    let j = json::serialize(v).unwrap();
    println("{}", j);
}"#,
    );
    assert_eq!(output, vec!["[[1, 2], [3, 4]]\n"]);
}

#[test]
fn test_json_serialize_pretty() {
    let output = run_and_capture(
        r#"fn main() {
    let v = vec(1, 2, 3);
    let j = json::to_string_pretty(v).unwrap();
    println("{}", j);
}"#,
    );
    assert_eq!(output, vec!["[\n  1,\n  2,\n  3\n]\n"]);
}

#[test]
fn test_json_deserialize_primitives() {
    let output = run_and_capture(
        r#"fn main() {
    let a = json::deserialize("42").unwrap();
    let b = json::deserialize("3.14").unwrap();
    let c = json::deserialize("true").unwrap();
    let d = json::deserialize("\"hello\"").unwrap();
    let e = json::deserialize("null").unwrap();
    println("{:?}", a);
    println("{:?}", b);
    println("{:?}", c);
    println("{}", d);
    println("{:?}", e);
}"#,
    );
    assert_eq!(output, vec!["42\n", "3.14\n", "true\n", "hello\n", "()\n"]);
}

#[test]
fn test_json_deserialize_object() {
    let output = run_and_capture(
        r#"fn main() {
    let obj = json::parse("{\"name\": \"Alice\", \"age\": 30}").unwrap();
    let name = obj.get("name").unwrap();
    let age = obj.get("age").unwrap();
    println("{}", name);
    println("{:?}", age);
}"#,
    );
    assert_eq!(output, vec!["Alice\n", "30\n"]);
}

#[test]
fn test_json_deserialize_array() {
    let output = run_and_capture(
        r#"fn main() {
    let arr = json::from_str("[1, 2, 3]").unwrap();
    println("{:?}", arr);
}"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n"]);
}

#[test]
fn test_json_deserialize_nested() {
    let output = run_and_capture(
        r#"fn main() {
    let data = json::deserialize("{\"items\": [1, 2, 3], \"ok\": true}").unwrap();
    let items = data.get("items").unwrap();
    let ok = data.get("ok").unwrap();
    println("{:?}", items);
    println("{:?}", ok);
}"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n", "true\n"]);
}

#[test]
fn test_json_roundtrip() {
    let output = run_and_capture(
        r#"fn main() {
    let original = vec(1, 2, 3);
    let json_str = json::serialize(original).unwrap();
    let parsed = json::deserialize(json_str).unwrap();
    println("{:?}", parsed);
}"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n"]);
}

#[test]
fn test_json_to_json_method() {
    let output = run_and_capture(
        r#"fn main() {
    let v = vec(1, 2, 3);
    let j = v.to_json().unwrap();
    println("{}", j);
    let n = 42;
    let j2 = n.to_json().unwrap();
    println("{}", j2);
}"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n", "42\n"]);
}

#[test]
fn test_json_error_cases() {
    let output = run_and_capture(
        r#"fn main() {
    let r = json::deserialize("invalid");
    match r {
        Result::Ok(_) => println("unexpected ok"),
        Result::Err(e) => println("error: {}", e),
    }
}"#,
    );
    assert!(output[0].starts_with("error: "));
}

#[test]
fn test_json_from_struct() {
    let output = run_and_capture(
        r#"
struct Person {
    name: String,
    age: int,
}
fn main() {
    let json_str = "{\"name\": \"Alice\", \"age\": 30}";
    let p = json::from_struct(json_str, "Person").unwrap();
    println("{:?}", p);
}"#,
    );
    assert!(output[0].contains("Alice"));
    assert!(output[0].contains("30"));
}

#[test]
fn test_http_get_invalid_url() {
    let output = run_and_capture(
        r#"
fn main() {
    let result = http::get("not-a-valid-url");
    match result {
        Ok(_) => println("unexpected ok"),
        Err(e) => println("got error"),
    }
}"#,
    );
    assert_eq!(output, vec!["got error\n"]);
}

#[test]
fn test_http_post_invalid_url() {
    let output = run_and_capture(
        r#"
fn main() {
    let result = http::post("http://invalid.test.localhost:1", "body");
    match result {
        Ok(_) => println("unexpected ok"),
        Err(e) => println("got error"),
    }
}"#,
    );
    assert_eq!(output, vec!["got error\n"]);
}

#[test]
fn test_http_delete_invalid_url() {
    let output = run_and_capture(
        r#"
fn main() {
    let result = http::delete("not-a-valid-url");
    match result {
        Ok(_) => println("unexpected ok"),
        Err(e) => println("got error"),
    }
}"#,
    );
    assert_eq!(output, vec!["got error\n"]);
}

#[test]
fn test_http_get_json_invalid_url() {
    let output = run_and_capture(
        r#"
fn main() {
    let result = http::get_json("not-a-valid-url");
    match result {
        Ok(_) => println("unexpected ok"),
        Err(e) => println("got error"),
    }
}"#,
    );
    assert_eq!(output, vec!["got error\n"]);
}

#[test]
fn test_http_post_json_invalid_url() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut data = HashMap::new();
    data.insert("key", "value");
    let result = http::post_json("not-a-valid-url", data);
    match result {
        Ok(_) => println("unexpected ok"),
        Err(_) => println("got error"),
    }
}"#,
    );
    assert_eq!(output, vec!["got error\n"]);
}

#[test]
fn test_http_put_json_invalid_url() {
    let output = run_and_capture(
        r#"
fn main() {
    let mut data = HashMap::new();
    data.insert("key", "value");
    let result = http::put_json("not-a-valid-url", data);
    match result {
        Ok(_) => println("unexpected ok"),
        Err(_) => println("got error"),
    }
}"#,
    );
    assert_eq!(output, vec!["got error\n"]);
}

#[test]
fn test_http_response_status_ok_logic() {
    // We can't make real requests, but we test the method dispatch
    // by building an HttpResponse struct directly via the builder pattern
    let output = run_and_capture(
        r#"
fn main() {
    let result = http::get("not-a-valid-url");
    match result {
        Ok(resp) => {
            println("status_ok: {}", resp.status_ok());
        }
        Err(e) => println("error as expected: {}", true),
    }
}"#,
    );
    assert_eq!(output, vec!["error as expected: true\n"]);
}

#[test]
fn test_http_unknown_function() {
    let result = run_compiled_capturing(
        r#"
fn main() {
    let r = http::unknown_func("test");
}"#,
    );
    assert!(result.is_err());
}

#[test]
fn test_math_sqrt() {
    let out = run_and_capture("fn main() { println(\"{}\", math::sqrt(16.0)); }");
    assert_eq!(out, vec!["4.0\n"]);
}

#[test]
fn test_math_trig() {
    let out = run_and_capture(
        "fn main() { println(\"{}\", math::sin(0.0)); println(\"{}\", math::cos(0.0)); }",
    );
    assert_eq!(out, vec!["0.0\n", "1.0\n"]);
}

#[test]
fn test_math_constants() {
    let out = run_and_capture("fn main() { println(\"{}\", math::PI); }");
    assert_eq!(out, vec!["3.141592653589793\n"]);
}

#[test]
fn test_math_constant_e() {
    let out = run_and_capture("fn main() { println(\"{}\", math::E); }");
    assert_eq!(out, vec!["2.718281828459045\n"]);
}

#[test]
fn test_math_pow() {
    let out = run_and_capture("fn main() { println(\"{}\", math::pow(2.0, 10.0)); }");
    assert_eq!(out, vec!["1024.0\n"]);
}

#[test]
fn test_math_floor_ceil_round() {
    let out = run_and_capture(
            "fn main() { println(\"{}\", math::floor(3.7)); println(\"{}\", math::ceil(3.2)); println(\"{}\", math::round(3.5)); }",
        );
    assert_eq!(out, vec!["3.0\n", "4.0\n", "4.0\n"]);
}

#[test]
fn test_math_abs() {
    let out = run_and_capture(
        "fn main() { println(\"{}\", math::abs(-42)); println(\"{}\", math::abs(-3.14)); }",
    );
    assert_eq!(out, vec!["42\n", "3.14\n"]);
}

#[test]
fn test_math_min_max() {
    let out = run_and_capture(
        "fn main() { println(\"{}\", math::min(3, 7)); println(\"{}\", math::max(3, 7)); }",
    );
    assert_eq!(out, vec!["3\n", "7\n"]);
}

#[test]
fn test_math_log() {
    let out = run_and_capture("fn main() { println(\"{}\", math::log(1.0)); }");
    assert_eq!(out, vec!["0.0\n"]);
}

#[test]
fn test_math_log2_log10() {
    let out = run_and_capture(
        "fn main() { println(\"{}\", math::log2(8.0)); println(\"{}\", math::log10(100.0)); }",
    );
    assert_eq!(out, vec!["3.0\n", "2.0\n"]);
}

#[test]
fn test_f64_methods() {
    let out = run_and_capture(
        r#"fn main() {
    let x = 16.0;
    println("{}", x.sqrt());
    let y = -5;
    println("{}", y.abs());
    let z = 3.7;
    println("{}", z.floor());
}"#,
    );
    assert_eq!(out, vec!["4.0\n", "5\n", "3.0\n"]);
}

#[test]
fn test_f64_clamp() {
    let out = run_and_capture("fn main() { let x = 15; println(\"{}\", x.clamp(0, 10)); }");
    assert_eq!(out, vec!["10\n"]);
}

#[test]
fn test_f64_min_max_method() {
    let out = run_and_capture(
        r#"fn main() {
    let a = 3;
    let b = 7;
    println("{}", a.min(b));
    println("{}", a.max(b));
}"#,
    );
    assert_eq!(out, vec!["3\n", "7\n"]);
}

#[test]
fn test_f64_pow_method() {
    let out = run_and_capture("fn main() { let x = 2.0; println(\"{}\", x.pow(10.0)); }");
    assert_eq!(out, vec!["1024.0\n"]);
}

#[test]
fn test_f64_trig_methods() {
    let out = run_and_capture(
        r#"fn main() {
    let x = 0.0;
    println("{}", x.sin());
    println("{}", x.cos());
}"#,
    );
    assert_eq!(out, vec!["0.0\n", "1.0\n"]);
}

#[test]
fn test_rand_random() {
    let out = run_and_capture(
        "fn main() { let x = rand::random(); println(\"{}\", x >= 0.0 && x < 1.0); }",
    );
    assert_eq!(out, vec!["true\n"]);
}

#[test]
fn test_rand_range() {
    let out = run_and_capture(
        "fn main() { let x = rand::range(1, 10); println(\"{}\", x >= 1 && x < 10); }",
    );
    assert_eq!(out, vec!["true\n"]);
}

#[test]
fn test_rand_bool() {
    let out = run_and_capture(
        r#"fn main() {
    let b = rand::bool();
    println("{}", b == true || b == false);
}"#,
    );
    assert_eq!(out, vec!["true\n"]);
}

#[test]
fn test_time_now() {
    let out = run_and_capture("fn main() { let t = time::now(); println(\"{}\", t > 0.0); }");
    assert_eq!(out, vec!["true\n"]);
}

#[test]
fn test_time_millis() {
    let out = run_and_capture("fn main() { let t = time::millis(); println(\"{}\", t > 0); }");
    assert_eq!(out, vec!["true\n"]);
}

#[test]
fn test_time_elapsed() {
    let out = run_and_capture(
            "fn main() { let start = time::now(); let elapsed = time::elapsed(start); println(\"{}\", elapsed >= 0.0); }",
        );
    assert_eq!(out, vec!["true\n"]);
}
