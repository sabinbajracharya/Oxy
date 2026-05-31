# Exercise: Add a Built-In Function

A small honesty note before you start: you can't do an exercise on the tree-walker itself, because
the tree-walker is gone — deleted from `main`, living only in git history. But that's fine, because
the *ideas* the tree-walker taught us didn't leave with it. Environments, the `Value` enum, builtin
dispatch, the way variable state has to live *somewhere* at runtime — all of that is still here, just
relocated into the parts of the codebase that survived. So these exercises work on the current code,
and where they touch a living component, they ask you to connect it back to the tree-walking concept
it descends from. Let's work with what's here.

## Part A: Add a built-in function to Oxy's stdlib

Oxy's standard library is registered in `crates/oxy-core/src/stdlib/registry.rs`.
Add a new built-in function `math::clamp(value, min, max)` that clamps a value to a range.

**Expected Oxy behavior:**
```rust
fn main() {
    println(math::clamp(5, 0, 10));   // 5
    println(math::clamp(-3, 0, 10));  // 0
    println(math::clamp(15, 0, 10));  // 10
}
```

**Step 1: Find the math module**

Open `crates/oxy-core/src/stdlib/math.rs`. This is where `math::abs`, `math::min`,
`math::max` etc. are implemented. Find how an existing function like `math::min` is structured.

**Step 2: Add `clamp` to the math module**

Add a `clamp` function following the same pattern as `min`/`max`. The signature in Oxy terms:
takes three `int` (or `float`) arguments, returns the same type.

**Step 3: Register it**

In `stdlib/registry.rs`, add `"math::clamp"` to the function table.

**Step 4: Add to symbols**

In `crates/oxy-core/src/symbols.rs`, add `CLAMP` to the math module constants.

**Step 5: Write a test**

Add a test in `examples/features/numbers/math_clamp.ox`:
```rust
#[test]
fn test_clamp_in_range() {
    assert_eq(math::clamp(5, 0, 10), 5);
}

#[test]
fn test_clamp_below_min() {
    assert_eq(math::clamp(-3, 0, 10), 0);
}

#[test]
fn test_clamp_above_max() {
    assert_eq(math::clamp(15, 0, 10), 10);
}
```

Run the tests:
```bash
docker compose run --rm dev bash -c "cargo test -p oxy-core -- feature_examples"
```

---

## Part B: Trace a `Value` through the pipeline

Run this program with `OXY_VM_TRACE=1`:

```rust
fn main() {
    let x = 42;
    let y = x + 1;
    println(y);
}
```

```bash
OXY_VM_TRACE=1 docker compose run --rm dev bash -c \
  "cargo run --bin oxy -- run examples/hello.ox" 2>&1 | head -40
```

Find the IR instructions that correspond to `let x = 42` and `x + 1`. Questions:
1. What register is `x` stored in?
2. What `IrOp` is emitted for `x + 1`?
3. Is `42` stored as a `Value::Int` in the IR, or as an immediate integer?

---

## Part C: Understand the `Value` enum

Open `crates/oxy-core/src/types/mod.rs`. The `Value` enum is the runtime representation
of every Oxy value — used by both the JIT (at FFI boundaries) and the wasm interpreter.

Count the variants. Then answer:
1. How does `Value::Struct` store its fields? (HashMap or Vec? Named or indexed?)
2. How does `Value::Closure` capture its environment?
3. What is `Value::FnPointer`? How is it different from `Value::Closure`?

The answers are in the source. Read the comments — they explain the trade-offs.

---

## Reflection question (no code required)

The tree-walker is gone. But the `Environment` type in `env/mod.rs` still exists and is
still used by the wasm IR interpreter. Why does the wasm interpreter need `Environment`
when the JIT does not?

Hint: the JIT resolves variables at compile time (they become register slots). The wasm
interpreter walks IR at runtime. Where does variable *state* live in each case?
