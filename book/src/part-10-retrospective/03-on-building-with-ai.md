# On Building with AI: The Real Story

<!-- OPUS_FILL
Write a 4-5 paragraph personal narrative about the experience of building Oxy with AI assistance.

This should be honest and grounded, not promotional. Hit these points:
- The AI wrote the implementation; the human directed the architecture and made the calls
- What that division felt like in practice (the human understood the "what" but not always the "how")
- The surprising thing: the hardest decisions were never about code, they were about language design
  (do we add references? do we keep the integer width zoo? do we retire the stack VM?)
- The AI as an amplifier: 500+ commits in roughly 3 months would have taken years solo
- The cost: when you don't write the code yourself, you don't always understand it
  (this book exists partly because of that gap — understanding comes from re-reading and explaining)
- The honest verdict: AI-assisted development is not magic. You still need to understand what you're building.
  The AI writes the code. You still have to understand the compiler.

Tone: honest, slightly rueful, but not negative. This is the most personal section of the book.
It should feel like the author is speaking directly to the reader.
-->

## What an AI-assisted project actually looks like

Here is what building Oxy with AI assistance looked like in practice:

- **The human** decided what to build, what features to add, when to retire a design, what the language identity was
- **The AI** wrote the implementation — the parser, the type checker, the IR gen, the JIT codegen, the interpreter
- **Together**: 548+ commits in roughly 3 months

This is not how most software development books are written. Usually the author wrote the code and is explaining what they built and why. Here, the architecture decisions were made by a human who understood the system at a high level, and the code was largely written by an AI under direction.

---

## The division of labor

The decisions that mattered were never implementation details. They were design decisions:

- Should Oxy have references and borrows? (Answer: no. Oxy is dynamic Rust — see Chapter 0.)
- Should we keep `i32 / u64 / f32`? (Answer: no. Three types: `int`, `byte`, `float`.)
- Should we build a stack VM? (Answer: yes, then no. Fifteen days, then removed.)
- Should the JIT and interpreter share a runtime? (Answer: yes. The shared FFI is the right investment.)
- When the test suite hit 129 failures, was the architecture wrong? (Answer: no. Three naming conventions that had drifted apart.)

These are not code questions. They are language design questions. An AI can implement
any answer you give it. It cannot decide which answer is right for your language.

---

## The knowledge gap

Here is the uncomfortable truth: when someone else writes 548 commits of implementation code,
you do not automatically understand what they wrote.

The README-per-folder initiative (described in Part 5) was added late — after the architecture
was stable — because the gap between "I directed this" and "I understand this" had grown wide.
This book exists for the same reason. Explaining a system forces you to understand it.

Reading code you didn't write is different from reading code you did write. When you write
something, you remember the decision. When you read someone else's code (or AI-generated code),
you have to reconstruct the decision from the artifact.

The antidote is what you are doing right now: reading the code, tracing the execution,
following the data structures from one layer to the next. Understanding does not transfer
automatically. It has to be built.

---

## What AI-assisted development is and isn't

It is:
- An amplifier. One person directing an AI produced a working JIT compiler in ~3 months.
  Solo, without AI, this would have taken years.
- A force multiplier for implementation. If you know what you want, the AI can build it fast.
- A good pair programmer for Rust. Rust's type system catches real bugs; AI-generated Rust
  gets corrected by the compiler and can iterate faster than a human typing alone.

It is not:
- A replacement for understanding. You still have to understand the system you are building.
  The AI writes the code. The human still has to read it, direct it, catch the bugs, and
  make the design decisions.
- Automatic. The human gives direction on every commit. The AI does not know what Oxy should
  be. It knows what you tell it Oxy should be.
- Magic. The 129-failure stabilization was not solved by asking the AI "fix it." It was solved
  by reading the IR traces, identifying the naming convention mismatch, and then directing the AI
  to fix each cluster.

---

## What this means for you

If you want to build your own language with AI assistance, here is the actual workflow:

1. **You** design the language identity. What does this language do that others don't?
   What does it deliberately omit?
2. **You** decide the architecture. Tree-walker first, then register IR, then JIT.
   (Or: skip the stack VM. Seriously.)
3. **You** direct the implementation, one feature at a time.
4. **You** read the code that comes back and verify it matches what you intended.
5. **You** make the calls when the implementation hits design forks.

The AI handles the Rust. You handle the language.

This book covers step 4 in depth — reading and understanding the implementation.
Steps 1-3 and 5 are yours. The AI cannot do those for you. But if you do them well,
the AI can build a JIT compiler in the time it used to take to build a tree-walker.

That is a real change. Not magic, but real.
