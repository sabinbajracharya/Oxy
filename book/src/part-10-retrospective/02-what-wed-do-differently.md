# 500 Commits Later: What We'd Do Differently

Here is the honest version, the one you don't put in the README. If we started over tomorrow,
knowing everything the last five hundred commits taught us, several things would go differently —
and saying so plainly is more useful to you than pretending the path was straight. It wasn't. It
was windy, and it worked, and those two facts coexist.

The tree-walker we wouldn't take back. It proved the language's semantics fast and cheap, and that
was real value. But we'd jump to the register IR sooner than we did. The stack VM, on the other
hand, was a genuine detour — fifteen days of solid work building a thing we then deleted whole,
because Cranelift never wanted a stack in the first place. It wasn't *wrong*, exactly; it was the
textbook next step after a tree-walker, and we took it on reflex. But if native code is the goal
from the start, the stack VM is a station you can skip entirely, and we'd skip it.

The divergence guards are the clearest "build it on day one" lesson. The exhaustive match, the FFI
consistency test, the parity run — all of those arrived *after* the JIT and interpreter had already
had a chance to drift, which is to say after we'd already paid for their absence. The guards are not
overhead; they're the only reason "one feature, two backends" is a true statement instead of an
aspiration. They belong in the first commit of any dual-backend design, not bolted on once the
backends have started disagreeing. The same goes for IR traces: `OXY_VM_TRACE=1` showed up
mid-stabilization, and it would have turned hours of the 129-failure slog into minutes if it had
existed from the day the IR did. Every compiler stage should be dumpable from the moment it exists.
The cost is an afternoon; the payoff is every debugging session afterward.

Two more, briefly. The investment that paid off most was the shared FFI runtime — routing all real
semantics through `oxy_*` functions was the decision that made the whole dual-backend architecture
possible, and we'd make it again without hesitation. And the thing we deferred too long was
documentation: the per-folder READMEs and the architecture docs came late, after the structure had
already settled, which meant that for most of the project the only documentation was the code
itself. That's fine while it all lives in one person's head; it's a tax on everyone who comes after,
including your own future self.

None of this is regret. Every one of those "mistakes" was a real step that got us to a working,
tested, native-compiling language with a browser playground — and, not incidentally, taught us
enough to write this book. The path being windy is *why* there's something worth saying here. We're
just marking the shortcuts on the map so the next person can walk it straighter.

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
