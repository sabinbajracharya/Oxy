# Oxy's IR Gen: AST → IrFunction

<!-- OPUS_FILL
1-paragraph intro. "The IR gen is where the AST becomes executable. Let's trace through it."
Mention the split across files (the large ir_gen/mod.rs was split into per-domain files
in a recent refactor). Reference the files to read.
-->

**Files** (after the May 2026 split refactor):
- `crates/oxy-core/src/vm/jit/ir_gen/mod.rs` — `IrGen` struct, entry points
- `crates/oxy-core/src/vm/jit/ir_gen/*.rs` — per-domain gen methods

---

## The `IrGen` struct

```rust
pub(crate) struct IrGen {
    functions: Vec<IrFunction>,   // all generated functions (output)
    current: IrFunction,          // function being generated right now
    current_block: BlockId,       // block being built
    next_reg: Reg,                // register counter
    next_block: BlockId,          // block ID counter
    locals: HashMap<String, usize>, // variable name → local slot
    local_count: usize,
    break_target: Option<BlockId>,   // for break
    continue_target: Option<BlockId>, // for continue
    variant_to_enum: HashMap<String, String>, // "Some" → "Option"
    use_aliases: HashMap<String, String>,     // short → qualified
    // ...15+ more fields
}
```

`IrGen` is stateful. It builds one function at a time (`current`), emitting ops into
`current_block`. When a function is complete, it is pushed to `functions` and a new
`IrFunction` starts.

## Entry point: `gen_program`

```rust
pub(crate) fn gen_program(mut self, program: &Program) -> Result<Vec<IrFunction>, PipelineError> {
    // Prescan: collect variant names, const values, module visibility
    self.prescan_items(&program.items, "");

    // Generate each top-level item
    for item in &program.items {
        self.gen_item(item)?;
    }

    Ok(self.functions)
}
```

`prescan_items` does a first pass: records enum variant names (so `Some(x)` can be
recognized as a variant constructor, not an unknown function call), collects `const`
expressions, seeds `use_aliases`. This is the IR gen's equivalent of the type checker's
`collect_defs`.

## `gen_fn`: compiling a function

```rust
fn gen_fn(&mut self, fn_def: &FnDef) -> Result<(), PipelineError> {
    // Start a new IrFunction
    self.begin_function(qualified_name, fn_def.params.clone(), return_type);

    // Register parameters as locals
    for param in &fn_def.params {
        let slot = self.alloc_local(&param.name);
        // Params are already in locals[0..param_count] from the call site
    }

    // Generate the function body
    let body_reg = self.gen_block(&fn_def.body)?;

    // Terminate the current block with Return
    self.terminate(Terminator::Return(body_reg));

    // Push the completed function
    self.finish_function();
    Ok(())
}
```

## `gen_expr`: expressions → register

The heart of IR gen is `gen_expr`, which takes an `Expr` and returns a `Reg`:

```rust
fn gen_expr(&mut self, expr: &Expr) -> Result<Reg, PipelineError> {
    match expr {
        Expr::IntLiteral(n, _, _) => {
            let r = self.alloc_reg();
            self.emit(IrOp::ConstInt(r, *n));
            Ok(r)
        }

        Expr::Ident(name, _) => {
            let r = self.alloc_reg();
            let slot = self.locals[name];
            self.emit(IrOp::LoadLocal(r, slot));
            Ok(r)
        }

        Expr::BinaryOp { left, op, right, .. } => {
            let lr = self.gen_expr(left)?;
            let rr = self.gen_expr(right)?;
            let result = self.alloc_reg();
            match op {
                BinOp::Add => self.emit(IrOp::Add(result, lr, rr)),
                BinOp::Sub => self.emit(IrOp::Sub(result, lr, rr)),
                BinOp::Mul => self.emit(IrOp::Mul(result, lr, rr)),
                // ...
                BinOp::Eq => self.emit(IrOp::Eq(result, lr, rr)),
            }
            Ok(result)
        }

        Expr::If { condition, then_block, else_block, .. } => {
            self.gen_if(condition, then_block, else_block.as_deref())
        }

        Expr::Call { callee, args, .. } => {
            self.gen_call(callee, args)
        }

        // ...30+ more
    }
}
```

The pattern is always: generate sub-expressions into registers, emit the op, return the
result register.

## `gen_if`: branches become blocks

```rust
fn gen_if(&mut self, cond: &Expr, then: &Block, else_: Option<&Expr>)
    -> Result<Reg, PipelineError>
{
    let cond_reg = self.gen_expr(cond)?;

    let then_block = self.alloc_block();
    let else_block = self.alloc_block();
    let join_block = self.alloc_block();

    // Terminate current block with Branch
    self.terminate(Terminator::Branch {
        cond: cond_reg,
        then_block,
        else_block,
    });

    // Generate then branch
    self.switch_to_block(then_block);
    let then_reg = self.gen_block(then)?;
    let then_result_slot = self.alloc_local("__if_result");
    self.emit(IrOp::StoreLocal(then_result_slot, then_reg));
    self.terminate(Terminator::Jump(join_block));

    // Generate else branch
    self.switch_to_block(else_block);
    let else_reg = match else_ {
        Some(e) => self.gen_expr(e)?,
        None => { let r = self.alloc_reg(); self.emit(IrOp::ConstUnit(r)); r }
    };
    self.emit(IrOp::StoreLocal(then_result_slot, else_reg));
    self.terminate(Terminator::Jump(join_block));

    // Join block: load the result
    self.switch_to_block(join_block);
    let result = self.alloc_reg();
    self.emit(IrOp::LoadLocal(result, then_result_slot));
    Ok(result)
}
```

The join value is passed through a local slot (store in each branch, load in join).
This avoids full SSA Phi node complexity for the common case.

## `gen_stmt`: statements are side effects

Statements do not produce values (they return `Ok(())`):

```rust
fn gen_stmt(&mut self, stmt: &Stmt) -> Result<(), PipelineError> {
    match stmt {
        Stmt::Let { name, value, .. } => {
            let slot = self.alloc_local(name);
            if let Some(expr) = value {
                let r = self.gen_expr(expr)?;
                self.emit(IrOp::StoreLocal(slot, r));
            }
            Ok(())
        }

        Stmt::Return { value, .. } => {
            let r = match value {
                Some(expr) => self.gen_expr(expr)?,
                None => { let r = self.alloc_reg(); self.emit(IrOp::ConstUnit(r)); r }
            };
            self.terminate(Terminator::Return(r));
            Ok(())
        }

        Stmt::While { condition, body, .. } => {
            self.gen_while(condition, body)
        }

        Stmt::Expr { expr, .. } => {
            self.gen_expr(expr)?;  // result register ignored
            Ok(())
        }

        // ...
    }
}
```

`Stmt::Expr` calls `gen_expr` and ignores the result register. The expression's IR is
emitted for its side effects (the `println` call happens inside `gen_expr`).

## The helper methods

IR gen has three helper methods used everywhere:

```rust
fn alloc_reg(&mut self) -> Reg { let r = self.next_reg; self.next_reg += 1; r }
fn alloc_block(&mut self) -> BlockId { let b = self.next_block; self.next_block += 1; b }
fn alloc_local(&mut self, name: &str) -> usize {
    let slot = self.local_count;
    self.local_count += 1;
    self.locals.insert(name.to_string(), slot);
    slot
}
fn emit(&mut self, op: IrOp) { self.current.blocks[self.current_block].ops.push(op); }
fn terminate(&mut self, t: Terminator) {
    self.current.blocks[self.current_block].terminator = t;
}
fn switch_to_block(&mut self, b: BlockId) { self.current_block = b; }
```

Every IR gen function is built from these six primitives.
