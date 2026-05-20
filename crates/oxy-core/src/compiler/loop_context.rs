//! Tracks loop nesting for break/continue backpatching.

pub(crate) struct LoopContext {
    pub(crate) label: Option<String>,
    pub(crate) continue_target: usize,
    pub(crate) break_patches: Vec<usize>,
    pub(crate) continue_patches: Vec<usize>,
}
