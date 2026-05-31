# 500 Commits Later: What We'd Do Differently

<!-- OPUS_FILL
Write a 4-5 paragraph honest retrospective.

This should be the most personal chapter in the book. Not hedged corporate lessons,
but genuine "if we started over tomorrow, here's what we'd do differently."

Points to hit:
- The tree-walker was not wasted time, but we'd probably jump to register IR sooner
- The stack VM was definitely an unnecessary detour (15 days, then removed)
- The divergence guards should have been built from day 1, not after the first parity failure
- The biggest investment that paid off most: the shared FFI runtime
- The thing that would have helped most: IR traces earlier, as a debugging tool
- The cost of 500+ commits that nobody else can read: the README-per-folder initiative was late

Tone: honest, a bit rueful, but not regretful. Every "mistake" was a real step that got us here.
The path was windy but it worked.
-->

## The real lessons (not the sanitized ones)

### 1. Jump to register IR earlier

The tree-walking interpreter was not wasted time — it proved the semantics, gave us a
working language fast, and its test suite is still running. But if the goal was always
native compilation, we would have been better served by:

1. Tree-walker for 2-3 weeks (prove the features work)
2. Design the register IR
3. IR gen directly

The stack VM (15 days, then removed) was a genuine detour. We added it because "that's
what you do" after a tree-walker — compile to bytecode. But Cranelift doesn't want
bytecode. It wants register IR. The stack VM was not wrong, just unnecessary — we were
solving a problem that the register IR approach did not have.

**Lesson:** when your target is a register-based code generator, skip the stack VM.
Go from tree-walker to register IR directly.

### 2. Divergence guards from day one

The FFI consistency test and the exhaustive match in `interp.rs` were added after the
JIT stabilization work. Before they existed, there was a period where it was possible to
add an `oxy_*` function to the JIT but forget to add it to the interpreter.

If we were starting over: the divergence guards would be in the first commit of the
dual-backend design. "Add the feature once, get it on both backends" only works if there
is a mechanism ensuring you actually did it on both backends.

The guards are not optional. They are not "nice to have." They are the reason the
"one feature, two backends" promise holds.

### 3. IR traces from the beginning

`OXY_VM_TRACE=1` was added mid-way through the JIT stabilization. Before that, debugging
IR gen bugs required reading the source code and mentally simulating execution.

The IR pretty-printer and snapshot tests should have been in the first JIT commit.
They would have caught the `ConstBool` tagging bug (Cluster 1) in minutes instead of
hours. The IR is the contract between ir_gen and codegen; you need to be able to read it.

**Lesson:** every compiler stage should have a `--dump-<stage>` flag from day one.
The cost to implement is two hours. The payoff in debugging time is enormous.

### 4. The shared FFI runtime was the right investment

The decision to route all complex operations through `oxy_*` FFI functions — rather than
having the JIT and interpreter each implement operations directly — was the most impactful
architectural decision. It made the dual-backend design feasible.

The cost: every FFI function is a function call overhead, even for operations that could
be inlined (struct access, simple method dispatch). A more optimized system might inline
more operations into CLIF directly.

The benefit: when the semantics of `Vec::push` are correct, they are correct on both backends.
When a bug is fixed in `oxy_vec_push`, both backends get the fix. You cannot accidentally
have a JIT-specific bug that the interpreter doesn't have for the same operation.

We would not change this decision. The maintainability benefit outweighs the performance cost.

### 5. Documentation should have been concurrent, not deferred

The README-per-folder initiative (the `docs/` folder and the per-directory `README.md`
files) was added late — after the architecture was stable, as a separate effort. This
meant that for most of the project's history, the only documentation was the code itself.

For a solo or small team working continuously, this is fine — you always know what's in
your head. For a new contributor (or your future self six months later), it is a tax on
every interaction with the codebase.

**Lesson:** write the folder README when you create the folder, not after the project
is done. The cost is 30 minutes. The payoff compounds over every future navigation of
that folder.

## What to build if you start over

If you were building Oxy from scratch today, armed with everything this book covers:

1. **Lexer + parser + AST** (2-4 weeks) — same design, same Pratt parsing
2. **Type checker** (2-3 weeks) — two-pass, TypeInfo enum, `accepts()` method
3. **Tree-walker** (1-2 weeks) — prove the semantics; use `#[test]` from the start
4. **Register IR + IR gen** (2-3 weeks) — skip stack VM entirely
5. **FFI design** — `oxy_*` functions from day one, `ffi_symbols()` + `ffi_decls()` immediately
6. **Cranelift JIT** (3-4 weeks) — the two-map register strategy; IR traces immediately
7. **IR interpreter** (1-2 weeks) — exhaustive match, no wildcard; divergence guards on day 1
8. **LSP** (1-2 weeks) — symbols.rs as single source of truth from the start

Total: ~4-5 months for a working JIT compiled language with wasm support.

Oxy took considerably longer — but the path included the tree-walker, the stack VM,
the 129-failure stabilization, and building the README infrastructure. None of those were
wasted. They produced a working, tested, documented system and the accumulated experience
that let this book exist.
