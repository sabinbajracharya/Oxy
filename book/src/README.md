# From Text to Machine Code

## Building the Oxy Programming Language

This book tells the real story of building a compiled programming language from scratch — in Rust, over 500 commits, with every wrong turn included.

We built **Oxy**: a language with Rust-like syntax, a Cranelift JIT backend for native execution, and a wasm IR interpreter for the browser. By the end of this book, you will understand every layer of that stack and be able to navigate and modify the Oxy codebase yourself.

### What this book is

- A complete guide to how programming languages work — from characters to machine code
- A walkthrough of the **actual Oxy source code**, not toy examples
- An honest account of the decisions, dead ends, and debugging sessions that shaped the project
- A Rust tutorial, embedded where you need it — just in time, not upfront

### What this book is not

- A cleaned-up ideal path. We made mistakes. We retired entire subsystems. Those stories are in here.
- A reference manual. For that, read the `README.md` files in each source folder.
- Boring.

### The pipeline

Every Oxy program travels this path:

```
source text
    │
    ▼
 Lexer          → stream of tokens
    │
    ▼
 Parser         → Abstract Syntax Tree (AST)
    │
    ▼
 Type Checker   → typed, validated AST
    │
    ▼
 IR Gen         → Register IR + Control Flow Graph
    │
    ├──────────────────────┐
    ▼                      ▼
 Cranelift JIT         IR Interpreter
 (native: x86/arm)     (wasm32: browser)
    │                      │
    ▼                      ▼
 machine code          walked directly
```

### How to read this book

Each part builds on the previous one. Read them in order. Every part ends with an exercise — do it. The exercises are the moments where things click.

Code snippets are pulled directly from the Oxy source — they are not copied. When Oxy changes, the book rebuilds with the current code.

### Prerequisites

- You can write a basic program in some language (any language)
- You are curious about how programming languages work
- You do not need to know Rust — we teach it as we go

---

*Built on [Oxy](https://github.com/your-repo/oxy) — 500+ commits of real evolution.*
