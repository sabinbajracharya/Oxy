# What You Will Learn

Let's be precise about the deal on offer, because vague promises help nobody. When you finish
this book, you will be able to open the Oxy source tree and *read it* — not squint at it and hope,
but actually trace a feature from the text you typed all the way down to the machine code that
runs it, naming every stage it passes through. You'll be able to add a keyword, a new operator,
or a new method, and know which files to touch in which order. You'll be able to dump Oxy's
internal IR for any program and read it like prose. And when a test fails, you'll be able to say
*which layer* broke — parser, type checker, IR gen, or codegen — instead of staring at a stack
trace and guessing. That's a real, concrete skill set, and it transfers: once you've seen how one
compiler is wired, every other compiler stops looking like a cathedral and starts looking like a
codebase.

What this book will *not* do is turn you into a programming-language theorist, and you should know
that going in. We don't cover formal grammars, parse tables, or the automata-theory machinery that
a university course would open with. We don't cover optimization passes, because Oxy mostly doesn't
have any — it leans on Cranelift for that. We don't cover type theory with Greek letters. This is
deliberate, not a gap we're embarrassed about: those subjects have excellent books already, and
none of them is required to understand how a working compiler is actually built. This book teaches
you to read and modify a *real* compiler — one with all the pragmatic shortcuts and hard-won scars
that real software has — rather than to design an idealized one in a vacuum. If, after this, you
want the theory, you'll be in a far better position to appreciate why any of it matters.

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
