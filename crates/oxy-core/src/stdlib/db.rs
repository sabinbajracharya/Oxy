//! SQLite database module for Oxy.
//!
//! Provides a simple interface to SQLite databases with parameterized
//! queries, returning results as Oxy `Value` types.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rusqlite::{params_from_iter, types::Value as SqlValue, Connection};

use crate::errors::{check_arg_count, expect_integer, expect_string, runtime_error, PipelineError};
use crate::lexer::Span;
use crate::types::Value;

thread_local! {
    /// Open SQLite connections keyed by integer handle. Returned to Oxy
    /// code as an opaque int; subsequent calls look up the connection here.
    static CONNS: RefCell<HashMap<i64, Rc<Connection>>> = RefCell::new(HashMap::new());
    static NEXT_HANDLE: std::cell::Cell<i64> = const { std::cell::Cell::new(1) };
}

fn register_conn(conn: Connection) -> i64 {
    let h = NEXT_HANDLE.with(|c| {
        let n = c.get();
        c.set(n + 1);
        n
    });
    CONNS.with(|m| m.borrow_mut().insert(h, Rc::new(conn)));
    h
}

fn get_conn(handle: i64) -> Option<Rc<Connection>> {
    CONNS.with(|m| m.borrow().get(&handle).cloned())
}

/// Dispatch `std::db::` function calls from Oxy code.
pub fn call(
    func_name: &str,
    args: &[Value],
    span: &Span,
    _cb: crate::stdlib::registry::ClosureInvoker<'_>,
) -> Result<Value, PipelineError> {
    match func_name {
        "open" => {
            check_arg_count("std::db::open", 1, args, span)?;
            let path = expect_string(&args[0], "std::db::open", span)?;
            match Connection::open(path) {
                Ok(c) => Ok(Value::ok(Value::I64(register_conn(c)))),
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }
        "open_in_memory" => {
            check_arg_count("std::db::open_in_memory", 0, args, span)?;
            match Connection::open_in_memory() {
                Ok(c) => Ok(Value::ok(Value::I64(register_conn(c)))),
                Err(e) => Ok(Value::err(Value::String(e.to_string()))),
            }
        }
        "execute" => {
            if args.len() < 2 {
                return Err(runtime_error(
                    "std::db::execute(handle, sql, ...params) requires at least 2 arguments",
                    span,
                ));
            }
            let handle = expect_integer(&args[0], "std::db::execute (handle)", span)?;
            let sql = expect_string(&args[1], "std::db::execute (sql)", span)?;
            let conn = match get_conn(handle) {
                Some(c) => c,
                None => {
                    return Ok(Value::err(Value::String(format!(
                        "invalid db handle: {handle}"
                    ))))
                }
            };
            match execute(&conn, sql, &args[2..], span) {
                Ok(v) => Ok(Value::ok(v)),
                Err(PipelineError::Runtime { message, .. }) => {
                    Ok(Value::err(Value::String(message)))
                }
                Err(e) => Err(e),
            }
        }
        "query" => {
            if args.len() < 2 {
                return Err(runtime_error(
                    "std::db::query(handle, sql, ...params) requires at least 2 arguments",
                    span,
                ));
            }
            let handle = expect_integer(&args[0], "std::db::query (handle)", span)?;
            let sql = expect_string(&args[1], "std::db::query (sql)", span)?;
            let conn = match get_conn(handle) {
                Some(c) => c,
                None => {
                    return Ok(Value::err(Value::String(format!(
                        "invalid db handle: {handle}"
                    ))))
                }
            };
            match query(&conn, sql, &args[2..], span) {
                Ok(v) => Ok(Value::ok(v)),
                Err(PipelineError::Runtime { message, .. }) => {
                    Ok(Value::err(Value::String(message)))
                }
                Err(e) => Err(e),
            }
        }
        "last_insert_id" => {
            check_arg_count("std::db::last_insert_id", 1, args, span)?;
            let handle = expect_integer(&args[0], "std::db::last_insert_id", span)?;
            match get_conn(handle) {
                Some(conn) => Ok(last_insert_id(&conn)),
                None => Err(runtime_error(format!("invalid db handle: {handle}"), span)),
            }
        }
        "close" => {
            check_arg_count("std::db::close", 1, args, span)?;
            let handle = expect_integer(&args[0], "std::db::close", span)?;
            let existed = CONNS.with(|m| m.borrow_mut().remove(&handle).is_some());
            Ok(Value::Bool(existed))
        }
        other => Err(runtime_error(
            format!("no function 'std::db::{other}'"),
            span,
        )),
    }
}

/// Open a SQLite database file. Creates the file if it doesn't exist.
pub fn open(path: &str, span: &Span) -> Result<Connection, PipelineError> {
    Connection::open(path).map_err(|e| PipelineError::Runtime {
        message: format!("failed to open database '{path}': {e}"),
        line: span.line,
        column: span.column,
    })
}

/// Open an in-memory SQLite database.
pub fn open_in_memory(span: &Span) -> Result<Connection, PipelineError> {
    Connection::open_in_memory().map_err(|e| PipelineError::Runtime {
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
) -> Result<Value, PipelineError> {
    let sql_params = convert_params(params, span)?;
    let affected = conn
        .execute(sql, params_from_iter(sql_params.iter()))
        .map_err(|e| PipelineError::Runtime {
            message: format!("SQL execute error: {e}"),
            line: span.line,
            column: span.column,
        })?;
    Ok(Value::I64(affected as i64))
}

/// Execute a SQL query and return results as a Vec of HashMaps.
pub fn query(
    conn: &Connection,
    sql: &str,
    params: &[Value],
    span: &Span,
) -> Result<Value, PipelineError> {
    let sql_params = convert_params(params, span)?;
    let mut stmt = conn.prepare(sql).map_err(|e| PipelineError::Runtime {
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
                fields.insert(col_name.clone(), sql_value_to_oxy(val));
            }
            Ok(fields)
        })
        .map_err(|e| PipelineError::Runtime {
            message: format!("SQL query error: {e}"),
            line: span.line,
            column: span.column,
        })?;

    let mut result = Vec::new();
    for row in rows {
        let fields = row.map_err(|e| PipelineError::Runtime {
            message: format!("SQL row error: {e}"),
            line: span.line,
            column: span.column,
        })?;
        result.push(Value::HashMap(Rc::new(RefCell::new(
            fields
                .into_iter()
                .map(|(k, v)| (Value::String(k), v))
                .collect(),
        ))));
    }

    Ok(Value::Vec(Rc::new(RefCell::new(result))))
}

/// Get the last inserted row ID.
pub fn last_insert_id(conn: &Connection) -> Value {
    Value::I64(conn.last_insert_rowid())
}

/// Convert Oxy Values to rusqlite-compatible parameters.
fn convert_params(params: &[Value], span: &Span) -> Result<Vec<SqlValue>, PipelineError> {
    params
        .iter()
        .map(|v| match v {
            Value::I64(n) => Ok(SqlValue::Integer(*n)),
            Value::F64(f) => Ok(SqlValue::Real(*f)),
            Value::String(s) => Ok(SqlValue::Text(s.clone())),
            Value::Bool(b) => Ok(SqlValue::Integer(if *b { 1 } else { 0 })),
            Value::Unit => Ok(SqlValue::Null),
            _ => Err(PipelineError::Runtime {
                message: format!("cannot use {} as SQL parameter", v.type_name()),
                line: span.line,
                column: span.column,
            }),
        })
        .collect()
}

/// Convert a rusqlite Value to a Oxy Value.
fn sql_value_to_oxy(val: SqlValue) -> Value {
    match val {
        SqlValue::Null => Value::Unit,
        SqlValue::Integer(n) => Value::I64(n),
        SqlValue::Real(f) => Value::F64(f),
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
        assert_eq!(affected, Value::I64(1));
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
            &[Value::String("Alice".to_string()), Value::I64(30)],
            &span,
        )
        .unwrap();
        execute(
            &conn,
            "INSERT INTO users (name, age) VALUES (?1, ?2)",
            &[Value::String("Bob".to_string()), Value::I64(25)],
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
        if let Value::Vec(rc) = result {
            let rows = rc.borrow();
            assert_eq!(rows.len(), 2);
            if let Value::HashMap(row_rc) = &rows[0] {
                let row = row_rc.borrow();
                assert_eq!(
                    row.get(&Value::String("name".to_string())),
                    Some(&Value::String("Bob".to_string()))
                );
                assert_eq!(
                    row.get(&Value::String("age".to_string())),
                    Some(&Value::I64(25))
                );
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
            &[Value::F64(3.125)],
            &span,
        )
        .unwrap();

        let result = query(
            &conn,
            "SELECT value FROM items WHERE value > ?1",
            &[Value::F64(3.0)],
            &span,
        )
        .unwrap();
        if let Value::Vec(rc) = result {
            assert_eq!(rc.borrow().len(), 1);
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
        assert_eq!(id, Value::I64(1));
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
        if let Value::Vec(rc) = result {
            let rows = rc.borrow();
            if let Value::HashMap(row_rc) = &rows[0] {
                let row = row_rc.borrow();
                assert_eq!(row.get(&Value::String("v".to_string())), Some(&Value::Unit));
            }
        }
    }

    #[test]
    fn test_convert_params_invalid() {
        let span = test_span();
        let result = convert_params(&[Value::Vec(Rc::new(RefCell::new(vec![])))], &span);
        assert!(result.is_err());
    }
}
