# On Building with AI: The Real Story

Let me talk to you directly for a moment, because this is the part of the book that's least like a
tutorial and most like a confession. Oxy was built with an AI writing most of the implementation.
Not as a gimmick, not as a demo — as the actual working method, across five-hundred-plus commits.
The AI wrote the parser, the type checker, the IR gen, the JIT codegen, the interpreter. The human
decided what to build, when to throw something away, and what the language *was*. That division is
the honest center of this whole project, and it's worth being precise about what it felt like.

What it felt like, most of the time, was understanding the *what* without always understanding the
*how*. I knew Oxy needed a register IR and why; I did not, in the moment, always know exactly how
the spill-slot bookkeeping in codegen worked. I knew the dual-backend design required divergence
guards; the specific shape of the exhaustive-match enforcement was something the AI built and I
approved. There's a peculiar vertigo to that — directing a system at a high level while the
implementation details live somewhere you didn't personally put them. It's productive and slightly
unnerving at once.

The genuinely surprising thing is what turned out to be hard. It was never the code. The AI could
implement essentially any decision once it was made. The hard parts were the *decisions* — and
every single one of them was about language design, not implementation. Should Oxy have references
and borrows? (No — and holding that line was a real act of will, because it would have been easy to
drift toward Rust.) Should we keep the integer width zoo? (No — three numeric types, on purpose.)
Should we retire the stack VM after fifteen days of work? (Yes.) None of those is a question an AI
can answer for you, because none of them has a *correct* answer — they have answers that are right
*for the language you're trying to build*, and only a human holding the vision can make that call.

So the AI was an amplifier, and a staggering one. Five hundred commits and a native-compiling
language with a browser playground in roughly three months is not a solo-in-evenings timeline; it's
a years timeline, compressed. But amplifiers have a cost, and here's the rueful part: when you don't
write the code yourself, you don't automatically understand it. The decision lives in your memory;
the *artifact* does not. The gap between "I directed this" and "I could explain this to a stranger"
turned out to be wide, and closing it took deliberate work — which is, frankly, a large part of why
this book exists. Explaining a system to someone else is the most reliable way to discover whether
you actually understand it. Writing these chapters was where a lot of my own understanding finally
caught up to the code.

Which leads to the only verdict I'm confident in: AI-assisted development is not magic, and anyone
selling it as magic is selling something. The AI writes the code, fast and well. It does not relieve
you of the obligation to understand the system you're building, make the calls only you can make, or
read what comes back and verify it means what you intended. The 129-failure stabilization was not
solved by asking the AI to "fix it" — it was solved by reading IR traces and finding convention
mismatches, the same way it would have been solved by hand. The tools changed. The thinking didn't.
You still have to understand the compiler.

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
