# Why We Built a Language

<!-- OPUS_FILL
Write a 4-6 paragraph opening for the entire book. This is the first thing the reader sees.

Goals:
- Shatter the myth that programming languages are built by wizards in ivory towers
- Make the reader feel "I could do this too" — not intimidated
- Set up the central thesis: a compiler is just a pipeline of simple transformations
- End with a hook into the Oxy story: "So we built one. This is that story."

Tone: conversational, slightly irreverent, genuinely excited. Like a friend who just finished
building something they can't stop talking about. First-person plural ("we built") throughout.

The reader is someone who has written code but never looked at a compiler before. They probably
think compilers are arcane magic. By the end of this paragraph block, they should be skeptical
of that belief.
-->

## What a compiler actually is

Strip away the mystique and a compiler is this:

1. Read text
2. Figure out what the text means
3. Transform it into something a machine can execute

That's it. Each step is a program. Each program has inputs and outputs. They chain together.
The output of step 1 is the input to step 2. The output of step 2 is the input to step 3.

There is no magic. There is no secret knowledge. There is only: **take this thing, produce that thing**.

```
"fn main() { println(42); }"
          │
          ▼ step 1: lexer
[Fn][Ident("main")][LParen][RParen][LBrace][Ident("println")][LParen][Int(42)][RParen][Semicolon][RBrace]
          │
          ▼ step 2: parser
FunctionDef { name: "main", body: [Call { fn: "println", args: [42] }] }
          │
          ▼ step 3: ...eventually
; machine code that prints 42
```

The hard part is not any single step. The hard part is getting all the steps to agree with each other.

## The thing most books don't tell you

Most books about compilers teach you the theory first. Grammars. Automata. Parse tables. Formal
semantics. By chapter 4 you're deep in mathematical notation and you've forgotten why you started.

This book does it the other way. We start with a working language — **Oxy**, which we built over
500+ commits — and work backwards. Here's the feature. Here's the code that implements it. Here's
why we wrote it this way. Here's what broke the first time.

Oxy is real. It compiles to native machine code via Cranelift. It has a package manager, an LSP,
a VS Code extension, and a browser playground. It is not a toy.

It was also, at various points, completely broken in ways that took days to debug.

Both of those facts are in this book.

## What you need to know to start

- You can read code in some language. Any language.
- You are comfortable with the idea of a function taking inputs and returning outputs.
- You are curious.

You do not need to know Rust. We teach the Rust we need, when we need it, and only what we need.
Rust syntax will appear at the moment it becomes relevant — not in a 50-page "Rust basics" chapter
you have to wade through before getting to the interesting parts.

You also do not need to have studied computer science formally. We will explain every concept
from first principles.

## The structure of what follows

Each part of this book covers one layer of the Oxy compiler:

| Part | Layer | What it does |
|------|-------|-------------|
| 1 | Lexer | Turns text into a stream of labeled pieces (tokens) |
| 2 | Parser | Turns tokens into a tree that represents the code's structure |
| 3 | Tree-Walker | The first way we ran code — simple and slow |
| 4 | Type Checker | Catches mistakes before anything runs |
| 5 | Stack VM | The second way we ran code — a classic approach we later retired |
| 6 | Register IR | The language we invented between Oxy and machine code |
| 7 | Cranelift JIT | How we turn that language into real native machine code |
| 8 | WASM Interpreter | How the same IR runs in a browser without compiling |
| 9 | Ecosystem | LSP, package manager, REPL — the shell around the language |
| 10 | Retrospective | What we learned. What we'd change. What's next. |

After Part 4 (Type Checker), you will understand how Oxy validates code before running it.
After Part 7 (JIT), you will understand how Oxy turns source code into native machine code.
After Part 10, you will be able to open `crates/oxy-core/src/` and navigate it without help.

That is the goal. Let's go.
