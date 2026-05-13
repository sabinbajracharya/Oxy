use crate::ast::*;
use crate::env::{Env, Environment};
use crate::errors::FerriError;
use crate::types::Value;

use super::super::Interpreter;

impl Interpreter {
    /// Evaluate a block, creating a new scope and returning the value
    /// of the last expression.
    pub(crate) fn eval_block(&mut self, block: &Block, env: &Env) -> Result<Value, FerriError> {
        let block_env = Environment::child(env);
        let mut result = Value::Unit;

        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = i == block.stmts.len() - 1;
            let val = self.eval_stmt(stmt, &block_env)?;

            if is_last {
                match stmt {
                    Stmt::Expr { has_semicolon, .. } if !has_semicolon => {
                        result = val;
                    }
                    Stmt::Loop { .. }
                    | Stmt::While { .. }
                    | Stmt::For { .. }
                    | Stmt::ForDestructure { .. } => {
                        result = val;
                    }
                    _ => {
                        result = Value::Unit;
                    }
                }
            }
        }

        Ok(result)
    }
}
