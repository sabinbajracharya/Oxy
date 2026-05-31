# Exercise: Add a New IR Instruction

<!-- OPUS_FILL
Write a 1-paragraph framing. The exercise adds a feature to the JIT that goes through
all three layers: IR op → codegen → FFI. This is the full stack.
Frame it as: "You are about to add a feature that touches every layer of the JIT.
After this, you have read every layer at least once."
-->

## Part A: Add `oxy_is_truthy` as an FFI function

Currently, truthiness checks in Oxy (for `if` conditions, `while` conditions, `&&`, `||`)
are done inline by the codegen. Add a `oxy_is_truthy(ctx) -> i64` FFI function that:
- Pops a `Value` from the operand stack
- Returns `1` if the value is truthy, `0` otherwise
- `Value::Bool(true)` → 1, `Value::Bool(false)` → 0
- `Value::I64(0)` → 0, all other integers → 1
- `Value::Unit` → 0

**Steps:**

1. In `ffi/mod.rs`, add:
   ```rust
   #[no_mangle]
   extern "C" fn oxy_is_truthy(ctx: *mut JitContext) -> i64 {
       let ctx = unsafe { &mut *ctx };
       let val = unsafe { pop(ctx) };
       i64::from(val.is_truthy())
   }
   ```

2. Add it to `ffi_symbols()` and `ffi_decls()`.

3. Run the FFI consistency test:
   ```bash
   docker compose run --rm dev bash -c "cargo test -p oxy-core ffi_consistency"
   ```

4. Write an Oxy test:
   ```rust
   #[test]
   fn test_truthy_int() {
       assert_eq(if 1 { "truthy" } else { "falsy" }, "truthy");
   }
   ```

---

## Part B: Understand the JitContext buffer layout

Look at `crates/oxy-core/src/vm/jit/context.rs`. Draw the buffer layout for a function
with 3 locals and an operand stack with 2 values:

```
buffer: [local0, local1, local2, stack0, stack1, ...]
         ↑                       ↑
         base                    base + local_count
```

Answer:
1. What does `ctx.push_slot()` return? Where in the buffer does it point?
2. What prevents the operand stack from overwriting locals? (Hint: read the `buffer_size`
   calculation in `context.rs` or `jit/mod.rs`)
3. The CLAUDE.md documents "per-function local counts stored in the engine vs. inferred
   from main is the canonical example" of a buffer sizing bug. What went wrong? Where
   is the fix?

---

## Part C: Add an IR snapshot for a closure

Write an Oxy closure and look at the IR snapshot:

```rust
fn make_adder(x: int) -> fn(int) -> int {
    |y| x + y
}

fn main() {
    let add5 = make_adder(5);
    println(add5(3));
}
```

Run with `OXY_VM_TRACE=1` and find:
1. How is the closure created? What `CallBuiltin` function creates the closure value?
2. What are the `captures` in the closure's IR? How is `x` captured?
3. When `add5(3)` is called, how does the compiled code access the captured `x`?
   (Hint: look for `LoadLocal` with slot indices corresponding to captured variables)

---

## Part D: Cluster-finding practice

This is a thought exercise based on the war stories.

Suppose you add a new feature (say, `do..while` loops) and the test suite shows 12 new failures.
The failures are:

- `test_do_while_runs_once_when_false`
- `test_do_while_increments_counter`
- `test_nested_do_while`
- `test_do_while_with_break`
- `test_do_while_returns_value`
- `test_do_while_in_function`
- And 6 more similar tests.

Before looking at any test output, which single question would you ask first?
A) "What does each failing test case expect?"
B) "What does the IR look like for a `do..while` loop?"
C) "Is there a shared code path that all `do..while` tests go through?"
D) "Which individual test is failing for the simplest reason?"

Explain your choice. Then explain why choices A and D would be the slowest approach.
