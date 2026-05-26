import type { Chapter } from '../types';

export const errorHandling: Chapter = {
  id: 'error-handling',
  title: 'Error Handling',
  lessons: [
    {
      id: 'option-basics',
      title: 'Option Basics',
      instructions: `## Option: Maybe a Value

\`Option<T>\` represents a value that may or may not be present:

- \`Some(value)\` — a value is present
- \`None\` — no value

This is perfect when you need "something or nothing" without an error message. Methods like \`is_some()\` and \`is_none()\` let you check which variant you have.

**Your task:** Implement \`safe_divide\` — return \`None\` when dividing by zero, \`Some(a / b)\` otherwise.`,
      hints: [
        'Use `if b == 0 { None } else { Some(a / b) }` to pick the right variant.',
        '`None` is a variant name, not a function call — no parentheses needed.',
        '`Some(value)` uses parentheses to wrap the inner value.',
      ],
      initialCode: `fn safe_divide(a: int, b: int) -> Option<int> {
    // TODO: return None when b == 0, otherwise Some(a / b)
    Some(a / b)  // crashes on zero — fix this!
}

fn main() {
    let r = safe_divide(10, 2);
    match r {
        Some(v) => println!("10 / 2 = {}", v),
        None => println!("division by zero"),
    }
    println!("is some? {}", r.is_some());
}
`,
      testCode: `#[test]
fn test_safe_divide_normal() {
    let r = safe_divide(10, 2);
    assert!(r.is_some());
    assert_eq!(r.unwrap(), 5);
}

#[test]
fn test_safe_divide_by_zero() {
    let r = safe_divide(10, 0);
    assert!(r.is_none());
}

#[test]
fn test_safe_divide_negative() {
    let r = safe_divide(-15, 3);
    assert!(r.is_some());
    assert_eq!(r.unwrap(), -5);
}

#[test]
fn test_safe_divide_one() {
    let r = safe_divide(7, 1);
    assert!(r.is_some());
    assert_eq!(r.unwrap(), 7);
}

#[test]
fn test_safe_divide_large() {
    let r = safe_divide(1000000, 1000);
    assert!(r.is_some());
    assert_eq!(r.unwrap(), 1000);
}
`,
    },
    {
      id: 'option-methods',
      title: 'Option Methods',
      instructions: `## Extracting Values from Option

Once you have an \`Option<T>\`, use these methods to get the value out:

- \`unwrap()\` — returns the value, panics on None
- \`unwrap_or(default)\` — returns the value or a default
- \`expect(msg)\` — returns the value or panics with a custom message
- \`unwrap_or_else(fn)\` — returns the value or calls a closure for the default

**Your task:** Complete \`get_or_default\` using \`unwrap_or\`, and \`safe_expect\` using \`expect\`.`,
      hints: [
        '`opt.unwrap_or(0)` returns the inner value or 0 if None.',
        '`opt.expect("should exist")` panics with that message on None — use when None is a bug.',
        '`opt.unwrap_or_else(|| expensive_default())` defers computation of the default.',
      ],
      initialCode: `fn get_or_default(opt: Option<int>, fallback: int) -> int {
    // TODO: use unwrap_or to return the value or the fallback
    0  // placeholder — replace with opt.unwrap_or(fallback)
}

fn safe_expect(opt: Option<int>, msg: String) -> int {
    // TODO: use expect to return the value or panic with the message
    0  // placeholder — replace with opt.expect(msg)
}

fn main() {
    let x = Some(42);
    let y = None;
    println!("Some(42) or 0: {}", get_or_default(x, 0));
    println!("None or 99: {}", get_or_default(y, 99));
    println!("expect Some(42): {}", safe_expect(x, "has value".to_string()));
}
`,
      testCode: `#[test]
fn test_unwrap_or_some() {
    assert_eq!(get_or_default(Some(10), 0), 10);
}

#[test]
fn test_unwrap_or_none() {
    assert_eq!(get_or_default(None, 42), 42);
}

#[test]
fn test_unwrap_or_none_negative() {
    assert_eq!(get_or_default(None, -1), -1);
}

#[test]
fn test_unwrap_or_some_ignores_fallback() {
    assert_eq!(get_or_default(Some(5), 999), 5);
}

#[test]
fn test_unwrap_or_large_some() {
    assert_eq!(get_or_default(Some(1000), 0), 1000);
}

#[test]
fn test_expect_some() {
    assert_eq!(safe_expect(Some(100), "msg".to_string()), 100);
}

#[test]
fn test_expect_some_zero() {
    assert_eq!(safe_expect(Some(0), "zero".to_string()), 0);
}

#[test]
fn test_expect_negative() {
    assert_eq!(safe_expect(Some(-42), "neg".to_string()), -42);
}
`,
    },
    {
      id: 'result-basics',
      title: 'Result Basics',
      instructions: `## Result: Success or Failure

\`Result<T, E>\` represents a fallible operation with two variants:

- \`Ok(value)\` — success with a value
- \`Err(error)\` — failure with an error payload

Unlike \`Option\`, \`Result\` carries error information in its \`E\` type. Use \`is_ok()\` / \`is_err()\` to check the state.

**Your task:** Implement \`parse_int\` that calls \`s.parse_int()\` and returns \`Ok(value)\` on success or \`Err("not a number".to_string())\` on failure.`,
      hints: [
        '`s.parse_int()` returns a `Result<int, String>` — check it with `is_ok()`.',
        'Use `match s.parse_int() { Ok(v) => ..., Err(_) => ... }` to handle both paths.',
        'Create Err with: `Err("not a number".to_string())`.',
      ],
      initialCode: `fn parse_int(s: String) -> Result<int, String> {
    // TODO: return Ok(value) if parse succeeds, Err("not a number") otherwise
    s.parse_int()  // placeholder — needs error handling
}

fn main() {
    let r = parse_int("42".to_string());
    match r {
        Ok(v) => println!("parsed: {}", v),
        Err(e) => println!("error: {}", e),
    }
    let bad = parse_int("hello".to_string());
    println!("bad parse is error: {}", bad.is_err());
}
`,
      testCode: `#[test]
fn test_parse_valid_number() {
    let r = parse_int("42".to_string());
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), 42);
}

#[test]
fn test_parse_zero() {
    let r = parse_int("0".to_string());
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), 0);
}

#[test]
fn test_parse_negative() {
    let r = parse_int("-5".to_string());
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), -5);
}

#[test]
fn test_parse_positive_with_plus() {
    let r = parse_int("+7".to_string());
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), 7);
}

#[test]
fn test_parse_invalid_string() {
    let r = parse_int("hello".to_string());
    assert!(r.is_err());
}

#[test]
fn test_parse_empty_string() {
    let r = parse_int("".to_string());
    assert!(r.is_err());
}

#[test]
fn test_parse_large_number() {
    let r = parse_int("999999".to_string());
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), 999999);
}
`,
    },
    {
      id: 'result-methods',
      title: 'Result Methods',
      instructions: `## Extracting Values from Result

\`Result<T, E>\` provides extraction and transformation methods:

- \`unwrap()\` — returns Ok value, panics on Err
- \`unwrap_or(default)\` — returns Ok value or a default
- \`expect(msg)\` — returns Ok value or panics with message
- \`map_err(fn)\` — transforms the error value (leaves Ok unchanged)

**Your task:** Complete \`get_or_default_result\` using \`unwrap_or\`, and \`transform_error\` using \`map_err\` to uppercase error messages.`,
      hints: [
        '`result.unwrap_or(0)` extracts Ok value or uses 0 as fallback.',
        '`result.map_err(|e| e.to_uppercase())` transforms only the Err variant.',
        '`map_err` leaves Ok values unchanged — it only touches Err.',
      ],
      initialCode: `fn get_or_default_result(result: Result<int, String>, fallback: int) -> int {
    // TODO: use unwrap_or to return the Ok value or the fallback
    0  // placeholder — replace with result.unwrap_or(fallback)
}

fn transform_error(result: Result<int, String>) -> Result<int, String> {
    // TODO: use map_err to uppercase the error string
    result  // placeholder — replace with result.map_err(...)
}

fn main() {
    let ok: Result<int, String> = Ok(42);
    let err: Result<int, String> = Err("fail".to_string());
    println!("Ok or 0: {}", get_or_default_result(ok, 0));
    println!("Err or 99: {}", get_or_default_result(err, 99));
    println!("mapped: {}", transform_error(err).unwrap_err());
}
`,
      testCode: `#[test]
fn test_unwrap_or_ok() {
    let r: Result<int, String> = Ok(10);
    assert_eq!(get_or_default_result(r, 0), 10);
}

#[test]
fn test_unwrap_or_err() {
    let r: Result<int, String> = Err("fail".to_string());
    assert_eq!(get_or_default_result(r, 42), 42);
}

#[test]
fn test_unwrap_or_ok_ignores_fallback() {
    let r: Result<int, String> = Ok(5);
    assert_eq!(get_or_default_result(r, 999), 5);
}

#[test]
fn test_unwrap_or_negative_fallback() {
    let r: Result<int, String> = Err("err".to_string());
    assert_eq!(get_or_default_result(r, -1), -1);
}

#[test]
fn test_transform_error_on_err() {
    let r: Result<int, String> = Err("oops".to_string());
    let result = transform_error(r);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "OOPS");
}

#[test]
fn test_transform_error_passes_ok() {
    let r: Result<int, String> = Ok(42);
    let result = transform_error(r);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_transform_error_empty_string() {
    let r: Result<int, String> = Err("".to_string());
    let result = transform_error(r);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "");
}
`,
    },
    {
      id: 'try-operator',
      title: 'The ? Operator',
      instructions: `## The Try Operator \`?\`

The \`?\` operator is syntactic sugar for "unwrap or return early":

\`\`\`oxy
let val = fallible_operation()?;
// If Ok(v) → val = v
// If Err(e) → return Err(e) from the enclosing function
\`\`\`

This eliminates deeply nested \`match\` chains. It works with both \`Result\` and \`Option\`. The enclosing function must return a compatible type.

**Your task:** Implement \`read_and_add\` that parses two strings to integers using \`?\` and returns their sum.`,
      hints: [
        'Use `s.parse_int().map_err(|e| f"parse error: {e}")?` for each parse.',
        'The `?` operator short-circuits on the first error.',
        'The function returns `Result<int, String>` so `?` propagates `Err` values.',
      ],
      initialCode: `fn read_and_add(a: String, b: String) -> Result<int, String> {
    // TODO: parse both strings with ? and return their sum
    // Tip: s.parse_int().map_err(|_| "parse error".to_string())?
    let x = 0;  // placeholder — replace with parse + ?
    let y = 0;  // placeholder — replace with parse + ?
    Ok(x + y)
}

fn main() {
    match read_and_add("10".to_string(), "20".to_string()) {
        Ok(v) => println!("10 + 20 = {}", v),
        Err(e) => println!("error: {}", e),
    }
}
`,
      testCode: `#[test]
fn test_add_two_valid() {
    let r = read_and_add("10".to_string(), "20".to_string());
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), 30);
}

#[test]
fn test_add_negative() {
    let r = read_and_add("-5".to_string(), "3".to_string());
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), -2);
}

#[test]
fn test_add_zero() {
    let r = read_and_add("0".to_string(), "0".to_string());
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), 0);
}

#[test]
fn test_add_positive_negative_to_zero() {
    let r = read_and_add("10".to_string(), "-10".to_string());
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), 0);
}

#[test]
fn test_first_arg_invalid() {
    let r = read_and_add("abc".to_string(), "10".to_string());
    assert!(r.is_err());
}

#[test]
fn test_second_arg_invalid() {
    let r = read_and_add("10".to_string(), "xyz".to_string());
    assert!(r.is_err());
}

#[test]
fn test_both_invalid() {
    let r = read_and_add("foo".to_string(), "bar".to_string());
    assert!(r.is_err());
}

#[test]
fn test_large_sum() {
    let r = read_and_add("500000".to_string(), "300000".to_string());
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), 800000);
}
`,
    },
    {
      id: 'combinators',
      title: 'Combinators',
      instructions: `## Chaining with Combinators

Combinators let you transform and chain fallible values without explicit \`match\` blocks:

- \`map(f)\` — transform the inner value (no-op on None/Err)
- \`and_then(f)\` — chain a fallible operation (flat_map)
- \`or_else(f)\` — provide an alternative on None/Err

Combinators compose into clean pipelines: \`opt.map(f).and_then(g).unwrap_or(d)\`.

**Your task:** Complete \`double_value\` using \`map\`, and \`chain_operations\` using \`and_then\`.`,
      hints: [
        '`opt.map(|x| x * 2)` doubles the value if Some, passes through None.',
        '`opt.and_then(|x| if x > 3 { Some(x * 2) } else { None })` conditionally chains.',
        'Chain them: `opt.map(a).and_then(b)` — clean and composable.',
      ],
      initialCode: `fn double_value(opt: Option<int>) -> Option<int> {
    // TODO: use map to double the inner value
    None  // placeholder — replace with opt.map(|x| x * 2)
}

fn chain_operations(opt: Option<int>) -> Option<int> {
    // TODO: use and_then — if value > 3, return Some(value * 2); otherwise None
    None  // placeholder — replace with opt.and_then(...)
}

fn main() {
    let doubled = double_value(Some(21));
    println!("21 doubled = {}", doubled.unwrap_or(0));

    let chained = chain_operations(Some(5));
    println!("5 chain = {}", chained.unwrap_or(0));

    let filtered = chain_operations(Some(1));
    println!("1 chain is none: {}", filtered.is_none());
}
`,
      testCode: `#[test]
fn test_double_value_some() {
    let r = double_value(Some(10));
    assert!(r.is_some());
    assert_eq!(r.unwrap(), 20);
}

#[test]
fn test_double_value_none() {
    let r = double_value(None);
    assert!(r.is_none());
}

#[test]
fn test_double_value_negative() {
    let r = double_value(Some(-3));
    assert_eq!(r.unwrap(), -6);
}

#[test]
fn test_double_value_zero() {
    let r = double_value(Some(0));
    assert_eq!(r.unwrap(), 0);
}

#[test]
fn test_double_value_large() {
    let r = double_value(Some(500));
    assert_eq!(r.unwrap(), 1000);
}

#[test]
fn test_chain_greater_than_three() {
    let r = chain_operations(Some(5));
    assert!(r.is_some());
    assert_eq!(r.unwrap(), 10);
}

#[test]
fn test_chain_equal_to_three() {
    let r = chain_operations(Some(3));
    assert!(r.is_none());
}

#[test]
fn test_chain_less_than_three() {
    let r = chain_operations(Some(1));
    assert!(r.is_none());
}

#[test]
fn test_chain_none() {
    let r = chain_operations(None);
    assert!(r.is_none());
}

#[test]
fn test_chain_negative_above_three() {
    let r = chain_operations(Some(-1));
    assert!(r.is_none());
}
`,
    },
  ],
};
