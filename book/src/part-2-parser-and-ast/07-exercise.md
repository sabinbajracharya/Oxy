# Exercise: Parse Simple Function Definitions

You're about to add a new expression form to a real compiler — a new AST node, a new keyword, a
new parser arm — and the steps below are not a simplified teaching exercise. They are, almost
exactly, what you'd do on an actual pull request to Oxy: touch the AST, touch the lexer, touch the
parser, write a test, and then chase the compiler's complaints until everything is green again.
That last part is the real lesson. The moment you add a variant to `Expr`, the build will break in
every `match` that didn't handle it, and the Rust compiler will hand you a to-do list of exactly
which files to visit next. Follow it. That's not the exercise going wrong — that's the exercise
working.

## Part A: Add a `typeof` expression

Add a `typeof expr` expression that returns the type name of its operand as a string.

**Example Oxy code that should work after your change:**
```rust
fn main() {
    let x = 42;
    println(typeof x);     // prints "int"
    println(typeof "hi");  // prints "String"
}
```

**Step 1: Add to the AST**

In `crates/oxy-core/src/ast/mod.rs`, add a new variant to `Expr`:

```rust
/// `typeof expr` — evaluates to the type name of expr as a string
TypeOf {
    expr: Box<Expr>,
    span: Span,
},
```

**Step 2: Add to the parser**

In `crates/oxy-core/src/parser/expr.rs`, in the `parse_prefix` function, add a new arm
to the match. You need to add `typeof` as a keyword first:

- In `lexer/token.rs`, add `TypeOf` to `TokenKind` and add `"typeof" => Some(Self::TypeOf)` to `from_keyword`.
- In `parser/expr.rs`, add to `parse_prefix`:

```rust
TokenKind::TypeOf => {
    let start = self.current_span();
    self.advance();
    let expr = self.parse_prefix()?;  // parse the operand
    Ok(Expr::TypeOf {
        expr: Box::new(expr),
        span: self.merge_spans(start, expr.span()),
    })
}
```

**Step 3: Write a test**

Add a test in the lexer tests (check that `typeof` tokenizes as `TypeOf`). Then add a
parser test in `crates/oxy-core/tests/vm_tests/` that verifies `typeof 42` produces
a string result.

Note: the compiler will fail after your AST change because the type checker and IR gen
do not know about `TypeOf` yet. Each failed compile will tell you exactly where you
need to add a case. Follow the compiler's guidance.

---

## Part B: Understand the `Stmt::Expr { has_semicolon }` field

Write a small Oxy program that demonstrates the difference between a block that returns
a value and a block whose result is discarded:

```rust
fn returns_value() -> int {
    let x = 5;
    x + 1      // no semicolon — this is the return value
}

fn discards_value() -> int {
    let x = 5;
    x + 1;     // semicolon — result discarded, returns ()
    0          // this is the return value
}
```

Now look at how `parse_stmt` handles these two cases. Specifically: what AST does
`x + 1` (no semicolon) produce vs `x + 1;` (with semicolon)?

Answer these questions by reading `parser/stmt.rs`:
1. When `has_semicolon` is `false` on the last statement in a block, what does IR gen do with that expression?
2. If a function declares `-> int` but its body ends with `x + 1;` (semicolon), what should the type checker do?

---

## Part C: Trace a match expression through the AST

Write this Oxy program, run it through `parse`, and inspect the resulting AST:

```rust
fn describe(x: int) -> String {
    match x {
        0 => "zero",
        1 => "one",
        _ => "other",
    }
}
```

To inspect the AST, add a temporary `println!("{:#?}", program)` after parsing in a test,
or use `OXY_VM_TRACE=1` to see the IR (which will show you indirectly what the parser produced).

Questions:
1. What is the type of `arms` in `Expr::Match`?
2. What is a `MatchArm`? (look it up in `ast/mod.rs`)
3. What does `Pattern::Wildcard` look like? How does `_` become a wildcard pattern?

---

## Checking your work

```bash
docker compose run --rm dev bash -c "cargo test -p oxy-core -- parser"
```

For Part A, the build will break in multiple places after each step. Let the compiler
guide you — it will list every place that needs a new case for `Expr::TypeOf`. This is
the exhaustive-match guarantee: you cannot add a new AST node variant without updating
every match that covers `Expr`.
