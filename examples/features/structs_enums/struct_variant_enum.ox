// === Feature: struct-style enum variants type as the enum, and match
//                arms after a tuple/struct-variant pattern don't underflow ===
// Two bugs lived here together:
//   1) `Shape::Rectangle { w, h }` initializer was typed as
//      `Shape::Rectangle` instead of `Shape`, so passing it to a fn
//      accepting `Shape` was wrongly rejected.
//   2) After a successful EnumVariant arm, the *next* match arm's
//      prelude `Pop` ran even though `EnumVariantEqual` had already
//      consumed the scrutinee — the spurious Pop dipped into the
//      caller's frame, corrupting downstream `println`/`?`/etc.

enum Shape {
    Circle(float),
    Rectangle { w: float, h: float },
    Nothing,
}

fn area(s: Shape) -> float {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rectangle { w, h } => w * h,
        Shape::Nothing => 0.0,
    }
}

#[test]
fn test_struct_variant_initializer_types_as_enum() {
    // Passes the type checker only if `Shape::Rectangle { ... }`'s
    // inferred type is `Shape`, not `Shape::Rectangle`.
    let r: Shape = Shape::Rectangle { w: 4.0, h: 6.0 };
    let _ = area(r);
}

#[test]
fn test_match_after_struct_variant_no_underflow() {
    // The user-reported crash: `circle area: ...` prints fine but the
    // second `println` panics with VM stack underflow because the
    // EnumVariant arm consumed the scrutinee and the next iter's Pop
    // ate from the caller's frame.
    let c = Shape::Circle(5.0);
    let ac = area(c);
    assert(ac > 78.0 && ac < 79.0);

    let r = Shape::Rectangle { w: 4.0, h: 6.0 };
    assert_eq(area(r) as int, 24);

    let n = Shape::Nothing;
    assert_eq(area(n), 0.0);
}
