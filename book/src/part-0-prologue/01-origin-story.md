# The Origin Story: Ferrite → Oxide → Oxy

<!-- OPUS_FILL
Write a 3-4 paragraph narrative intro for this chapter.

The story: a language that started as "Ferrite" (a type of iron, hard), became "Oxide" (a Rust
play-on-words — Rust oxidizes iron), then became "Oxy" (shorter, cleaner). The name changed but
the mission didn't: build a language that feels like Rust but without the borrow checker — all
the ergonomics, none of the ownership ceremony.

Tone: origin story energy. Like someone explaining how a band got its name. A little self-deprecating
about the name changes, but proud of the mission that stayed constant throughout.

End by setting up the timeline this chapter will walk through.
-->

## The timeline

Here is the real commit history of the project, condensed into its major moments:

| Date | Event | Commit |
|------|-------|--------|
| 2026-03-03 | Project born as "Ferrite" — lexer, parser, and tree-walking interpreter | `eba8b59` |
| 2026-03-03 | Phases 2–11: collections, structs, traits, closures, modules, async | `435bae1`–`af6fd28` |
| 2026-03-07 | Renamed: Ferrite → Oxide. File extension `.fe` → `.ox` | `033da68` |
| 2026-05-13 | Bytecode compiler + stack-based VM added | `7d26f68` |
| 2026-05-14 | Renamed: Oxide → Oxy | `bb7b4ac` |
| 2026-05-27 | Cranelift JIT skeleton added. Register IR gen begins | `~` |
| 2026-05-28 | Stack VM removed. Register IR is the only backend | `~` |
| 2026-05-30 | Full JIT merge + wasm IR interpreter | `cfb4e9a` |
| 2026-05-31 | Refactoring complete. 500+ commits. All tests green. | `e5c366e` |

Notice something: Phases 1–11 (lexer through async) all landed on the same day. That is not
a typo. The early tree-walking interpreter was scaffolded fast — the features were simple because
evaluation was just "walk the AST and do stuff". The complexity came later, when we had to actually
*compile* those features.

## The three eras

### Era 1: Ferrite / Tree-Walking (March 2026)

The first Oxy was called Ferrite and it ran like every beginner language does: walk the AST node
by node, evaluate each node recursively, return a value.

This works. This works well enough to build a usable language. Ferrite had closures, generics,
async/await, HTTP, JSON, and a full module system — all implemented as AST evaluation.

The price: speed. An AST-walking interpreter visits every node fresh on every execution. There
is no compilation, no optimization, no native code. Fast enough for scripts. Not fast enough
for anything compute-intensive.

### Era 2: Oxide / Stack-Based VM (May 2026, week 1)

The textbook upgrade: compile the AST to bytecode, execute bytecode on a stack machine. This is
how CPython works. This is how the JVM started. This is how almost every "learn compiler design"
course teaches execution.

The rename to Oxide happened here — fitting, because Rust oxidizes iron, and the language was
becoming something more refined.

The stack VM worked. It was faster than tree-walking. But it had a ceiling: the shape of a stack
machine makes certain optimizations hard. More importantly, we had a bigger goal — native machine
code. And stack VMs are not what Cranelift (our target code generator) expects.

### Era 3: Oxy / Register IR + JIT (May 2026, weeks 2-4)

The final rename to Oxy coincided with the real architectural shift: replace the stack VM with a
**register-based intermediate representation** (IR). Each instruction operates on named registers
rather than an implicit stack. The IR is organized into **basic blocks** forming a **control flow
graph** (CFG).

This IR is what Cranelift wants. From IR to native code is Cranelift's job. From AST to IR is ours.

Then came the twist: the IR also runs on a second backend — a direct interpreter for wasm32.
One IR, two execution engines. The same program runs natively on your laptop and in your browser,
with zero semantic divergence by construction.

The total cost of this third era: about 400 commits, multiple debugging marathons, and one
particularly memorable session tracking down why `bool` values were being mis-tagged and
causing wrong comparison results. That session became the war story in Part 7.

## The name evolution

**Ferrite** — iron that's been magnetized. Hard and structured. A good name for a first attempt.

**Oxide** — iron that's rusted. A deliberate Rust pun. The language was becoming more refined,
so naturally it had to be named after chemical degradation.

**Oxy** — short for oxygen, the thing that causes oxidation. Also just shorter to type. The
`.ox` file extension already existed (from the Oxide rename), so "Oxy" fit cleanly.

The mission never changed: **Rust-like syntax, without borrow checking**. Variables can be
mutable or immutable. Types are checked. Generics work. Closures work. No `&T`, no `'a`, no
ownership ceremony. Call it "dynamic Rust" — all the ergonomics, none of the compile-time
ownership lectures.

## What stayed constant

Through all three eras, the pipeline front-end stayed the same:

```
source text → Lexer → Parser → Type Checker → [backend]
```

The lexer we wrote in Era 1 still runs in Era 3. The parser we wrote in Era 1 still runs in
Era 3. The type checker accumulated features but its structure was set early.

This is an important lesson: **the front end of a compiler is stable; the back end is where
you iterate**. Build a solid lexer and parser first. You will not throw them away. You will
throw away backends.

We threw away two.
