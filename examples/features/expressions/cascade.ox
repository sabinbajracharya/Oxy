// Cascade block `..{ }` examples — scoped mutation on a receiver.
// `receiver ..{ .field = val; .method(); }` — leading dot targets the receiver.

// --- Method calls ---

#[test]
fn test_cascade_method_chain() {
    var v = [1, 2, 3];
    v..{
        .push(4);
        .push(5);
    };
    assert_eq(v.len(), 5);
    assert_eq(v[3], 4);
    assert_eq(v[4], 5);
}

#[test]
fn test_cascade_single_call() {
    var v = [1, 2];
    v..{ .push(3); };
    assert_eq(v.len(), 3);
    assert_eq(v[2], 3);
}

// --- Field assignment ---

struct Point {
    x: Int,
    y: Int,
}

#[test]
fn test_cascade_single_field() {
    var p = Point { x: 0, y: 0 };
    p..{ .x = 10; .y = 20; };
    assert_eq(p.x, 10);
    assert_eq(p.y, 20);
}

// --- Builder pattern ---

struct Button {
    text: String,
    width: Int,
    height: Int,
}

#[test]
fn test_cascade_builder() {
    var btn = Button { text: "", width: 0, height: 0 };
    btn..{
        .text = "Save";
        .width = 120;
        .height = 40;
    };
    assert_eq(btn.text, "Save");
    assert_eq(btn.width, 120);
    assert_eq(btn.height, 40);
}
