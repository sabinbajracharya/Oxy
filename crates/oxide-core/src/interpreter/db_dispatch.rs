//! SQLite database dispatch for the Oxide interpreter.
//!
//! Handles `Db::open()`, `Db::memory()`, and method calls on database
//! values (`.execute()`, `.query()`, `.query_row()`, `.last_insert_id()`,
//! `.close()`).

use std::collections::HashMap;

use rusqlite::Connection;

use crate::errors::{check_arg_count, expect_string, FerriError};
use crate::lexer::Span;
use crate::stdlib::db;
use crate::types::Value;

use super::Interpreter;

impl Interpreter {
    /// Handle `Db::open(path)` and `Db::memory()` path calls.
    pub(crate) fn call_db_path(
        &mut self,
        method_name: &str,
        args: &[Value],
        span: &Span,
    ) -> Result<Value, FerriError> {
        match method_name {
            "open" => {
                check_arg_count("Db::open", 1, args, span)?;
                let path = expect_string(&args[0], "Db::open()", span)?;
                let conn = db::open(path, span)?;
                Ok(self.store_db_connection(conn))
            }
            "memory" => {
                check_arg_count("Db::memory", 0, args, span)?;
                let conn = db::open_in_memory(span)?;
                Ok(self.store_db_connection(conn))
            }
            _ => Err(FerriError::Runtime {
                message: format!("unknown Db method `{method_name}`"),
                line: span.line,
                column: span.column,
            }),
        }
    }

    /// Handle method calls on Db values: .execute(), .query(), .close(), etc.
    pub(crate) fn call_db_method(
        &mut self,
        receiver: &Value,
        method: &str,
        args: Vec<Value>,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let db_id = self.get_db_id(receiver, span)?;

        match method {
            "execute" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(FerriError::Runtime {
                        message: "execute() takes 1-2 arguments (sql, optional params)".to_string(),
                        line: span.line,
                        column: span.column,
                    });
                }
                let sql = expect_string(&args[0], "execute()", span)?;
                let params = if args.len() > 1 {
                    extract_vec_params(&args[1], span)?
                } else {
                    vec![]
                };
                let conn = self.get_db_connection(&db_id, span)?;
                db::execute(conn, sql, &params, span)
            }
            "query" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(FerriError::Runtime {
                        message: "query() takes 1-2 arguments (sql, optional params)".to_string(),
                        line: span.line,
                        column: span.column,
                    });
                }
                let sql = expect_string(&args[0], "query()", span)?;
                let params = if args.len() > 1 {
                    extract_vec_params(&args[1], span)?
                } else {
                    vec![]
                };
                let conn = self.get_db_connection(&db_id, span)?;
                db::query(conn, sql, &params, span)
            }
            "query_row" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(FerriError::Runtime {
                        message: "query_row() takes 1-2 arguments (sql, optional params)"
                            .to_string(),
                        line: span.line,
                        column: span.column,
                    });
                }
                let sql = expect_string(&args[0], "query_row()", span)?;
                let params = if args.len() > 1 {
                    extract_vec_params(&args[1], span)?
                } else {
                    vec![]
                };
                let conn = self.get_db_connection(&db_id, span)?;
                let result = db::query(conn, sql, &params, span)?;
                // Return first row or None
                if let Value::Vec(rows) = result {
                    if let Some(row) = rows.into_iter().next() {
                        Ok(Value::some(row))
                    } else {
                        Ok(Value::none())
                    }
                } else {
                    Ok(Value::none())
                }
            }
            "last_insert_id" => {
                check_arg_count("last_insert_id", 0, &args, span)?;
                let conn = self.get_db_connection(&db_id, span)?;
                Ok(db::last_insert_id(conn))
            }
            "close" => {
                check_arg_count("close", 0, &args, span)?;
                self.db_connections.remove(&db_id);
                Ok(Value::Unit)
            }
            _ => Err(FerriError::Runtime {
                message: format!("unknown Db method `{method}`"),
                line: span.line,
                column: span.column,
            }),
        }
    }

    /// Store a connection and return a Db value.
    fn store_db_connection(&mut self, conn: Connection) -> Value {
        self.db_id_counter += 1;
        let id = format!("__db_{}", self.db_id_counter);
        self.db_connections.insert(id.clone(), conn);
        let mut fields = HashMap::new();
        fields.insert("__id".to_string(), Value::String(id));
        Value::Struct {
            name: "Db".to_string(),
            fields,
        }
    }

    /// Extract the database ID from a Db struct value.
    fn get_db_id(&self, value: &Value, span: &Span) -> Result<String, FerriError> {
        if let Value::Struct { name, fields } = value {
            if name == "Db" {
                if let Some(Value::String(id)) = fields.get("__id") {
                    return Ok(id.clone());
                }
            }
        }
        Err(FerriError::Runtime {
            message: "expected a Db value".to_string(),
            line: span.line,
            column: span.column,
        })
    }

    /// Get a reference to a stored database connection.
    fn get_db_connection(&self, db_id: &str, span: &Span) -> Result<&Connection, FerriError> {
        self.db_connections.get(db_id).ok_or(FerriError::Runtime {
            message: "database connection is closed".to_string(),
            line: span.line,
            column: span.column,
        })
    }
}

/// Extract a Vec of params from a Value::Vec argument.
fn extract_vec_params(value: &Value, span: &Span) -> Result<Vec<Value>, FerriError> {
    match value {
        Value::Vec(v) => Ok(v.clone()),
        _ => Err(FerriError::Runtime {
            message: format!("expected Vec for SQL parameters, got {}", value.type_name()),
            line: span.line,
            column: span.column,
        }),
    }
}
