# Why We Built a Language

Somewhere along the way, somebody convinced you that programming languages are built by a
different species of programmer. People with PhDs in type theory. People who read papers with
Greek letters in the titles. People who use words like "denotational semantics" in casual
conversation and mean it. The compiler, in this telling, is a cathedral — and you are a tourist
who is allowed to walk through it but never, ever to pick up a tool.

This is nonsense, and we are going to spend a whole book proving it.

Here is the secret that the cathedral story hides: a compiler is a pipeline of small, boring
programs. The first one reads your source text and chops it into labeled pieces. The next one
arranges those pieces into a tree. The next one walks the tree and checks that you didn't add a
number to a string. The next one flattens the tree into a list of simple instructions. The last
one turns those instructions into the ones your CPU actually speaks. Each program in the chain
does one transformation — takes one representation, hands back another — and then gets out of the
way. None of them is magic. None of them requires a Greek letter. The hard part isn't any single
step; it's getting all the steps to agree with each other, and that's a *plumbing* problem, not a
*genius* problem.

We know this because we built one. It's called Oxy, and it's a real language — it compiles to
native machine code, it has a package manager and an editor plugin and a browser playground, and
it took over 500 commits and a number of debugging sessions that ran past midnight. We are not
going to pretend it was easy. We *are* going to show you that every single piece of it is
something you could have written, because none of those pieces is harder than the code you write
at your day job. It's just code you haven't seen before.

So if you've ever looked at `rustc` or `clang` or the thing that runs your Python and felt a
small, quiet certainty that *you* could never make one of those — good. Hold onto that feeling.
We're going to take it apart.

So we built one. This is that story.

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
