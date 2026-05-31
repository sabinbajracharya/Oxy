# Your Turn: What Can You Build Now?

This is the last page, so let's not be grand about it. You started this book believing, somewhere
in the back of your mind, that compilers were built by a different kind of programmer. By now that
belief should be gone, because you've watched the whole thing taken apart and seen that there was
never a wizard behind the curtain — just five layers of transformation, each one a program that
reads one representation and produces another. Text to tokens. Tokens to tree. Tree to IR. IR to
machine code. That's the mythical creature, fully dissected, and it turns out to be made of the same
parts as everything else you've ever written.

So here's what you can actually do now. You can open `crates/oxy-core/src/` and *read* it — not
squint and hope, but navigate it, knowing which layer owns what and where to look when something
breaks. That was the first goal of this book, and you have it. You can add a feature: find the right
layer, write the `.ox` test first the way CLAUDE.md insists, watch it fail, fix the layer, watch it
pass. And you can build your own language — you have the blueprint, the rough timeline, and, more
importantly, the *judgment*, because you've seen which decisions are load-bearing and which are
detours you can skip.

I'll be honest about why I hope you do. Building a compiler is one of the most satisfying things you
can do as a programmer, because it refuses to let you specialize. It demands data structures and
algorithms and a little type theory and some systems programming and, hardest of all, taste — the
willingness to decide what your language *is* and what it deliberately refuses to be. It asks for
everything you've got. And then, at the end, the thing *runs*. You type a program into something you
built and a real CPU executes it. There are not many feelings in this craft quite like that one.

The compiler was never a mythical creature. It was always just five honest programs in a trench
coat. Now you know how to build them.

Now go build something.

## What you can do now

After reading this book, you can:

1. **Navigate the Oxy codebase without AI assistance.** You know what each layer does,
   where the boundaries are, and what to look for when something goes wrong.
   The README files will help; so will `OXY_VM_TRACE=1`.

2. **Add a feature.** Pick something small:
   - A new built-in method on `String` or `Vec`
   - A new operator
   - A new statement form
   
   The process is always the same: write the `.ox` test first, run it, watch it fail,
   find the right layer, fix it, run it again. The TDD loop in the CLAUDE.md is not
   just a convention — it is the most reliable way to add a feature without breaking six
   other things.

3. **Debug something real.** When a test fails: read the IR trace, identify which layer
   produced wrong output, fix that layer. The layers are independent enough that a bug
   in IR gen cannot be caused by the type checker — the IR trace will tell you exactly
   where the wrong value appeared.

---

## If you want to build your own language

You have the blueprint:

| What | How long | Notes |
|------|----------|-------|
| Lexer | 1 week | Finite state machine. Start with identifiers, numbers, keywords, operators. |
| Parser + AST | 2-3 weeks | Pratt parsing for expressions. Recursive descent for statements. |
| Type checker | 2-3 weeks | Two-pass. TypeInfo enum. `accepts()` method. Catch errors before execution. |
| Tree-walker | 1-2 weeks | Prove the semantics. Get a passing test suite. |
| Register IR + IR gen | 2-3 weeks | Skip the stack VM. Go directly to named registers and basic blocks. |
| FFI runtime | 1 week | All complex operations as `oxy_*`-style Rust functions. Shared between all backends. |
| Cranelift JIT | 3-4 weeks | Two-map strategy. IR traces from day 1. Divergence guards from day 1. |
| IR interpreter (wasm) | 1-2 weeks | Exhaustive match, no wildcard. Consistency tests for the FFI surface. |

**Total: ~4-5 months.** Probably more. Maybe less if you skip the parts Oxy didn't need to skip.

The most important design decision you will make is not about the parser or the JIT.
It is about the language identity: what does this language do that others don't, and what
does it deliberately leave out?

Oxy's answer was: dynamic Rust — Rust syntax without the borrow checker. Every other
decision followed from that.

---

## What the exercises were for

Each chapter had exercises. They were not optional busywork. They were the places where
the book could not do the understanding for you.

Reading the `ConstBool` bug in Part 7 explains what happened. Tracing through `ir_gen` for
a boolean literal yourself is what makes you able to catch the next one.

Understanding is not transferred by reading. It is built by doing. If you skipped the exercises,
go back and do the ones that felt hard. The hard ones are the ones that actually build the intuition.

---

## One last thing

A compiler is not a mythical creature. It is five layers of transformation:

```
text → tokens → AST → IR → native code
```

Each layer is a program that reads one representation and produces another. Each layer can
be read, debugged, and understood independently. The whole pipeline is ~20 source files
(ignoring the FFI and stdlib) that you have now read, traced, and — if you did the exercises —
partially written.

That's the whole thing.

Now go build something.

---

*The Oxy source code is at [`crates/oxy-core/src/`](../../crates/oxy-core/src/). The test suite is at [`examples/features/`](../../examples/features/). The TDD process is in [`CLAUDE.md`](../../CLAUDE.md). Everything you need is already there.*
