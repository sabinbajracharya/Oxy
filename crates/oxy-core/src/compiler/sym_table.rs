//! Symbol table tracking local variables in the current scope.
//!
//! ```text
//! sym_table.rs  ── pub(crate) struct SymTable, no dependencies
//!   re-exported by: mod.rs (pub(crate) use sym_table::SymTable)
//!   used by: Compiler.sym field, expr.rs (compile_stmt/compile_expr)
//! ```

use std::collections::{HashMap, HashSet};

use crate::types::IntegerWidth;

#[derive(Clone)]
pub(crate) struct SymTable {
    pub(crate) locals: HashMap<String, usize>,
    pub(crate) mutable: HashSet<String>,
    /// Declared integer width per binding name. Set when the binding is
    /// `let x: u8 = ...` (or similar). Used to narrow reassignments and
    /// compound updates back to the declared width.
    pub(crate) declared_widths: HashMap<String, IntegerWidth>,
    pub(crate) next_slot: usize,
}

impl SymTable {
    pub(crate) fn new(start_slot: usize) -> Self {
        Self {
            locals: HashMap::new(),
            mutable: HashSet::new(),
            declared_widths: HashMap::new(),
            next_slot: start_slot,
        }
    }

    pub(crate) fn set_width(&mut self, name: &str, width: IntegerWidth) {
        self.declared_widths.insert(name.to_string(), width);
    }

    pub(crate) fn width_of(&self, name: &str) -> Option<IntegerWidth> {
        self.declared_widths.get(name).copied()
    }

    pub(crate) fn define(&mut self, name: &str) -> usize {
        let slot = self.next_slot;
        self.locals.insert(name.to_string(), slot);
        self.next_slot += 1;
        slot
    }

    pub(crate) fn define_mut(&mut self, name: &str) -> usize {
        self.mutable.insert(name.to_string());
        self.define(name)
    }

    pub(crate) fn is_mutable(&self, name: &str) -> bool {
        self.mutable.contains(name)
    }

    pub(crate) fn get(&self, name: &str) -> Option<usize> {
        self.locals.get(name).copied()
    }

    pub(crate) fn build_slot_names(&self) -> Vec<String> {
        let max_slot = self.locals.values().max().copied().unwrap_or(0);
        let size = (max_slot + 1).max(self.next_slot);
        let mut names = vec![String::new(); size];
        for (name, slot) in &self.locals {
            names[*slot] = name.clone();
        }
        names
    }
}
