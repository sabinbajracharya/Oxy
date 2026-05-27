// Stub — the bytecode compiler has been retired in favor of AST → Register IR + CFG.
// This stub keeps existing callers compiling while ir_gen.rs is being built.
// Once ir_gen+codegen are wired, this file and api.rs will be updated.

use crate::ast::Program;
use crate::errors::FerriError;
use crate::vm::Chunk;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct Compiler {
    #[allow(dead_code)]
    source_path: Option<String>,
    #[allow(dead_code)]
    externs: HashMap<String, PathBuf>,
}

impl Compiler {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            source_path: None,
            externs: HashMap::new(),
        }
    }

    pub fn new_with_options(source_path: Option<&str>, externs: HashMap<String, PathBuf>) -> Self {
        Self {
            source_path: source_path.map(|s| s.to_string()),
            externs,
        }
    }

    pub fn new_for_tests(source_path: Option<&str>) -> Self {
        Self {
            source_path: source_path.map(|s| s.to_string()),
            externs: HashMap::new(),
        }
    }

    pub fn with_externs(mut self, externs: HashMap<String, PathBuf>) -> Self {
        self.externs = externs;
        self
    }

    pub fn compile(&self, _program: &Program) -> Result<Chunk, FerriError> {
        Err(FerriError::Runtime {
            message: "bytecode compiler retired — AST→IR path not yet wired".to_string(),
            line: 0,
            column: 0,
        })
    }
}
