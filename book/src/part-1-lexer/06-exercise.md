# Exercise: Build a Mini-Lexer

<!-- OPUS_FILL
Write a 1-2 paragraph framing for this exercise.
Key message: the exercise is not about building a complete lexer. It's about the moment
where something clicks — where you feel the mechanics in your hands, not just understand
them in your head. Something slightly playful about how getting it wrong is part of the deal.
-->

## Part A: Add a new operator to Oxy's lexer

**Goal:** Add support for the `**` (power/exponentiation) operator to the lexer.

**Expected behavior:**
- `2 ** 3` should lex to `[IntLiteral(2), StarStar, IntLiteral(3), Eof]`
- `2 * 3` should still lex to `[IntLiteral(2), Star, IntLiteral(3), Eof]`
- `2 *= 3` should still lex to `[IntLiteral(2), StarEq, IntLiteral(3), Eof]`

**Steps:**

1. Open `crates/oxy-core/src/lexer/token.rs`. Add a `StarStar` variant to `TokenKind`:
   ```rust
   /// `**` — exponentiation operator
   StarStar,
   ```

2. Add `"'**'"` to the `description()` method's `match` arm for `StarStar`.

3. Open `crates/oxy-core/src/lexer/mod.rs`. Find the `'*'` arm in `next_token`.
   It currently looks like:
   ```rust
   '*' => {
       if self.match_char('=') {
           TokenKind::StarEq
       } else {
           TokenKind::Star
       }
   }
   ```
   Modify it to also check for `**`:
   ```rust
   '*' => {
       if self.match_char('=') {
           TokenKind::StarEq
       } else if self.match_char('*') {
           TokenKind::StarStar
       } else {
           TokenKind::Star
       }
   }
   ```

4. Add a test to the `#[cfg(test)]` module in `mod.rs`:
   ```rust
   #[test]
   fn test_power_operator() {
       assert_eq!(
           kinds("2 ** 3"),
           vec![
               TokenKind::IntLiteral(2, IntegerSuffix::None),
               TokenKind::StarStar,
               TokenKind::IntLiteral(3, IntegerSuffix::None),
               TokenKind::Eof,
           ]
       );
   }
   ```

5. Run the tests:
   ```bash
   docker compose run --rm dev bash -c "cargo test -p oxy-core -- lexer"
   ```

**Expected result:** All lexer tests pass, including your new one.

**Note:** Adding `StarStar` to the token type will likely break compilation elsewhere — the
`description()` match in `token.rs` will complain that `StarStar` is not handled. The compiler
will tell you exactly where. Fix each one. This is the exhaustive-match guarantee in action.

---

## Part B: Add a `#!` shebang comment

Unix scripts often start with a shebang line: `#!/usr/bin/env oxy`. This should be treated as
a comment by the lexer (ignored entirely), so that `.ox` files can be made executable.

**Expected behavior:**
- A `#!` at the very start of the file (position 0) should skip everything until the end of the line
- `#` not followed by `!` should still produce `TokenKind::Hash` (used for attributes like `#[test]`)
- A `#!` on any line other than line 1 should probably be an error or just a `Hash` + `Bang`

**This is open-ended.** There is no single right answer. Think through:
1. Where in `next_token` should the check go? (Hint: is `#` handled before or after whitespace skipping?)
2. What state does the lexer need to track whether it's at position 0?
3. How do you skip "until end of line"?

Try to implement it. If you get stuck, look at how `skip_whitespace_and_comments` handles
`//` single-line comments — the shebang logic is very similar.

---

## Part C: Understand this existing test

Read this test in `crates/oxy-core/src/lexer/mod.rs` and explain, in your own words, why it exists:

```rust
fn test_range_vs_float() {
    assert_eq!(
        kinds("0..10"),
        vec![
            TokenKind::IntLiteral(0, IntegerSuffix::None),
            TokenKind::DotDot,
            TokenKind::IntLiteral(10, IntegerSuffix::None),
            TokenKind::Eof,
        ]
    );
}
```

Questions to answer:
1. What would go wrong if the lexer incorrectly tokenized `0..10` as `FloatLiteral(0.0)` followed by `IntLiteral(10)`?
2. Where in `scan_number` does the lexer decide whether a `.` is part of a float or the start of a `..` range?
3. Write a similar edge-case test for `1.0..5.0` (a float range). What should the token list be?

---

## Checking your work

```bash
docker compose run --rm dev bash -c "cargo test -p oxy-core -- lexer"
```

All tests in the `lexer` module should pass. If something breaks unexpectedly, run with
`-- --nocapture` to see test output, and add `dbg!(tokens)` to print the token list in
failing tests.
