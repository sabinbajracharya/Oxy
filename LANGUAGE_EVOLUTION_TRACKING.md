# Oxy Language Evolution — Task Tracking

> **Goal:** Evolve from "dynamic Rust" to a **fast, approachable general-purpose language that runs everywhere** — native, WASM, embedded.
> **Invariant:** IR + JIT codegen + wasm interpreter untouched. Changes are parser/type-checker/stdlib only.
> **Rules:** No backward compat. Architecture-first. Green after every commit. One commit per logical change.

---

## Phase 1: Surface Cleanup — Remove Rust Baggage

### 1.1 Retire vestigial keywords from lexer
- [x] Remove `ref` keyword and token variant
- [x] Remove `dyn` keyword and token variant
- [x] Remove `move` keyword and token variant
- [x] Remove `where` keyword and token variant
- [x] Remove `static` keyword and token variant
- [x] Update KEYWORDS constant
- [x] Remove `is_static` from `Item::Const` AST node
- [x] Clean up parser: remove Move from expr.rs, Static from item dispatch, Where clause parsing
- [x] Simplify `parse_const_def` to not take `is_static`
- [x] Update parser tests, symbol consistency tests, VM tests
- [x] Update all `.ox` example/test files
- [x] **Committed:** `f5b6518` — refactor: retire vestigial keywords (ref, dyn, move, where, static)

### 1.2 Simplify visibility system
- [x] Remove `Visibility::PubCrate` and `Visibility::PubSuper` from AST
- [x] Simplify visibility parsing: `pub` token → `Visibility::Pub`, else `Private`
- [x] Simplify `is_visible_from()` — remove PubCrate/PubSuper arms
- [x] Remove dead `parent_module` helper
- [x] Update `.ox` tests: `pub(crate)`/`pub(super)` → `pub`
- [x] `super`/`crate` keywords stay for module path navigation (`super::`, `crate::`)
- [x] **Committed:** `ac68659` — refactor: simplify visibility to pub/private only

### 1.3 Remove `mut` from parameter position
- [x] Remove `is_mut` field from `Param` struct in AST
- [x] Remove `mut` token consumption from `parse_param_list`
- [x] Type checker: always pass `true` to `define_mut` for params
- [x] Remove `immutable self` error check — self always mutable
- [x] Update error message for `&` rejection
- [x] Update all `.ox` test files and VM test files
- [x] **Committed:** `e1f8440` — refactor: remove mut from function parameters

### 1.4 Update remaining test corpus
- [x] All .ox test files updated across commits
- [x] All VM test files updated across commits
- [x] All 989+ tests pass with zero failures

---

## Phase 2: Type Inference Upgrade ✅

### 2.1 Architecture: bidirectional type checking foundation
- [x] Add `expected: Option<&TypeInfo>` parameter to `infer_expr()` signature
- [x] Thread it through all recursive calls in check_expr sub-modules
- [x] No second pass, no constraint solver — single traversal, bidirectional flow
- [x] **Committed:** `2168756` — feat(tc): add expected-type plumbing for bidirectional type checking

### 2.2 Closure parameter inference
- [x] When inferring a closure literal inside a function call, pass expected param types from fn signature
- [x] Handle multi-param closures
- [x] Handle generic closures (expected type has generic params — via Unknown fallback)
- [x] **Committed:** `dd69aeb` — feat(tc): infer closure parameter types from expected function signature

### 2.3 Literal auto-cast to expected type
- [x] `let x: float = 42` — literal int auto-casts to float when expected
- [x] `let v: Vec<String> = []` — empty array typed from expected Vec element type
- [x] `let b: byte = 0` — literal int auto-casts to byte within range
- [x] **Committed:** `9a1a0ab` — feat(tc): auto-cast literals to expected type (bidirectional)

### 2.4 Generic return type inference at call sites
- [x] When calling `fn first<T>(v: Vec<T>) -> Option<T>`, infer T from argument type
- [x] Already worked via substitute_generics + check_builtin_method_args — verified and tested
- [x] **Committed:** `4add1d7` — feat(tc): strengthen generic return type inference + closure/let integration

### 2.5 Nested/local function inference
- [x] Closures (Oxy's local functions) infer params from let binding type + call context
- [x] Return types already inferred from body
- [x] **Committed:** `4add1d7` — feat(tc): strengthen generic return type inference + closure/let integration

---

## Phase 3: Expressiveness

### 3.1 Pipeline operator `|>`
- [x] Add `PipeArrow` token to lexer
- [x] Add `Precedence::Pipeline` level (between Assignment and Range)
- [x] Parse `|>` as binary infix, desugar to `Expr::Call` or `Expr::MethodCall` in parser
- [x] Handle edge cases: `?`, `.await`, method chains, multi-line
- [x] Add pipeline test file: `examples/features/expressions/pipeline.ox` + 5 Rust TC tests
- [x] **Committed:** `feat: add pipeline operator |>`

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
