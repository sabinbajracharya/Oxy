# From Text to Machine Code

[Introduction](README.md)

---

# Part 0 — The Myth

- [Why We Built a Language](part-0-prologue/00-why-we-built-this.md)
- [The Origin Story: Ferrite → Oxide → Oxy](part-0-prologue/01-origin-story.md)
- [What You Will Learn](part-0-prologue/02-what-you-will-learn.md)
- [How to Read This Book](part-0-prologue/03-how-to-read.md)

---

# Part 1 — Words: The Lexer

- [What Is a Token?](part-1-lexer/01-what-is-a-token.md)
- [How Lexers Work](part-1-lexer/02-how-lexers-work.md)
- [Rust Concepts: Enums and Match](part-1-lexer/03-rust-enums-and-match.md)
- [Oxy's Lexer: A Full Walkthrough](part-1-lexer/04-oxy-lexer-walkthrough.md)
- [Spans: How Error Messages Know Where You Went Wrong](part-1-lexer/05-spans-and-errors.md)
- [Exercise: Build a Mini-Lexer](part-1-lexer/06-exercise.md)

---

# Part 2 — Structure: The Parser and AST

- [What Is an AST?](part-2-parser-and-ast/01-what-is-an-ast.md)
- [Pratt Parsing: The Elegant Algorithm](part-2-parser-and-ast/02-pratt-parsing.md)
- [Rust Concepts: Box, Recursive Enums, and the Heap](part-2-parser-and-ast/03-rust-box-and-recursive-types.md)
- [Oxy's AST: Every Node Explained](part-2-parser-and-ast/04-oxy-ast-walkthrough.md)
- [Oxy's Parser: A Full Walkthrough](part-2-parser-and-ast/05-oxy-parser-walkthrough.md)
- [Grammar Decisions: Why Oxy Looks Like Rust](part-2-parser-and-ast/06-grammar-decisions.md)
- [Exercise: Parse Simple Function Definitions](part-2-parser-and-ast/07-exercise.md)

---

# Part 3 — Meaning: The Tree-Walking Interpreter

- [The Simplest Way to Run Code](part-3-tree-walker/01-the-simplest-execution.md)
- [Environments and Scope](part-3-tree-walker/02-environments-and-scope.md)
- [Rust Concepts: HashMap, Box<dyn Trait>, and Recursion](part-3-tree-walker/03-rust-hashmaps-and-traits.md)
- [Oxy's Original Interpreter: A Walkthrough](part-3-tree-walker/04-oxy-tree-walker-walkthrough.md)
- [It Works! So Why Isn't This Enough?](part-3-tree-walker/05-why-not-enough.md)
- [Exercise: Add a Built-In Function](part-3-tree-walker/06-exercise.md)

---

# Part 4 — Knowing: The Type Checker

- [What Is a Type System?](part-4-type-checker/01-what-is-a-type-system.md)
- [Inference vs Annotation](part-4-type-checker/02-inference-vs-annotation.md)
- [Rust Concepts: Enums as Tagged Unions, Pattern Matching](part-4-type-checker/03-rust-enums-as-tagged-unions.md)
- [The Two-Pass Design: Collect Then Check](part-4-type-checker/04-two-pass-design.md)
- [Oxy's Type Checker: A Full Walkthrough](part-4-type-checker/05-oxy-type-checker-walkthrough.md)
- [Field Visibility and Module Boundaries](part-4-type-checker/06-field-visibility.md)
- [Exercise: Add a Type Error](part-4-type-checker/07-exercise.md)

---

# Part 5 — A Better Runtime: The Stack-Based VM

- [Why Tree-Walking Is Slow](part-5-stack-vm/01-why-tree-walking-is-slow.md)
- [Stack Machines: The Mental Model](part-5-stack-vm/02-stack-machines.md)
- [Bytecode: A Language Between Languages](part-5-stack-vm/03-bytecode.md)
- [Oxy's Stack VM: What It Was](part-5-stack-vm/04-oxy-stack-vm-walkthrough.md)
- [Why We Outgrew It](part-5-stack-vm/05-why-we-outgrew-it.md)
- [Exercise: Trace a Program on the Stack](part-5-stack-vm/06-exercise.md)

---

# Part 6 — Registers and Graphs: The IR

- [Why Register Machines Beat Stacks](part-6-register-ir/01-registers-vs-stacks.md)
- [Basic Blocks and Control Flow Graphs](part-6-register-ir/02-basic-blocks-and-cfg.md)
- [The IR as a Language Between Languages](part-6-register-ir/03-ir-as-language.md)
- [Rust Concepts: Ownership, Vec, and Indices](part-6-register-ir/04-rust-ownership-and-indices.md)
- [IrOp and Terminator: Every Instruction Explained](part-6-register-ir/05-irop-and-terminator.md)
- [Oxy's IR Gen: AST → IrFunction](part-6-register-ir/06-oxy-ir-gen-walkthrough.md)
- [Reading Oxy IR with OXY_VM_TRACE=1](part-6-register-ir/07-reading-ir-traces.md)
- [Exercise: Trace a Program Through the IR](part-6-register-ir/08-exercise.md)

---

# Part 7 — Native Speed: The Cranelift JIT

- [What Is JIT Compilation?](part-7-jit-cranelift/01-what-is-jit.md)
- [Cranelift: The Rust-Native Code Generator](part-7-jit-cranelift/02-cranelift-concepts.md)
- [Rust Concepts: Unsafe, Raw Pointers, and Memory](part-7-jit-cranelift/03-rust-unsafe-and-memory.md)
- [From IR Ops to CLIF: The Codegen Walkthrough](part-7-jit-cranelift/04-ir-to-clif-walkthrough.md)
- [The FFI Bridge: How Rust Becomes a Runtime](part-7-jit-cranelift/05-ffi-bridge.md)
- [Value Representation: How Oxy Stores Everything](part-7-jit-cranelift/06-value-representation.md)
- [War Stories: From 129 Failures to Zero](part-7-jit-cranelift/07-war-stories.md)
- [Exercise: Add a New IR Instruction](part-7-jit-cranelift/08-exercise.md)

---

# Part 8 — Running Everywhere: The WASM Interpreter

- [What Is WebAssembly and Why Does It Matter?](part-8-wasm-interpreter/01-what-is-wasm.md)
- [The Problem: Cranelift Cannot Run in a Browser](part-8-wasm-interpreter/02-the-problem.md)
- [The Solution: Interpret the Same IR](part-8-wasm-interpreter/03-the-solution.md)
- [One IR, Two Backends: The Elegance of the Design](part-8-wasm-interpreter/04-one-ir-two-backends.md)
- [The Divergence Guards: How You Stop Backends from Drifting](part-8-wasm-interpreter/05-divergence-guards.md)
- [Oxy's Interpreter: A Full Walkthrough](part-8-wasm-interpreter/06-oxy-interpreter-walkthrough.md)
- [The Closure-Invoker Hook](part-8-wasm-interpreter/07-closure-invoker-hook.md)
- [Exercise: Add a New Op to Both Backends](part-8-wasm-interpreter/08-exercise.md)

---

# Part 9 — The Shell: Ecosystem and Tooling

- [Language Server Protocol: How IDEs Understand Your Code](part-9-ecosystem/01-lsp.md)
- [Oxy's LSP: A Walkthrough](part-9-ecosystem/02-oxy-lsp-walkthrough.md)
- [Package Managers: The Boring Essential](part-9-ecosystem/03-package-managers.md)
- [Tug: Oxy's Package Manager](part-9-ecosystem/04-oxy-tug-walkthrough.md)
- [The REPL and CLI](part-9-ecosystem/05-repl-and-cli.md)
- [The VS Code Extension](part-9-ecosystem/06-vscode-extension.md)

---

# Part 10 — Looking Back: The Retrospective

- [The Full Picture: Every Layer Explained](part-10-retrospective/01-full-picture.md)
- [500 Commits Later: What We'd Do Differently](part-10-retrospective/02-what-wed-do-differently.md)
- [On Building with AI: The Real Story](part-10-retrospective/03-on-building-with-ai.md)
- [Your Turn: What Can You Build Now?](part-10-retrospective/04-your-turn.md)
