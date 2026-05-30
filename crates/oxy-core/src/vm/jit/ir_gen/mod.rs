//! AST → Register IR + CFG code generator.
//!
//! Walks typed AST items and emits register-based IR with basic blocks.
//! Replaces the old bytecode compiler (`compiler/`).
//!
//! # Pipeline
//! 1. Parse source → AST
//! 2. Type-check AST
//! 3. `IrGen::gen_program()` → `Vec<IrFunction>` (register IR + CFG)
//! 4. `codegen.rs` → Cranelift CLIF

use super::ir::*;
use crate::ast::*;
use crate::type_checker::TypeInfo;

/// IR code generator. Walks a typed AST and produces register IR.
pub(crate) struct IrGen {
    /// All generated functions (including closures, async blocks).
    pub(crate) functions: Vec<IrFunction>,
    /// Closure metadata indexed by meta_idx (param_names, captures, is_async).
    pub(crate) closure_meta: Vec<(Vec<String>, Vec<(String, usize, bool)>, bool)>,
    /// Current function being generated.
    current: IrFunction,
    /// Current basic block being built.
    current_block: BlockId,
    /// Next available virtual register.
    next_reg: Reg,
    /// Next available block ID.
    next_block: BlockId,
    /// Local variable name → slot index.
    locals: std::collections::HashMap<String, usize>,
    /// Number of local slots allocated.
    local_count: usize,
    /// Current break target (loop exit block).
    break_target: Option<BlockId>,
    /// Current continue target (loop header block).
    continue_target: Option<BlockId>,
    /// Slot to store break value for `loop { break expr; }` result propagation.
    break_value_slot: Option<usize>,
    /// Labeled loop targets: label → (break_block, continue_block).
    labeled_targets: std::collections::HashMap<String, (BlockId, BlockId)>,
    /// Global const values: name → value expression (from `const NAME = expr;`).
    global_consts: std::collections::HashMap<String, crate::ast::Expr>,
    /// Use aliases: local_name → qualified_name (from `use path::to::item;`).
    use_aliases: std::collections::HashMap<String, String>,
    /// Source modules brought into scope by glob imports (`use path::to::*;`),
    /// already resolved to absolute paths. A bare callee that matches no alias
    /// or sibling is resolved against these by trying `glob_mod::name`.
    glob_mods: Vec<String>,
    /// Variant name → parent enum name (e.g. "Some" → "Option").
    /// Seeded from AST enum definitions so user-defined enums work without hardcoding.
    variant_to_enum: std::collections::HashMap<String, String>,
    /// Local slot → type annotation name (for width coercion on compound assignment).
    local_types: std::collections::HashMap<usize, String>,
    /// Current module path prefix for resolving self/super/crate in ir_gen.
    current_module_prefix: String,
    /// Re-exported function aliases: module::local_name → original_qualified_name.
    /// Populated from `pub use` inside modules so call resolution can redirect.
    fn_aliases: std::collections::HashMap<String, String>,
    /// Trait definitions collected from Item::Trait during gen_program.
    /// Used to compile default method bodies for impl Trait blocks that don't override them.
    trait_defs: std::collections::HashMap<String, crate::ast::TraitDef>,
    /// Tuple-struct names → field arity (e.g. "WrappedInt" → 1, "shapes::Pair" → 2).
    /// A call `WrappedInt(17)` is lowered to struct construction with positional
    /// field names "0", "1", … — matching what `oxy_field_access` expects for `.0`.
    tuple_structs: std::collections::HashMap<String, usize>,
    /// Slots of `let mut` bindings in the current function — the candidates for
    /// Cell promotion when captured by a closure (see `MakeCell`).
    mut_slots: std::collections::HashSet<usize>,
    /// Slots already promoted to a `Value::Cell` via `MakeCell` in the current
    /// function. Guards against double-wrapping when several closures capture the
    /// same mutable variable.
    celled_slots: std::collections::HashSet<usize>,
    /// Generic free/module function templates by qualified name → (def, module
    /// prefix active at definition). Used to emit monomorphized copies for each
    /// turbofish instantiation (e.g. `make_zero::<int>()`).
    generic_fns: std::collections::HashMap<String, (crate::ast::FnDef, String)>,
    /// Active type-parameter → concrete-type substitution while lowering a
    /// monomorphized instance (e.g. `{T: "int"}`), so `T::zero()` resolves to
    /// `int::zero()`. Empty when lowering a non-generic function.
    type_subst: std::collections::HashMap<String, String>,
    /// Monomorphized instance names already emitted, for deduplication.
    mono_emitted: std::collections::HashSet<String>,
    /// Qualified names of unit structs (`struct Thing;`). A bare reference to
    /// one (e.g. `let t = Thing;`) constructs an empty `Value::Struct` rather
    /// than a function reference.
    unit_structs: std::collections::HashSet<String>,
    /// Qualified names of all free / module functions, collected before body
    /// lowering. Lets a call site qualify a bare callee with the current module
    /// prefix (sibling call) only when such a function actually exists.
    fn_names: std::collections::HashSet<String>,
    /// Type name of the impl block currently being lowered (e.g. "Counter" or
    /// "shapes::Point"). `Self` in a method body — both `Self { .. }` struct
    /// literals and `Self::assoc()` paths — resolves to this name so the value
    /// carries its concrete struct name for method dispatch. `None` outside a
    /// method body.
    current_self_type: Option<String>,
    /// Type aliases: qualified alias name → qualified target type name (from
    /// `type Alias = Target;`). Lets `Alias::Variant` and `Alias::assoc()`
    /// resolve to the underlying enum/struct. Collected in the pre-pass so
    /// forward references work regardless of definition order.
    type_aliases: std::collections::HashMap<String, String>,
}

impl IrGen {
    /// Register a variant → enum mapping. For module-level enums, the prefix is
    /// already baked into the enum name (e.g. "shapes::Color").
    fn register_enum(&mut self, enum_def: &crate::ast::EnumDef) {
        for variant in &enum_def.variants {
            self.variant_to_enum
                .insert(variant.name.clone(), enum_def.name.clone());
        }
    }
}

impl IrGen {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            closure_meta: Vec::new(),
            current: IrFunction::new(String::new(), 0, 0, usize::MAX),
            current_block: 0,
            next_reg: 0,
            next_block: 0,
            locals: std::collections::HashMap::new(),
            local_count: 0,
            break_target: None,
            continue_target: None,
            break_value_slot: None,
            labeled_targets: std::collections::HashMap::new(),
            global_consts: std::collections::HashMap::new(),
            use_aliases: std::collections::HashMap::new(),
            glob_mods: Vec::new(),
            variant_to_enum: Self::builtin_variants(),
            local_types: std::collections::HashMap::new(),
            current_module_prefix: String::new(),
            fn_aliases: std::collections::HashMap::new(),
            trait_defs: std::collections::HashMap::new(),
            tuple_structs: std::collections::HashMap::new(),
            mut_slots: std::collections::HashSet::new(),
            celled_slots: std::collections::HashSet::new(),
            generic_fns: std::collections::HashMap::new(),
            type_subst: std::collections::HashMap::new(),
            mono_emitted: std::collections::HashSet::new(),
            unit_structs: std::collections::HashSet::new(),
            fn_names: std::collections::HashSet::new(),
            current_self_type: None,
            type_aliases: std::collections::HashMap::new(),
        }
    }

    /// Built-in enum variants that don't come from user AST definitions.
    fn builtin_variants() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("Some".to_string(), "Option".to_string());
        m.insert("None".to_string(), "Option".to_string());
        m.insert("Ok".to_string(), "Result".to_string());
        m.insert("Err".to_string(), "Result".to_string());
        m
    }

    // ── Helpers ────────────────────────────────────────────────────────

    fn alloc_reg(&mut self) -> Reg {
        let r = self.next_reg;
        self.next_reg += 1;
        r
    }

    fn alloc_block(&mut self) -> BlockId {
        let id = self.next_block;
        self.next_block += 1;
        id
    }

    fn alloc_local(&mut self, name: &str) -> usize {
        let slot = self.local_count;
        self.locals.insert(name.to_string(), slot);
        self.local_count += 1;
        slot
    }

    fn lookup_local(&self, name: &str) -> Option<usize> {
        self.locals.get(name).copied()
    }

    fn emit(&mut self, op: IrOp) {
        self.current.block_mut(self.current_block).push(op);
    }

    fn terminate(&mut self, term: Terminator) {
        self.current.block_mut(self.current_block).terminate(term);
    }

    /// Emit a cast for a register value to the given type annotation.
    /// Returns the (possibly new) register holding the coerced value.
    fn coerce_reg(&mut self, reg: Reg, type_ann: &TypeAnnotation) -> Reg {
        match type_ann {
            TypeAnnotation::Named { name, .. } if name == "byte" => {
                let result = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result,
                    func: "oxy_cast_byte",
                    args: vec![reg],
                    immediates: vec![],
                    strings: vec![],
                });
                result
            }
            _ => reg,
        }
    }

    /// Emit a cast for a register value to the given TypeInfo.
    fn coerce_reg_to_type_info(&mut self, reg: Reg, ty: &TypeInfo) -> Reg {
        if *ty == TypeInfo::U8 {
            let result = self.alloc_reg();
            self.emit(IrOp::CallBuiltin {
                result,
                func: "oxy_cast_byte",
                args: vec![reg],
                immediates: vec![],
                strings: vec![],
            });
            result
        } else {
            reg
        }
    }

    fn start_block(&mut self, id: BlockId) {
        while self.current.blocks.len() <= id {
            self.current.add_block();
        }
        self.current_block = id;
    }

    // ── Top-level ──────────────────────────────────────────────────────
}

mod closures;
mod control_flow;
mod expressions;
mod functions;
mod patterns;
mod resolve;
mod statements;

#[cfg(test)]
mod tests;
