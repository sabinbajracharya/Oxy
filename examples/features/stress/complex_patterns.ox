// === STRESS: complex pattern matching combinations ===

enum Event {
    Click(int, int),
    KeyPress(char),
    Quit,
    Resize { w: int, h: int },
}

// --- match with multiple data extraction ---
#[test]
fn test_match_tuple_variant_extracts_all() {
    let e = Event::Click(50, 100);
    let s = match e {
        Event::Click(x, y) => x + y,
        Event::KeyPress(_) => -1,
        Event::Quit => -2,
        Event::Resize { w, h } => w + h,
    };
    assert_eq(s, 150);
}

#[test]
fn test_match_struct_variant_extracts_all() {
    let e = Event::Resize { w: 800, h: 600 };
    let area = match e {
        Event::Click(_, _) => 0,
        Event::KeyPress(_) => 0,
        Event::Quit => 0,
        Event::Resize { w, h } => w * h,
    };
    assert_eq(area, 480000);
}

// --- match with guards on enum bindings ---
#[test]
fn test_match_guards_on_binding() {
    let e = Event::Click(5, 5);
    let kind = match e {
        Event::Click(x, y) if x == y => "diagonal",
        Event::Click(_, _) => "off-diagonal",
        _ => "not a click",
    };
    assert_eq(kind, "diagonal");
}

// --- match with literal in tuple position ---
enum Op {
    Add(int, int),
    Inc(int),
}

#[test]
fn test_match_literal_in_tuple_pos() {
    let op = Op::Inc(7);
    let r = match op {
        Op::Add(a, b) => a + b,
        Op::Inc(n) => n + 1,
    };
    assert_eq(r, 8);
}

// --- nested match ---
#[test]
fn test_deeply_nested_match() {
    let o: Option<Result<int, String>> = Some(Ok(42));
    let v = match o {
        Some(r) => match r {
            Ok(n) => n,
            Err(_) => -1,
        },
        None => -2,
    };
    assert_eq(v, 42);
}

// --- match returning closure ---
#[test]
fn test_match_returns_closure() {
    let pick = 1;
    let f = match pick {
        0 => |x: int| x + 1,
        1 => |x: int| x * 2,
        _ => |x: int| x,
    };
    assert_eq(f(5), 10);
}

// --- match on bool ---
#[test]
fn test_match_on_bool() {
    let b = false;
    let s = match b {
        true => "yes",
        false => "no",
    };
    assert_eq(s, "no");
}

// --- match with all-pattern coverage ---
enum Trio { A, B, C }

fn trio_to_int(t: Trio) -> int {
    match t {
        Trio::A => 1,
        Trio::B => 2,
        Trio::C => 3,
    }
}

#[test]
fn test_trio_exhaustive() {
    assert_eq(trio_to_int(Trio::A), 1);
    assert_eq(trio_to_int(Trio::B), 2);
    assert_eq(trio_to_int(Trio::C), 3);
}

// --- match on Option<T> from fn ---
fn safe_div(a: int, b: int) -> Option<int> {
    if b == 0 { None } else { Some(a / b) }
}

#[test]
fn test_match_option_returned_from_fn() {
    let v1 = match safe_div(10, 2) {
        Some(x) => x,
        None => -1,
    };
    let v2 = match safe_div(10, 0) {
        Some(x) => x,
        None => -1,
    };
    assert_eq(v1, 5);
    assert_eq(v2, -1);
}

// --- match with or-pattern on literal ---
#[test]
fn test_match_or_pattern_strings() {
    let day = "Sat";
    let r = match day {
        "Sat" | "Sun" => "weekend",
        "Mon" | "Tue" | "Wed" | "Thu" | "Fri" => "weekday",
        _ => "unknown",
    };
    assert_eq(r, "weekend");
}

// --- match inside fn returning tuple ---
fn classify(n: int) -> (String, int) {
    match n {
        0 => ("zero".to_string(), 0),
        n if n < 0 => ("neg".to_string(), -n),
        n if n < 10 => ("small".to_string(), n),
        n => ("big".to_string(), n),
    }
}

#[test]
fn test_match_returning_tuple() {
    let (a, b) = classify(7);
    assert_eq(a, "small");
    assert_eq(b, 7);
}

#[test]
fn test_match_returning_tuple_neg() {
    let (a, b) = classify(-3);
    assert_eq(a, "neg");
    assert_eq(b, 3);
}

// --- match used inside for body ---
#[test]
fn test_match_in_for_body() {
    let mut hits = 0;
    let mut misses = 0;
    for n in vec(1, 2, 3, 4, 5) {
        match n % 2 {
            0 => hits = hits + 1,
            _ => misses = misses + 1,
        }
    }
    assert_eq(hits, 2);
    assert_eq(misses, 3);
}

// --- if-let chained with else ---
#[test]
fn test_if_let_else_chain() {
    let opt: Option<int> = None;
    let r = if let Some(x) = opt {
        x * 10
    } else {
        99
    };
    assert_eq(r, 99);
}

// --- pattern bindings used in arm body ---
#[test]
fn test_pattern_bindings_in_complex_body() {
    let e = Event::Click(7, 12);
    let r = match e {
        Event::Click(x, y) => {
            let sum = x + y;
            let prod = x * y;
            sum + prod
        }
        _ => 0,
    };
    assert_eq(r, 19 + 84);  // 19 + 84 = 103
}
