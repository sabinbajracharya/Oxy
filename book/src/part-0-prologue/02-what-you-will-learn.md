# What You Will Learn

<!-- OPUS_FILL
Write a 2-paragraph intro that sets expectations honestly.
First para: what the reader will genuinely be able to do after finishing this book.
Second para: what this book does NOT teach (theory, formal grammars, optimization passes) and why
that's fine — those things are in other books. This book teaches you to read and modify a real
compiler, not to design one from scratch in a vacuum.
Tone: honest, grounded. Not hyping it up.
-->

## Concrete skills

By the end of this book you will be able to:

**Navigate the Oxy codebase without help.** Open any file in `crates/oxy-core/src/`, understand
what it does, trace a feature from source text through each pipeline stage to execution.

**Add a feature to the lexer or parser.** New keyword, new operator, new syntax form — you will
know exactly which files to touch and in what order.

**Read and write Oxy's IR.** Run `OXY_VM_TRACE=1` on any program, read the register IR output,
understand what each instruction does and why it was emitted.

**Understand what Cranelift does.** Not how to write a Cranelift backend from scratch, but how
to read `codegen.rs` and understand how Oxy IR ops become CLIF instructions that become machine code.

**Debug compiler test failures.** Given a failing `.ox` test, trace the failure through the pipeline:
is it a parse error? A type error? An IR gen bug? A codegen bug? You will know the diagnostic path.

**Write Rust code in this codebase.** The Rust concepts you need — enums, match, HashMap, Box,
traits, unsafe — are taught in context, at the moment you need them, anchored to real code.

## What this book does not cover

**Formal language theory.** Context-free grammars, pushdown automata, LR parse tables,
first/follow sets — none of this appears in this book. These are real, useful concepts, but
they are not what you need to understand Oxy. When we need a concept from formal theory, we
explain it from first principles with a concrete example.

**Optimization passes.** Dead code elimination, constant folding, inlining, register allocation —
Oxy does not implement these (yet). Cranelift handles some optimizations internally, but Oxy's
IR gen does no optimization. This book does not cover optimization theory.

**Type theory.** Hindley-Milner, System F, dependent types — none of this. Oxy's type checker
is a straightforward two-pass algorithm that most programmers can reason about without a type
theory background.

**How to design a language from scratch.** We explain the decisions that shaped Oxy's design, but
this book is about understanding an existing language, not the open-ended process of designing one.
That is a different book.

## The Rust you will learn

These are the specific Rust concepts this book teaches, organized by when they appear:

| Appears in | Concept | Why it matters |
|-----------|---------|----------------|
| Part 1 | `enum`, `match` | Token types are enums; matching on them is the entire lexer |
| Part 2 | `Box<T>`, recursive types | AST nodes contain child nodes — requires heap indirection |
| Part 3 | `HashMap`, trait objects | Environments are maps; the interpreter uses `dyn` dispatch |
| Part 4 | Enums as tagged unions | `TypeInfo` is an enum; pattern matching is type inference |
| Part 6 | `Vec`, indices, ownership basics | IR uses index-based references into `Vec` buffers |
| Part 7 | `unsafe`, raw pointers | The JIT hands memory directly to Cranelift — this is unavoidable |
| Part 8 | `cfg` attributes, feature flags | The interpreter is gated on `wasm32` via conditional compilation |

You do not need to understand all of Rust before Part 1. You need to understand each concept
before the section that uses it. That is exactly when we introduce it.
