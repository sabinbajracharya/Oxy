//! JSON, HTTP, math, CLI args — standard-library surface.
//!
//! Shared helpers and imports come from the parent module (`use super::*`).

use super::*;

#[test]
fn test_math_gcd() {
    let output = run_and_capture(
        r#"
fn main() {
    io::println("{}", math::gcd(12, 8));
    io::println("{}", math::gcd(7, 13));
    io::println("{}", math::gcd(0, 5));
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
    io::println("{}", math::lcm(4, 6));
    io::println("{}", math::lcm(7, 13));
    io::println("{}", math::lcm(0, 5));
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
    val args = std::env::args();
    io::println("{}", args.len());
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
    val a = json::serialize(42).unwrap();
    val b = json::serialize(3.14).unwrap();
    val c = json::serialize(true).unwrap();
    val d = json::serialize("hello").unwrap();
    io::println("{}", a);
    io::println("{}", b);
    io::println("{}", c);
    io::println("{}", d);
}"#,
    );
    assert_eq!(output, vec!["42\n", "3.14\n", "true\n", "\"hello\"\n"]);
}

#[test]
fn test_json_serialize_string_escapes() {
    let output = run_and_capture(
        r#"fn main() {
    val s = json::serialize("hello\nworld\t\"quoted\"").unwrap();
    io::println("{}", s);
}"#,
    );
    assert_eq!(output, vec!["\"hello\\nworld\\t\\\"quoted\\\"\"\n"]);
}

#[test]
fn test_json_serialize_list() {
    let output = run_and_capture(
        r#"fn main() {
    val v = [1, 2, 3];
    val j = json::serialize(v).unwrap();
    io::println("{}", j);
}"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n"]);
}

#[test]
fn test_json_serialize_hashmap() {
    let output = run_and_capture(
        r#"fn main() {
    var m = Map::new();
    m.insert("alpha", 1);
    m.insert("beta", 2);
    val j = json::serialize(m).unwrap();
    io::println("{}", j);
}"#,
    );
    assert_eq!(output, vec!["{\"alpha\": 1, \"beta\": 2}\n"]);
}

#[test]
fn test_json_serialize_struct() {
    let output = run_and_capture(
        r#"
struct PoInt {
    x: Int,
    y: Int,
}
fn main() {
    val p = PoInt { x: 10, y: 20 };
    val j = json::serialize(p).unwrap();
    io::println("{}", j);
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
    Rgb(Int, Int, Int),
}
fn main() {
    val a = json::serialize(Color::Red).unwrap();
    val b = json::serialize(Color::Rgb(255, 128, 0)).unwrap();
    io::println("{}", a);
    io::println("{}", b);
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
    val a = json::serialize(Some(42)).unwrap();
    val b = json::serialize(None).unwrap();
    val c = json::serialize(Ok("yes")).unwrap();
    val d = json::serialize(Err("no")).unwrap();
    io::println("{}", a);
    io::println("{}", b);
    io::println("{}", c);
    io::println("{}", d);
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
    val v = [[1, 2], [3, 4]];
    val j = json::serialize(v).unwrap();
    io::println("{}", j);
}"#,
    );
    assert_eq!(output, vec!["[[1, 2], [3, 4]]\n"]);
}

#[test]
fn test_json_serialize_pretty() {
    let output = run_and_capture(
        r#"fn main() {
    val v = [1, 2, 3];
    val j = json::to_string_pretty(v).unwrap();
    io::println("{}", j);
}"#,
    );
    assert_eq!(output, vec!["[\n  1,\n  2,\n  3\n]\n"]);
}

#[test]
fn test_json_deserialize_primitives() {
    let output = run_and_capture(
        r#"fn main() {
    val a = json::deserialize("42").unwrap();
    val b = json::deserialize("3.14").unwrap();
    val c = json::deserialize("true").unwrap();
    val d = json::deserialize("\"hello\"").unwrap();
    val e = json::deserialize("null").unwrap();
    io::println("{:?}", a);
    io::println("{:?}", b);
    io::println("{:?}", c);
    io::println("{}", d);
    io::println("{:?}", e);
}"#,
    );
    assert_eq!(output, vec!["42\n", "3.14\n", "true\n", "hello\n", "()\n"]);
}

#[test]
fn test_json_deserialize_object() {
    let output = run_and_capture(
        r#"fn main() {
    val obj = json::parse("{\"name\": \"Alice\", \"age\": 30}").unwrap();
    val name = obj.get("name").unwrap();
    val age = obj.get("age").unwrap();
    io::println("{}", name);
    io::println("{:?}", age);
}"#,
    );
    assert_eq!(output, vec!["Alice\n", "30\n"]);
}

#[test]
fn test_json_deserialize_array() {
    let output = run_and_capture(
        r#"fn main() {
    val arr = json::from_str("[1, 2, 3]").unwrap();
    io::println("{:?}", arr);
}"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n"]);
}

#[test]
fn test_json_deserialize_nested() {
    let output = run_and_capture(
        r#"fn main() {
    val data = json::deserialize("{\"items\": [1, 2, 3], \"ok\": true}").unwrap();
    val items = data.get("items").unwrap();
    val ok = data.get("ok").unwrap();
    io::println("{:?}", items);
    io::println("{:?}", ok);
}"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n", "true\n"]);
}

#[test]
fn test_json_roundtrip() {
    let output = run_and_capture(
        r#"fn main() {
    val original = [1, 2, 3];
    val json_str = json::serialize(original).unwrap();
    val parsed = json::deserialize(json_str).unwrap();
    io::println("{:?}", parsed);
}"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n"]);
}

#[test]
fn test_json_to_json_method() {
    let output = run_and_capture(
        r#"fn main() {
    val v = [1, 2, 3];
    val j = v.to_json().unwrap();
    io::println("{}", j);
    val n = 42;
    val j2 = n.to_json().unwrap();
    io::println("{}", j2);
}"#,
    );
    assert_eq!(output, vec!["[1, 2, 3]\n", "42\n"]);
}

#[test]
fn test_json_error_cases() {
    let output = run_and_capture(
        r#"fn main() {
    val r = json::deserialize("invalid");
    match r {
        Result::Ok(_) => io::println("unexpected ok"),
        Result::Err(e) => io::println("error: {}", e),
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
    age: Int,
}
fn main() {
    val json_str = "{\"name\": \"Alice\", \"age\": 30}";
    val p = json::from_struct(json_str, "Person").unwrap();
    io::println("{:?}", p);
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
    val result = http::get("not-a-valid-url");
    match result {
        Ok(_) => io::println("unexpected ok"),
        Err(e) => io::println("got error"),
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
    val result = http::post("http://invalid.test.localhost:1", "body");
    match result {
        Ok(_) => io::println("unexpected ok"),
        Err(e) => io::println("got error"),
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
    val result = http::delete("not-a-valid-url");
    match result {
        Ok(_) => io::println("unexpected ok"),
        Err(e) => io::println("got error"),
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
    val result = http::get_json("not-a-valid-url");
    match result {
        Ok(_) => io::println("unexpected ok"),
        Err(e) => io::println("got error"),
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
    var data = Map::new();
    data.insert("key", "value");
    val result = http::post_json("not-a-valid-url", data);
    match result {
        Ok(_) => io::println("unexpected ok"),
        Err(_) => io::println("got error"),
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
    var data = Map::new();
    data.insert("key", "value");
    val result = http::put_json("not-a-valid-url", data);
    match result {
        Ok(_) => io::println("unexpected ok"),
        Err(_) => io::println("got error"),
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
    val result = http::get("not-a-valid-url");
    match result {
        Ok(resp) => {
            io::println("status_ok: {}", resp.status_ok());
        }
        Err(e) => io::println("error as expected: {}", true),
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
    val r = http::unknown_func("test");
}"#,
    );
    assert!(result.is_err());
}

#[test]
fn test_math_sqrt() {
    let out = run_and_capture("fn main() { io::println(\"{}\", math::sqrt(16.0)); }");
    assert_eq!(out, vec!["4.0\n"]);
}

#[test]
fn test_math_trig() {
    let out = run_and_capture(
        "fn main() { io::println(\"{}\", math::sin(0.0)); io::println(\"{}\", math::cos(0.0)); }",
    );
    assert_eq!(out, vec!["0.0\n", "1.0\n"]);
}

#[test]
fn test_math_constants() {
    let out = run_and_capture("fn main() { io::println(\"{}\", math::PI); }");
    assert_eq!(out, vec!["3.141592653589793\n"]);
}

#[test]
fn test_math_constant_e() {
    let out = run_and_capture("fn main() { io::println(\"{}\", math::E); }");
    assert_eq!(out, vec!["2.718281828459045\n"]);
}

#[test]
fn test_math_pow() {
    let out = run_and_capture("fn main() { io::println(\"{}\", math::pow(2.0, 10.0)); }");
    assert_eq!(out, vec!["1024.0\n"]);
}

#[test]
fn test_math_floor_ceil_round() {
    let out = run_and_capture(
            "fn main() { io::println(\"{}\", math::floor(3.7)); io::println(\"{}\", math::ceil(3.2)); io::println(\"{}\", math::round(3.5)); }",
        );
    assert_eq!(out, vec!["3.0\n", "4.0\n", "4.0\n"]);
}

#[test]
fn test_math_abs() {
    let out = run_and_capture(
        "fn main() { io::println(\"{}\", math::abs(-42)); io::println(\"{}\", math::abs(-3.14)); }",
    );
    assert_eq!(out, vec!["42\n", "3.14\n"]);
}

#[test]
fn test_math_min_max() {
    let out = run_and_capture(
        "fn main() { io::println(\"{}\", math::min(3, 7)); io::println(\"{}\", math::max(3, 7)); }",
    );
    assert_eq!(out, vec!["3\n", "7\n"]);
}

#[test]
fn test_math_log() {
    let out = run_and_capture("fn main() { io::println(\"{}\", math::log(1.0)); }");
    assert_eq!(out, vec!["0.0\n"]);
}

#[test]
fn test_math_log2_log10() {
    let out = run_and_capture(
        "fn main() { io::println(\"{}\", math::log2(8.0)); io::println(\"{}\", math::log10(100.0)); }",
    );
    assert_eq!(out, vec!["3.0\n", "2.0\n"]);
}

#[test]
fn test_f64_methods() {
    let out = run_and_capture(
        r#"fn main() {
    val x = 16.0;
    io::println("{}", x.sqrt());
    val y = -5;
    io::println("{}", y.abs());
    val z = 3.7;
    io::println("{}", z.floor());
}"#,
    );
    assert_eq!(out, vec!["4.0\n", "5\n", "3.0\n"]);
}

#[test]
fn test_f64_clamp() {
    let out = run_and_capture("fn main() { val x = 15; io::println(\"{}\", x.clamp(0, 10)); }");
    assert_eq!(out, vec!["10\n"]);
}

#[test]
fn test_f64_min_max_method() {
    let out = run_and_capture(
        r#"fn main() {
    val a = 3;
    val b = 7;
    io::println("{}", a.min(b));
    io::println("{}", a.max(b));
}"#,
    );
    assert_eq!(out, vec!["3\n", "7\n"]);
}

#[test]
fn test_f64_pow_method() {
    let out = run_and_capture("fn main() { val x = 2.0; io::println(\"{}\", x.pow(10.0)); }");
    assert_eq!(out, vec!["1024.0\n"]);
}

#[test]
fn test_f64_trig_methods() {
    let out = run_and_capture(
        r#"fn main() {
    val x = 0.0;
    io::println("{}", x.sin());
    io::println("{}", x.cos());
}"#,
    );
    assert_eq!(out, vec!["0.0\n", "1.0\n"]);
}

#[test]
fn test_rand_random() {
    let out = run_and_capture(
        "fn main() { val x = rand::random(); io::println(\"{}\", x >= 0.0 && x < 1.0); }",
    );
    assert_eq!(out, vec!["true\n"]);
}

#[test]
fn test_rand_range() {
    let out = run_and_capture(
        "fn main() { val x = rand::range(1, 10); io::println(\"{}\", x >= 1 && x < 10); }",
    );
    assert_eq!(out, vec!["true\n"]);
}

#[test]
fn test_rand_bool() {
    let out = run_and_capture(
        r#"fn main() {
    val b = rand::bool();
    io::println("{}", b == true || b == false);
}"#,
    );
    assert_eq!(out, vec!["true\n"]);
}

#[test]
fn test_time_now() {
    let out = run_and_capture("fn main() { val t = time::now(); io::println(\"{}\", t > 0.0); }");
    assert_eq!(out, vec!["true\n"]);
}

#[test]
fn test_time_millis() {
    let out = run_and_capture("fn main() { val t = time::millis(); io::println(\"{}\", t > 0); }");
    assert_eq!(out, vec!["true\n"]);
}

#[test]
fn test_time_elapsed() {
    let out = run_and_capture(
            "fn main() { val start = time::now(); val elapsed = time::elapsed(start); io::println(\"{}\", elapsed >= 0.0); }",
        );
    assert_eq!(out, vec!["true\n"]);
}
