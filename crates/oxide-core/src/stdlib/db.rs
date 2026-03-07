//! SQLite database module for Oxide.
//!
//! Provides a simple interface to SQLite databases with parameterized
//! queries, returning results as Oxide `Value` types.

use std::collections::HashMap;

use rusqlite::{params_from_iter, types::Value as SqlValue, Connection};

use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

/// Open a SQLite database file. Creates the file if it doesn't exist.
pub fn open(path: &str, span: &Span) -> Result<Connection, FerriError> {
    Connection::open(path).map_err(|e| FerriError::Runtime {
        message: format!("failed to open database '{path}': {e}"),
        line: span.line,
        column: span.column,
    })
}

/// Open an in-memory SQLite database.
pub fn open_in_memory(span: &Span) -> Result<Connection, FerriError> {
    Connection::open_in_memory().map_err(|e| FerriError::Runtime {
        message: format!("failed to open in-memory database: {e}"),
        line: span.line,
        column: span.column,
    })
}

/// Execute a SQL statement that doesn't return rows (INSERT, UPDATE, DELETE, CREATE, etc.).
/// Returns the number of rows affected.
pub fn execute(
    conn: &Connection,
    sql: &str,
    params: &[Value],
    span: &Span,
) -> Result<Value, FerriError> {
    let sql_params = convert_params(params, span)?;
    let affected = conn
        .execute(sql, params_from_iter(sql_params.iter()))
        .map_err(|e| FerriError::Runtime {
            message: format!("SQL execute error: {e}"),
            line: span.line,
            column: span.column,
        })?;
    Ok(Value::Integer(affected as i64))
}

/// Execute a SQL query and return results as a Vec of HashMaps.
pub fn query(
    conn: &Connection,
    sql: &str,
    params: &[Value],
    span: &Span,
) -> Result<Value, FerriError> {
    let sql_params = convert_params(params, span)?;
    let mut stmt = conn.prepare(sql).map_err(|e| FerriError::Runtime {
        message: format!("SQL prepare error: {e}"),
        line: span.line,
        column: span.column,
    })?;

    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    let rows = stmt
        .query_map(params_from_iter(sql_params.iter()), |row| {
            let mut fields = HashMap::new();
            for (i, col_name) in column_names.iter().enumerate() {
                let val: SqlValue = row.get_unwrap(i);
                fields.insert(col_name.clone(), sql_value_to_oxide(val));
            }
            Ok(fields)
        })
        .map_err(|e| FerriError::Runtime {
            message: format!("SQL query error: {e}"),
            line: span.line,
            column: span.column,
        })?;

    let mut result = Vec::new();
    for row in rows {
        let fields = row.map_err(|e| FerriError::Runtime {
            message: format!("SQL row error: {e}"),
            line: span.line,
            column: span.column,
        })?;
        result.push(Value::HashMap(fields));
    }

    Ok(Value::Vec(result))
}

/// Get the last inserted row ID.
pub fn last_insert_id(conn: &Connection) -> Value {
    Value::Integer(conn.last_insert_rowid())
}

/// Convert Oxide Values to rusqlite-compatible parameters.
fn convert_params(params: &[Value], span: &Span) -> Result<Vec<SqlValue>, FerriError> {
    params
        .iter()
        .map(|v| match v {
            Value::Integer(n) => Ok(SqlValue::Integer(*n)),
            Value::Float(f) => Ok(SqlValue::Real(*f)),
            Value::String(s) => Ok(SqlValue::Text(s.clone())),
            Value::Bool(b) => Ok(SqlValue::Integer(if *b { 1 } else { 0 })),
            Value::Unit => Ok(SqlValue::Null),
            _ => Err(FerriError::Runtime {
                message: format!("cannot use {} as SQL parameter", v.type_name()),
                line: span.line,
                column: span.column,
            }),
        })
        .collect()
}

/// Convert a rusqlite Value to a Oxide Value.
fn sql_value_to_oxide(val: SqlValue) -> Value {
    match val {
        SqlValue::Null => Value::Unit,
        SqlValue::Integer(n) => Value::Integer(n),
        SqlValue::Real(f) => Value::Float(f),
        SqlValue::Text(s) => Value::String(s),
        SqlValue::Blob(b) => Value::String(String::from_utf8_lossy(&b).to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_span() -> Span {
        Span {
            start: 0,
            end: 0,
            line: 0,
            column: 0,
        }
    }

    #[test]
    fn test_open_in_memory() {
        let conn = open_in_memory(&test_span()).unwrap();
        drop(conn);
    }

    #[test]
    fn test_create_and_insert() {
        let conn = open_in_memory(&test_span()).unwrap();
        let span = test_span();
        execute(
            &conn,
            "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)",
            &[],
            &span,
        )
        .unwrap();
        let affected = execute(
            &conn,
            "INSERT INTO test (name) VALUES (?1)",
            &[Value::String("Alice".to_string())],
            &span,
        )
        .unwrap();
        assert_eq!(affected, Value::Integer(1));
    }

    #[test]
    fn test_query_results() {
        let conn = open_in_memory(&test_span()).unwrap();
        let span = test_span();
        execute(
            &conn,
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)",
            &[],
            &span,
        )
        .unwrap();
        execute(
            &conn,
            "INSERT INTO users (name, age) VALUES (?1, ?2)",
            &[Value::String("Alice".to_string()), Value::Integer(30)],
            &span,
        )
        .unwrap();
        execute(
            &conn,
            "INSERT INTO users (name, age) VALUES (?1, ?2)",
            &[Value::String("Bob".to_string()), Value::Integer(25)],
            &span,
        )
        .unwrap();

        let result = query(
            &conn,
            "SELECT name, age FROM users ORDER BY age",
            &[],
            &span,
        )
        .unwrap();
        if let Value::Vec(rows) = result {
            assert_eq!(rows.len(), 2);
            if let Value::HashMap(row) = &rows[0] {
                assert_eq!(row.get("name"), Some(&Value::String("Bob".to_string())));
                assert_eq!(row.get("age"), Some(&Value::Integer(25)));
            }
        } else {
            panic!("expected Vec");
        }
    }

    #[test]
    fn test_parameterized_query() {
        let conn = open_in_memory(&test_span()).unwrap();
        let span = test_span();
        execute(
            &conn,
            "CREATE TABLE items (id INTEGER PRIMARY KEY, value REAL)",
            &[],
            &span,
        )
        .unwrap();
        execute(
            &conn,
            "INSERT INTO items (value) VALUES (?1)",
            &[Value::Float(3.125)],
            &span,
        )
        .unwrap();

        let result = query(
            &conn,
            "SELECT value FROM items WHERE value > ?1",
            &[Value::Float(3.0)],
            &span,
        )
        .unwrap();
        if let Value::Vec(rows) = result {
            assert_eq!(rows.len(), 1);
        } else {
            panic!("expected Vec");
        }
    }

    #[test]
    fn test_last_insert_id() {
        let conn = open_in_memory(&test_span()).unwrap();
        let span = test_span();
        execute(
            &conn,
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            &[],
            &span,
        )
        .unwrap();
        execute(
            &conn,
            "INSERT INTO t (v) VALUES (?1)",
            &[Value::String("x".to_string())],
            &span,
        )
        .unwrap();
        let id = last_insert_id(&conn);
        assert_eq!(id, Value::Integer(1));
    }

    #[test]
    fn test_null_handling() {
        let conn = open_in_memory(&test_span()).unwrap();
        let span = test_span();
        execute(
            &conn,
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            &[],
            &span,
        )
        .unwrap();
        execute(&conn, "INSERT INTO t (v) VALUES (NULL)", &[], &span).unwrap();

        let result = query(&conn, "SELECT v FROM t", &[], &span).unwrap();
        if let Value::Vec(rows) = result {
            if let Value::HashMap(row) = &rows[0] {
                assert_eq!(row.get("v"), Some(&Value::Unit));
            }
        }
    }

    #[test]
    fn test_convert_params_invalid() {
        let span = test_span();
        let result = convert_params(&[Value::Vec(vec![])], &span);
        assert!(result.is_err());
    }
}
