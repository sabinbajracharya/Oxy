# Oxy Language Evolution — Task Tracking

> **Goal:** Evolve from "dynamic Rust" to a **fast, approachable general-purpose language that runs everywhere** — native, WASM, embedded.
> **Invariant:** IR + JIT codegen + wasm interpreter untouched. Changes are parser/type-checker/stdlib only.
> **Rules:** No backward compat. Architecture-first. Green after every commit. One commit per logical change.

---

## Phase 1: Surface Cleanup — Remove Rust Baggage

### 1.1 Retire vestigial keywords from lexer
- [ ] Remove `ref` keyword and token variant
- [ ] Remove `dyn` keyword and token variant
- [ ] Remove `move` keyword and token variant
- [ ] Remove `where` keyword and token variant
- [ ] Remove `static` keyword and token variant
- [ ] Update KEYWORDS constant
- [ ] **Commit:** `refactor: retire vestigial keywords from lexer (ref, dyn, move, where, static)`

### 1.2 Clean up parser of vestigial keyword handling
- [ ] Remove `ref` from pattern parsing
- [ ] Remove `dyn` from type parsing
- [ ] Remove `move` from closure parsing
- [ ] Remove `where` from fn parsing (bounds stay inline on generics as `T: Bound`)
- [ ] Remove `static` from item parsing (keep `const` only)
- [ ] **Commit:** `refactor: remove vestigial keyword handling from parser`

### 1.3 Simplify visibility system
- [ ] Remove `Visibility::PubCrate` from AST
- [ ] Remove `Visibility::PubSuper` from AST
- [ ] Simplify visibility parsing (just `pub` or none)
- [ ] Simplify visibility checking in type checker
- [ ] Simplify path resolution (no crate/super paths)
- [ ] Remove `super` and `crate` keywords? — evaluate: needed for module navigation?
- [ ] **Commit:** `refactor: simplify visibility to pub/private only`

### 1.4 Remove `mut` from parameter position
- [ ] Remove `mut` parsing from function parameter lists
- [ ] Clean up type checker param handling
- [ ] Update all `.ox` test files
- [ ] **Commit:** `refactor: remove mut from function parameters`

### 1.5 Update test corpus for Phase 1
- [ ] Strip retired keywords from all 117+ `.ox` test files
- [ ] Strip `pub(crate)` / `pub(super)` → `pub` or private
- [ ] Strip `mut` from parameter lists
- [ ] All tests pass: `cargo test -p oxy-core`
- [ ] **Commit:** `test: update test corpus for Phase 1 syntax changes`

---

## Phase 2: Type Inference Upgrade

### 2.1 Architecture: bidirectional type checking foundation
- [ ] Add `expected: Option<&TypeInfo>` parameter to `infer_expr()` signature
- [ ] Thread it through all recursive calls in check_expr sub-modules
- [ ] No second pass, no constraint solver — single traversal, bidirectional flow
- [ ] **Commit:** `feat(tc): add expected-type threading to infer_expr`

### 2.2 Closure parameter inference
- [ ] When inferring a closure literal inside a function call, pass expected param types from fn signature
- [ ] Handle multi-param closures
- [ ] Handle generic closures (expected type has generic params)
- [ ] **Commit:** `feat(tc): infer closure parameter types from expected signature`

### 2.3 Literal auto-cast to expected type
- [ ] `let x: float = 42` — literal int auto-casts to float when expected
- [ ] `let v: Vec<String> = []` — empty array typed from expected Vec element type
- [ ] `let b: byte = 0` — literal int auto-casts to byte within range
- [ ] **Commit:** `feat(tc): auto-cast literals to expected type`

### 2.4 Generic return type inference at call sites
- [ ] When calling `fn first<T>(v: Vec<T>) -> Option<T>`, infer T from argument type
- [ ] Already partially works via substitute_generics — verify and extend
- [ ] **Commit:** `feat(tc): strengthen generic return type inference`

### 2.5 Nested/local function inference
- [ ] Allow nested functions to omit parameter types when inferrable from calls
- [ ] Allow nested functions to omit return type when inferrable from body
- [ ] **Commit:** `feat(tc): relax annotation requirements for nested functions`

---

## Phase 3: Expressiveness

### 3.1 Pipeline operator `|>`
- [ ] Add `PipeArrow` token to lexer
- [ ] Add `Precedence::Pipeline` level (between Assignment and Range)
- [ ] Parse `|>` as binary infix, desugar to `Expr::Call` or `Expr::MethodCall` in parser
- [ ] Handle edge cases: `?`, `.await`, method chains, multi-line
- [ ] Add pipeline test file: `examples/features/expressions/pipeline.ox`
- [ ] **Commit:** `feat: add pipeline operator |>`

### 3.2 Single-line function syntax
- [ ] Parse `fn name(params) -> T = expr` — desugar to block with tail expr
- [ ] Handle return type omission: `fn add(x, y) = x + y`
- [ ] Add test file
- [ ] **Commit:** `feat: add single-line function syntax`

### 3.3 Pipeline-friendly stdlib
- [ ] Add free functions: `map(data, f)`, `filter(data, f)`, `fold(data, init, f)`
- [ ] Add: `sort(data)`, `sort_by(data, f)`, `collect(data)`
- [ ] Add: `find(data, pred)`, `any(data, pred)`, `all(data, pred)`
- [ ] These share implementation with existing Iterator methods
- [ ] **Commit:** `feat(stdlib): add pipeline-friendly free functions`

### 3.4 F-string ergonomics
- [ ] Add `print()` and `println()` as built-in functions (not macros)
- [ ] Accept f-string-style arguments
- [ ] Keep `println!()` macro for backward compat during transition, then remove
- [ ] **Commit:** `feat: add print/println built-in functions`

---

## Phase 4: Identity — General-Purpose, Runs Everywhere

### 4.1 Free-standing method syntax
- [ ] Parse `fn Type.method(self, params) -> T { ... }` 
- [ ] Desugar to `impl Type { fn method(self, params) -> T { ... } }`
- [ ] Keep `impl` blocks valid — free-standing is sugar, not replacement
- [ ] **Commit:** `feat: add free-standing method syntax (fn Type.method(...))`

### 4.2 Remove `return` keyword
- [ ] Remove `return` from lexer and parser
- [ ] Last expression in block is always the value
- [ ] Early exit from loops via `break value` (already supported)
- [ ] Update all `.ox` test files
- [ ] **Commit:** `refactor: remove return keyword — use tail expressions and break value`

### 4.3 Remove `println!` / `print!` macros
- [ ] Remove 9 built-in macros entirely? Or keep `vec!`, `format!`, `panic!`, `todo!`?
- [ ] Decision: keep `vec!`, `format!`, `panic!`, `todo!`, `dbg!` — remove `println!`, `print!`, `eprintln!`
- [ ] Built-in `print()`, `println()`, `eprintln()` functions take their place
- [ ] **Commit:** `refactor: replace println!/print!/eprintln! macros with functions`

### 4.4 Update CLAUDE.md identity
- [ ] Replace "Dynamic Rust" section with positive "Typed Scripting" identity
- [ ] Update language identity description
- [ ] Update syntax mapping table
- [ ] **Commit:** `docs: update CLAUDE.md language identity`

### 4.5 Update README and docs
- [ ] New tagline: "Oxy: typed scripting for data, CLIs, and servers"
- [ ] New hello world example
- [ ] Update all folder README.md files
- [ ] **Commit:** `docs: update README and folder docs for new identity`

---

## Phase 5: Ecosystem

### 5.1 VS Code extension
- [ ] Remove highlighting for retired keywords
- [ ] Add `|>` token highlighting
- [ ] Update language configuration
- [ ] **Commit:** `feat(vscode): update syntax highlighting for language evolution`

### 5.2 LSP updates
- [ ] Verify LSP works with all parser changes
- [ ] Update keyword completions
- [ ] Update hover docs
- [ ] **Commit:** `feat(lsp): update completions and hover for evolved syntax`

### 5.3 Tug (package manager) updates
- [ ] Verify tug compiles and tests pass with all changes
- [ ] Update any `.ox` templates (scaffolding)
- [ ] **Commit:** `feat(tug): update templates for evolved syntax`
