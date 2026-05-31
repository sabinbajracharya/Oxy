//! Register-based IR with basic blocks and CFG.
//!
//! Virtual registers are infinite (`Reg = usize`). Each `IrOp` defines a register.
//! Complex operations call FFI helpers via `CallBuiltin`; simple arithmetic/comparison
//! is inlined in codegen. The IR has no operand stack — register allocation is done
//! by Cranelift's SSA construction.

use crate::type_checker::TypeInfo;

/// Virtual register index.
pub(crate) type Reg = usize;

/// Block identifier.
pub(crate) type BlockId = usize;

/// A function in register IR form with basic-block CFG.
pub(crate) struct IrFunction {
    /// Position in the functions Vec = codegen fn_index. Set by ir_gen before pushing.
    pub fn_index: usize,
    pub name: String,
    pub blocks: Vec<BasicBlock>,
    pub entry: BlockId,
    /// Number of local variable slots needed (captures + declared locals).
    pub local_count: usize,
    pub return_type: TypeInfo,
    /// Parameter names and types.
    pub params: Vec<(String, TypeInfo)>,
    /// Captured variables from enclosing scope (name, slot_index).
    pub captures: Vec<(String, usize)>,
    /// Whether this is an async function (needs yield state save/restore).
    pub is_async: bool,
}

/// A basic block: straight-line register operations ending with a terminator.
pub(crate) struct BasicBlock {
    pub id: BlockId,
    pub ops: Vec<IrOp>,
    pub terminator: Terminator,
    /// CFG predecessor edges. Reserved for dataflow/SSA analysis passes; not yet
    /// populated or read by the current ir_gen/codegen (see IR_DESIGN.md).
    #[allow(dead_code)]
    pub predecessors: Vec<BlockId>,
}

/// A register operation. The result register is always the first field.
#[derive(Debug, Clone)]
pub(crate) enum IrOp {
    // ── Constants ──────────────────────────────────────────────────────
    ConstInt(Reg, i64),
    ConstFloat(Reg, f64),
    ConstBool(Reg, bool),
    ConstChar(Reg, char),
    ConstUnit(Reg),
    /// String constant (stored inline in the IR, codegen pushes it via FFI).
    ConstString(Reg, String),

    // ── Locals ─────────────────────────────────────────────────────────
    /// Load a local variable from slot `index` into register.
    LoadLocal(Reg, usize),
    /// Load a local variable WITHOUT Cell unwrapping (for method-call receivers).
    LoadLocalRaw(Reg, usize),
    /// Store register value into local slot `index`.
    StoreLocal(usize, Reg),
    /// Promote local slot `index` to a `Value::Cell` (shared mutable box) so a
    /// closure capturing it observes the same storage. Emitted for a `let mut`
    /// binding that is captured by a closure; subsequent loads/stores transparently
    /// go through the cell, and the captured closure shares the same `Rc<RefCell>`.
    MakeCell(usize),

    // ── Binary arithmetic (inlined in CLIF) ────────────────────────────
    Add(Reg, Reg, Reg),
    Sub(Reg, Reg, Reg),
    Mul(Reg, Reg, Reg),
    Div(Reg, Reg, Reg),
    Rem(Reg, Reg, Reg),
    Eq(Reg, Reg, Reg),
    Neq(Reg, Reg, Reg),
    Lt(Reg, Reg, Reg),
    Gt(Reg, Reg, Reg),
    Le(Reg, Reg, Reg),
    Ge(Reg, Reg, Reg),
    And(Reg, Reg, Reg),
    Or(Reg, Reg, Reg),

    // ── Bitwise (inlined in CLIF) ──────────────────────────────────────
    BitAnd(Reg, Reg, Reg),
    BitOr(Reg, Reg, Reg),
    BitXor(Reg, Reg, Reg),
    Shl(Reg, Reg, Reg),
    Shr(Reg, Reg, Reg),

    // ── Unary (inlined in CLIF) ────────────────────────────────────────
    Neg(Reg, Reg),
    Not(Reg, Reg),
    BitNot(Reg, Reg),

    // ── FFI-backed operations ──────────────────────────────────────────
    /// Call an `oxy_*` FFI function. Codegen pushes args onto the operand
    /// stack (in JitContext), calls the FFI function, and pops the result
    /// into `result` register.
    CallBuiltin {
        result: Reg,
        /// FFI function name (e.g. "oxy_push_int", "oxy_add", "oxy_struct_init").
        func: &'static str,
        /// Register arguments to push before calling.
        args: Vec<Reg>,
        /// Extra immediate arguments (e.g. field_count, meta_idx, usize params).
        immediates: Vec<usize>,
        /// String metadata (function names, field names, method names, paths).
        strings: Vec<String>,
    },

    // ── Special ────────────────────────────────────────────────────────
    /// Copy a register value (used when a value is needed in multiple places).
    Copy(Reg, Reg),
    /// Read the result slot from ctx after a function call returns.
    ///
    /// Result/error-plumbing vocabulary (see IR_DESIGN.md): implemented in both
    /// backends and codegen, but ir_gen currently routes returns through
    /// `Terminator::Return` and the FFI rather than emitting these directly.
    #[allow(dead_code)]
    ReadResult(Reg),
    /// Write register to ctx.result for function return.
    #[allow(dead_code)]
    WriteResult(Reg),
    /// Set error message in ctx. Emitted for explicit `panic!()` calls.
    SetError(Reg),
    /// Check if ctx has an error set (returns bool-like in result register).
    CheckError(Reg),

    // ── Phi node (block parameter) ─────────────────────────────────────
    Phi(Reg, Reg, Reg),
}

impl IrOp {
    /// Return the register that this op defines (if any).
    /// StoreLocal returns 0 as it defines no register.
    pub(crate) fn result_reg(&self) -> Reg {
        match self {
            IrOp::ConstInt(r, _)
            | IrOp::ConstBool(r, _)
            | IrOp::ConstUnit(r)
            | IrOp::ConstFloat(r, _)
            | IrOp::ConstChar(r, _)
            | IrOp::ConstString(r, _)
            | IrOp::LoadLocal(r, _)
            | IrOp::LoadLocalRaw(r, _)
            | IrOp::Add(r, _, _)
            | IrOp::Sub(r, _, _)
            | IrOp::Mul(r, _, _)
            | IrOp::Div(r, _, _)
            | IrOp::Rem(r, _, _)
            | IrOp::Eq(r, _, _)
            | IrOp::Neq(r, _, _)
            | IrOp::Lt(r, _, _)
            | IrOp::Gt(r, _, _)
            | IrOp::Le(r, _, _)
            | IrOp::Ge(r, _, _)
            | IrOp::And(r, _, _)
            | IrOp::Or(r, _, _)
            | IrOp::BitAnd(r, _, _)
            | IrOp::BitOr(r, _, _)
            | IrOp::BitXor(r, _, _)
            | IrOp::Shl(r, _, _)
            | IrOp::Shr(r, _, _)
            | IrOp::Neg(r, _)
            | IrOp::Not(r, _)
            | IrOp::BitNot(r, _)
            | IrOp::Copy(r, _)
            | IrOp::Phi(r, _, _)
            | IrOp::CallBuiltin { result: r, .. }
            | IrOp::ReadResult(r)
            | IrOp::WriteResult(r)
            | IrOp::SetError(r)
            | IrOp::CheckError(r) => *r,
            IrOp::StoreLocal(_, _) | IrOp::MakeCell(_) => 0,
        }
    }
}

/// How control flow leaves a basic block.
#[derive(Debug, Clone)]
pub(crate) enum Terminator {
    /// Return from the function with the given register value.
    Return(Reg),
    /// Unconditional jump to another block.
    Jump(BlockId),
    /// Conditional branch: if `cond` is truthy, go to `then_block`, else `else_block`.
    Branch {
        cond: Reg,
        then_block: BlockId,
        else_block: BlockId,
    },
    /// Halt execution (end of program).
    Halt,
    /// Early-exit with error discriminant 2. The register names the value that
    /// triggered the exit (informational — used in IR display and snapshots).
    /// The error state must already be set by a preceding op (e.g. oxy_try_pop
    /// for `?`, or SetError for explicit panics). This terminator does NOT call
    /// oxy_panic itself — it just exits.
    Panic(Reg),
}

impl Terminator {
    /// True if this is the default terminator of a newly created block (Halt).
    /// If the body set a different terminator (return, break, continue), we must
    /// not overwrite it with a loop-back Jump.
    pub(crate) fn is_default(&self) -> bool {
        matches!(self, Terminator::Halt)
    }
}

impl IrFunction {
    pub(crate) fn new(name: String, entry: BlockId, local_count: usize, fn_index: usize) -> Self {
        Self {
            fn_index,
            name,
            blocks: Vec::new(),
            entry,
            local_count,
            return_type: TypeInfo::Unit,
            params: Vec::new(),
            captures: Vec::new(),
            is_async: false,
        }
    }

    pub(crate) fn add_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock {
            id,
            ops: Vec::new(),
            terminator: Terminator::Halt,
            predecessors: Vec::new(),
        });
        id
    }

    pub(crate) fn block_mut(&mut self, id: BlockId) -> &mut BasicBlock {
        &mut self.blocks[id]
    }
}

impl BasicBlock {
    pub(crate) fn push(&mut self, op: IrOp) {
        self.ops.push(op);
    }

    pub(crate) fn terminate(&mut self, term: Terminator) {
        self.terminator = term;
    }
}

// ── Display for tracing ────────────────────────────────────────────────

impl std::fmt::Display for IrOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IrOp::ConstInt(r, n) => write!(f, "r{r} = ConstInt({n})"),
            IrOp::ConstFloat(r, n) => write!(f, "r{r} = ConstFloat({n})"),
            IrOp::ConstBool(r, b) => write!(f, "r{r} = ConstBool({b})"),
            IrOp::ConstChar(r, c) => write!(f, "r{r} = ConstChar('{c}')"),
            IrOp::ConstUnit(r) => write!(f, "r{r} = ConstUnit"),
            IrOp::ConstString(r, s) => write!(f, "r{r} = ConstString(\"{s}\")"),
            IrOp::LoadLocal(r, s) => write!(f, "r{r} = LoadLocal({s})"),
            IrOp::LoadLocalRaw(r, s) => write!(f, "r{r} = LoadLocalRaw({s})"),
            IrOp::StoreLocal(s, r) => write!(f, "StoreLocal({s}, r{r})"),
            IrOp::MakeCell(s) => write!(f, "MakeCell({s})"),
            IrOp::Add(r, a, b) => write!(f, "r{r} = Add(r{a}, r{b})"),
            IrOp::Sub(r, a, b) => write!(f, "r{r} = Sub(r{a}, r{b})"),
            IrOp::Mul(r, a, b) => write!(f, "r{r} = Mul(r{a}, r{b})"),
            IrOp::Div(r, a, b) => write!(f, "r{r} = Div(r{a}, r{b})"),
            IrOp::Rem(r, a, b) => write!(f, "r{r} = Rem(r{a}, r{b})"),
            IrOp::Eq(r, a, b) => write!(f, "r{r} = Eq(r{a}, r{b})"),
            IrOp::Neq(r, a, b) => write!(f, "r{r} = Neq(r{a}, r{b})"),
            IrOp::Lt(r, a, b) => write!(f, "r{r} = Lt(r{a}, r{b})"),
            IrOp::Gt(r, a, b) => write!(f, "r{r} = Gt(r{a}, r{b})"),
            IrOp::Le(r, a, b) => write!(f, "r{r} = Le(r{a}, r{b})"),
            IrOp::Ge(r, a, b) => write!(f, "r{r} = Ge(r{a}, r{b})"),
            IrOp::And(r, a, b) => write!(f, "r{r} = And(r{a}, r{b})"),
            IrOp::Or(r, a, b) => write!(f, "r{r} = Or(r{a}, r{b})"),
            IrOp::BitAnd(r, a, b) => write!(f, "r{r} = BitAnd(r{a}, r{b})"),
            IrOp::BitOr(r, a, b) => write!(f, "r{r} = BitOr(r{a}, r{b})"),
            IrOp::BitXor(r, a, b) => write!(f, "r{r} = BitXor(r{a}, r{b})"),
            IrOp::Shl(r, a, b) => write!(f, "r{r} = Shl(r{a}, r{b})"),
            IrOp::Shr(r, a, b) => write!(f, "r{r} = Shr(r{a}, r{b})"),
            IrOp::Neg(r, a) => write!(f, "r{r} = Neg(r{a})"),
            IrOp::Not(r, a) => write!(f, "r{r} = Not(r{a})"),
            IrOp::BitNot(r, a) => write!(f, "r{r} = BitNot(r{a})"),
            IrOp::Copy(r, a) => write!(f, "r{r} = Copy(r{a})"),
            IrOp::Phi(r, a, b) => write!(f, "r{r} = Phi(r{a}, r{b})"),
            IrOp::CallBuiltin {
                result,
                func,
                args,
                immediates,
                strings,
            } => {
                write!(
                    f,
                    "r{result} = Call {func}(args={args:?}, imm={immediates:?}, strs={strings:?})"
                )
            }
            IrOp::ReadResult(r) => write!(f, "r{r} = ReadResult"),
            IrOp::WriteResult(r) => write!(f, "r{r} = WriteResult"),
            IrOp::SetError(r) => write!(f, "r{r} = SetError"),
            IrOp::CheckError(r) => write!(f, "r{r} = CheckError"),
        }
    }
}

impl std::fmt::Display for Terminator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Terminator::Return(r) => write!(f, "Return(r{r})"),
            Terminator::Jump(b) => write!(f, "Jump(block{b})"),
            Terminator::Branch {
                cond,
                then_block,
                else_block,
            } => {
                write!(
                    f,
                    "Branch(r{cond}, then=block{then_block}, else=block{else_block})"
                )
            }
            Terminator::Halt => write!(f, "Halt"),
            Terminator::Panic(r) => write!(f, "Panic(r{r})"),
        }
    }
}

impl IrFunction {
    /// Render this function's IR for `OXY_VM_TRACE`. Only reached from the
    /// native codegen path, so it is dead on wasm (where codegen isn't built).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(crate) fn dump(&self) -> String {
        let mut out = String::new();
        use std::fmt::Write;
        let _ = writeln!(out, "── fn {} (locals={}) ──", self.name, self.local_count);
        for block in &self.blocks {
            let _ = writeln!(out, "  block{}:", block.id);
            for op in &block.ops {
                let _ = writeln!(out, "    {op}");
            }
            let _ = writeln!(out, "    {term}", term = block.terminator);
        }
        out
    }
}
