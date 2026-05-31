# Pratt Parsing: The Elegant Algorithm

<!-- OPUS_FILL
Write a 2-3 paragraph intro.
The key observation: most parser algorithms for expressions are either too complex
(full parser generator tables) or too verbose (one function per precedence level, which
is the "recursive descent" approach that gives you 14 nested functions).

Pratt parsing is the elegant middle ground: one loop, a precedence number, and two
dispatch tables (prefix handlers, infix handlers). Bob Nystrom's "Crafting Interpreters"
popularized it. It's what Oxy uses.

The "aha!" to build toward: precedence is just a number you pass to a recursive call.
"Parse everything to my right that binds tighter than I do" — that's it.

Optionally: reference that Vaughan Pratt published this in 1973 and it was mostly ignored
for decades. It deserved better.
-->

## The problem with naive recursive descent

The obvious way to parse expressions is one function per precedence level:

```rust
fn parse_or() -> Expr { /* handles || */ }
fn parse_and() -> Expr { /* handles && */ }
fn parse_equality() -> Expr { /* handles ==, != */ }
fn parse_comparison() -> Expr { /* handles <, >, <=, >= */ }
fn parse_term() -> Expr { /* handles +, - */ }
fn parse_factor() -> Expr { /* handles *, / */ }
fn parse_unary() -> Expr { /* handles -, ! */ }
fn parse_primary() -> Expr { /* handles literals, identifiers, () */ }
```

Each function calls the next-higher-precedence function to parse its operands. This works
but gives you 8+ functions, each knowing about exactly one precedence level. Adding a new
operator means finding the right function and inserting it. Changing precedence means
restructuring the call chain.

Oxy uses **Pratt parsing** instead: one loop, a precedence number, and dispatch tables.

## The core idea: precedence as a number

In Pratt parsing, every operator has a precedence number. The core expression parser takes
a **minimum precedence** and keeps consuming operators as long as they bind at least that tightly:

```rust
// crates/oxy-core/src/parser/expr.rs
pub(super) fn parse_expr(&mut self, min_prec: Precedence) -> Result<Expr, PipelineError> {
    let mut left = self.parse_prefix()?;          // parse the left operand

    while !self.is_at_end() {
        let prec = Precedence::of_binary(self.peek_kind());
        if prec <= min_prec {
            break;                                // operator not strong enough — stop
        }
        left = self.parse_infix(left, prec)?;    // consume operator and right side
    }

    Ok(left)
}
```

That's the entire algorithm. `parse_prefix` handles things that appear at the start of an
expression (literals, identifiers, unary operators, parenthesized groups). `parse_infix`
handles binary operators that appear between two expressions.

## Oxy's precedence table

```rust
// crates/oxy-core/src/parser/mod.rs
enum Precedence {
    None       = 0,
    Assignment = 1,  // = += -= etc.
    Range      = 2,  // .. ..=
    Or         = 3,  // ||
    And        = 4,  // &&
    BitOr      = 5,  // |
    BitXor     = 6,  // ^
    BitAnd     = 7,  // &
    Equality   = 8,  // == !=
    Comparison = 9,  // < > <= >=
    Shift      = 10, // << >>
    Term       = 11, // + -
    Factor     = 12, // * / %
    Unary      = 13, // - ! (prefix)
    Call       = 14, // () .  (postfix)
}
```

Higher number = tighter binding. `*` (Factor=12) binds tighter than `+` (Term=11), which is
why `2 + 3 * 4` evaluates as `2 + (3 * 4)`.

## Tracing `2 + 3 * 4`

Let's trace the parser step by step for `2 + 3 * 4`, starting with `parse_expr(min_prec=None=0)`:

1. **`parse_prefix()`** → sees `IntLiteral(2)`, returns `Expr::IntLiteral(2)`. `left = 2`.

2. **Loop iteration 1:** peek at `+`. `Precedence::of_binary(Plus)` = `Term = 11`. Is `11 > 0`? Yes. Call `parse_infix(left=2, prec=11)`.

3. **`parse_infix`** consumes `+`, then calls `parse_expr(min_prec=11)` to get the right operand.

4. **Recursive `parse_expr(11)`:** `parse_prefix()` → `IntLiteral(3)`. `left = 3`.

5. **Recursive loop iteration 1:** peek at `*`. `Precedence::of_binary(Star)` = `Factor = 12`. Is `12 > 11`? Yes. Call `parse_infix(left=3, prec=12)`.

6. **`parse_infix`** consumes `*`, calls `parse_expr(min_prec=12)` → `parse_prefix()` → `IntLiteral(4)`. Loop: peek is EOF, stop. Returns `4`.

7. **`parse_infix` returns** `BinaryOp(3 * 4)`. Recursive loop: peek is EOF, stop. Recursive `parse_expr(11)` returns `BinaryOp(3 * 4)`.

8. **Back in outer `parse_infix`:** right side is `BinaryOp(3 * 4)`. Returns `BinaryOp(2 + BinaryOp(3 * 4))`.

9. **Outer loop:** peek is EOF, stop. Returns `BinaryOp(2 + BinaryOp(3 * 4))`.

The key moment is step 5: the recursive call sees `*` with precedence 12, which is greater than the minimum 11, so it consumes it. If we had `2 * 3 + 4` instead, step 5 would see `+` with precedence 11 — not greater than 11 — so it would stop and return, leaving `+` for the outer loop to handle.

**One rule produces all precedence behavior:** "keep consuming operators that bind tighter than my minimum."

## Right-associativity: assignment

Assignment is right-associative: `a = b = c` means `a = (b = c)`, not `(a = b) = c`.

In Pratt parsing, right-associativity is achieved by passing `prec - 1` as the minimum
for the right operand, instead of `prec`. This allows operators at the same level to be
consumed on the right:

```rust
// In parse_infix, for assignment operators:
TokenKind::Eq | TokenKind::PlusEq | ... => {
    // Pass Assignment-1 so right side can also be assignment (right-assoc)
    let right = self.parse_expr(Precedence::Assignment - 1)?;
    Expr::Assign { target: Box(left), value: Box(right), .. }
}
```

For left-associative operators (most of them), pass `prec` (the current level), which
prevents the right side from consuming another operator at the same level.

## Postfix operators: calls and field access

The dot (`.`) and call (`(`) operators appear after an expression, not before. In Pratt
terms, they are "infix" operators with high precedence (`Call = 14`):

```rust
// In parse_infix:
TokenKind::LParen => {
    // left is the callee expression
    let args = self.parse_call_args()?;
    Expr::Call { callee: Box(left), args, .. }
}
TokenKind::Dot => {
    let field = self.expect_ident()?;
    if self.check(&TokenKind::LParen) {
        // method call: obj.method(args)
        let args = self.parse_call_args()?;
        Expr::MethodCall { object: Box(left), method: field, args, .. }
    } else {
        // field access: obj.field
        Expr::FieldAccess { object: Box(left), field, .. }
    }
}
```

`a.b.c()` works naturally: `a.b` is parsed first (dot binds at level 14), then `.c()`
is parsed on the result. Left-to-right, as expected.

## The `no_struct_literal` disambiguation

There is one tricky case: `if score { ... }`. Should `score {` be parsed as a struct
initializer (`Point { x: 1 }`) or as an identifier followed by a block?

Oxy uses a context flag borrowed from Rust: `no_struct_literal`. When parsing the condition
of an `if`, `while`, or `for`, the parser sets this flag:

```rust
fn with_no_struct_literal<F, R>(&mut self, f: F) -> R {
    let saved = self.ctx.no_struct_literal;
    self.ctx.no_struct_literal = true;
    let result = f(self);
    self.ctx.no_struct_literal = saved;
    result
}
```

When the flag is set, seeing an identifier followed by `{` does not start a struct init —
it returns the identifier as a plain expression. The `{` starts the block of the `if`/`while`/`for`.

This is a real ambiguity in the grammar and this flag is how Rust solves it too.
