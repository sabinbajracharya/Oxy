//! Symbol table tracking local variables in the current scope.

use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub(crate) struct SymTable {
    pub(crate) locals: HashMap<String, usize>,
    pub(crate) mutable: HashSet<String>,
    pub(crate) next_slot: usize,
}

impl SymTable {
    pub(crate) fn new(start_slot: usize) -> Self {
        Self {
            locals: HashMap::new(),
            mutable: HashSet::new(),
            next_slot: start_slot,
        }
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

    pub(crate) fn define_at(&mut self, name: &str, slot: usize) {
        self.locals.insert(name.to_string(), slot);
        if slot >= self.next_slot {
            self.next_slot = slot + 1;
        }
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
