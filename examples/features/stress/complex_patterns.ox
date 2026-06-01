// === STRESS: complex pattern matching combinations ===

enum Event {
    Click(Int, Int),
    KeyPress(char),
    Quit,
    Resize { w: Int, h: Int },
}

// --- match with multiple data extraction ---
#[test]
fn test_match_tuple_variant_extracts_all() {
    val e = Event::Click(50, 100);
    val s = match e {
        Event::Click(x, y) => x + y,
        Event::KeyPress(_) => -1,
        Event::Quit => -2,
        Event::Resize { w, h } => w + h,
    };
    assert::eq(s, 150);
}

#[test]
fn test_match_struct_variant_extracts_all() {
    val e = Event::Resize { w: 800, h: 600 };
    val area = match e {
        Event::Click(_, _) => 0,
        Event::KeyPress(_) => 0,
        Event::Quit => 0,
        Event::Resize { w, h } => w * h,
    };
    assert::eq(area, 480000);
}

// --- match with guards on enum bindings ---
#[test]
fn test_match_guards_on_binding() {
    val e = Event::Click(5, 5);
    val kind = match e {
        Event::Click(x, y) if x == y => "diagonal",
        Event::Click(_, _) => "off-diagonal",
        _ => "not a click",
    };
    assert::eq(kind, "diagonal");
}

// --- match with literal in tuple position ---
enum Op {
    Add(Int, Int),
    Inc(Int),
}

#[test]
fn test_match_literal_in_tuple_pos() {
    val op = Op::Inc(7);
    val r = match op {
        Op::Add(a, b) => a + b,
        Op::Inc(n) => n + 1,
    };
    assert::eq(r, 8);
}

// --- nested match ---
#[test]
fn test_deeply_nested_match() {
    val o: Option<Result<Int, String>> = Some(Ok(42));
    val v = match o {
        Some(r) => match r {
            Ok(n) => n,
            Err(_) => -1,
        },
        None => -2,
    };
    assert::eq(v, 42);
}

// --- match returning closure ---
#[test]
fn test_match_returns_closure() {
    val pick = 1;
    val f = match pick {
        0 => |x: Int| x + 1,
        1 => |x: Int| x * 2,
        _ => |x: Int| x,
    };
    assert::eq(f(5), 10);
}

// --- match on bool ---
#[test]
fn test_match_on_bool() {
    val b = false;
    val s = match b {
        true => "yes",
        false => "no",
    };
    assert::eq(s, "no");
}

// --- match with all-pattern coverage ---
enum Trio { A, B, C }

fn trio_to_int(t: Trio) -> Int {
    match t {
        Trio::A => 1,
        Trio::B => 2,
        Trio::C => 3,
    }
}

#[test]
fn test_trio_exhaustive() {
    assert::eq(trio_to_int(Trio::A), 1);
    assert::eq(trio_to_int(Trio::B), 2);
    assert::eq(trio_to_int(Trio::C), 3);
}

// --- match on Option<T> from fn ---
fn safe_div(a: Int, b: Int) -> Option<Int> {
    if b == 0 { None } else { Some(a / b) }
}

#[test]
fn test_match_option_returned_from_fn() {
    val v1 = match safe_div(10, 2) {
        Some(x) => x,
        None => -1,
    };
    val v2 = match safe_div(10, 0) {
        Some(x) => x,
        None => -1,
    };
    assert::eq(v1, 5);
    assert::eq(v2, -1);
}

// --- match with or-pattern on literal ---
#[test]
fn test_match_or_pattern_strings() {
    val day = "Sat";
    val r = match day {
        "Sat" | "Sun" => "weekend",
        "Mon" | "Tue" | "Wed" | "Thu" | "Fri" => "weekday",
        _ => "unknown",
    };
    assert::eq(r, "weekend");
}

// --- match inside fn returning tuple ---
fn classify(n: Int) -> (String, Int) {
    match n {
        0 => ("zero".to_string(), 0),
        n if n < 0 => ("neg".to_string(), -n),
        n if n < 10 => ("small".to_string(), n),
        n => ("big".to_string(), n),
    }
}

#[test]
fn test_match_returning_tuple() {
    val (a, b) = classify(7);
    assert::eq(a, "small");
    assert::eq(b, 7);
}

#[test]
fn test_match_returning_tuple_neg() {
    val (a, b) = classify(-3);
    assert::eq(a, "neg");
    assert::eq(b, 3);
}

// --- match used inside for body ---
#[test]
fn test_match_in_for_body() {
    var hits = 0;
    var misses = 0;
    for n in [1, 2, 3, 4, 5] {
        match n % 2 {
            0 => hits = hits + 1,
            _ => misses = misses + 1,
        }
    }
    assert::eq(hits, 2);
    assert::eq(misses, 3);
}

// --- if-val chained with else ---
#[test]
fn test_if_let_else_chain() {
    val opt: Option<Int> = None;
    val r = if val Some(x) = opt {
        x * 10
    } else {
        99
    };
    assert::eq(r, 99);
}

// --- pattern bindings used in arm body ---
#[test]
fn test_pattern_bindings_in_complex_body() {
    val e = Event::Click(7, 12);
    val r = match e {
        Event::Click(x, y) => {
            val sum = x + y;
            val prod = x * y;
            sum + prod
        }
        _ => 0,
    };
    assert::eq(r, 19 + 84);  // 19 + 84 = 103
}
