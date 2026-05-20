//! Tracks loop nesting for break/continue backpatching.
//!
//! ```text
//! loop_context.rs  ── pub(crate) struct LoopContext, no dependencies
//!   re-exported by: mod.rs (pub(crate) use loop_context::LoopContext)
//!   used by: Compiler.loop_stack field, expr.rs (compile_stmt)
//! ```

pub(crate) struct LoopContext {
    pub(crate) label: Option<String>,
    pub(crate) continue_target: usize,
    pub(crate) break_patches: Vec<usize>,
    pub(crate) continue_patches: Vec<usize>,
}
