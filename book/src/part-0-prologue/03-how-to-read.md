# How to Read This Book

<!-- OPUS_FILL
Write a short (2 paragraph) intro.
First para: this is a book you read with a code editor open next to it.
Second para: the exercises are not optional — they are the moments where intuition forms.
Tone: direct, encouraging, slightly insistent about the exercises.
-->

## Keep the Oxy codebase open

Every code snippet in this book is pulled directly from the Oxy source. The path and line
reference are always included. When you see:

```rust
// crates/oxy-core/src/lexer/mod.rs
pub fn tokenize(src: &str) -> Vec<Token> {
```

...open that file. Read the surrounding context. Look at what comes before and after. The book
explains the snippet; the file shows how it fits into the whole.

This is especially important for the walkthrough chapters (04-oxy-lexer-walkthrough,
05-oxy-parser-walkthrough, etc.). These chapters are meant to be read side-by-side with the source.

## Building the book locally

To read this as a formatted web page rather than raw Markdown:

```bash
docker compose run --rm book        # build once → book/book/index.html
docker compose run --rm book-serve  # live server → open localhost:3000
```

The live server reloads when you edit any source file, which is useful when adding new content.

## How the code snippets stay current

Code in this book is not copy-pasted. The `{{#include}}` directives pull code directly from
the Oxy source at build time using mdBook's include feature. This means:

- If a function is renamed in the source, the book shows the new name
- If code is moved to a different file, the book's path reference breaks loudly at build time
- Stale snippets cannot silently accumulate

When you add anchors to source files for new book sections, follow the existing pattern:

```rust
// ANCHOR: tokenize-loop
for ch in src.chars() {
    // ...
}
// ANCHOR_END: tokenize-loop
```

Then reference it in the book with:

```
{{#include ../../../crates/oxy-core/src/lexer/mod.rs:tokenize-loop}}
```

## The exercises

Each part ends with an exercise. The exercises ask you to make a small change to the Oxy
codebase — add a keyword, extend an AST node, handle a new IR op. They are sized to be
completable in 1-3 hours.

Do the exercises. The text explains concepts; the exercises build intuition. There is a
specific kind of understanding that only comes from making something break and then fixing it.
The exercises are designed to put you in that position.

Exercise solutions are not provided. This is intentional. The goal is not to produce a correct
answer — it is to spend time exploring the code. Getting stuck and then unstuck is the point.

## Parts you can skip (if you already know the topic)

| If you already know... | You can skim... |
|----------------------|-----------------|
| How lexers work | Part 1 (but read the Oxy walkthrough) |
| Pratt parsing | Part 2 chapter 2 (but read the AST walkthrough) |
| Stack machines | Part 5 chapter 2 |
| What JIT means | Part 7 chapter 1 |
| What WebAssembly is | Part 8 chapter 1 |

Do not skip the Oxy-specific walkthrough chapters even if you know the concept. Those chapters
are where you learn the codebase specifically, not just the concept in general.

## A note on the Rust chapters

Each part has a "Rust Concepts" chapter that teaches the specific Rust features needed for
that part. If you already know Rust, these chapters are still worth skimming — they explain
which Rust patterns Oxy uses and why, which is different from how Rust is usually taught.

If you are new to Rust: read these chapters carefully. Then read them again after reading the
technical chapters in the same part. The second read will make more sense because you will
have seen the concepts in context.
