//! Single source of truth for built-in path dispatch.
//!
//! Both `compiler::helpers::is_builtin_path` (the compile-time whitelist
//! deciding when to emit `PathCallBuiltin`) and `vm::Vm::dispatch_pathcall`
//! (the runtime handler) read from this registry. Adding a new built-in
//! now needs ONE registration here plus its implementation — the
//! compiler whitelist stays in sync automatically.
//!
//! # Two kinds of entries
//!
//! - **Module** (`crate::stdlib::math::call` and friends): `name::any_fn(args)`
//!   and `std::name::any_fn(args)` both route to `call("any_fn", args)`.
//!   Any function name passes the compiler's whitelist; the module's `call`
//!   function returns an error at runtime if it doesn't recognise the name.
//!
//! - **Item**: a full path like `["HashMap", "new"]` or
//!   `["std", "regex", "Regex", "new"]` dispatches to one specific handler.
//!   Used for constructors and one-off built-ins that don't fit the module
//!   shape.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
use std::rc::Rc;

use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

pub type ModuleCall = fn(&str, &[Value], &Span) -> Result<Value, FerriError>;
pub type ItemHandler = fn(&[Value]) -> Result<Value, String>;

pub struct Module {
    pub name: &'static str,
    pub call: ModuleCall,
}

pub struct Item {
    pub path: &'static [&'static str],
    pub handler: ItemHandler,
}

pub fn modules() -> &'static [Module] {
    MODULES
}

pub fn items() -> &'static [Item] {
    ITEMS
}

/// Look up a module by name. Returns `Some(call)` if `name` is a registered
/// stdlib module.
pub fn lookup_module(name: &str) -> Option<ModuleCall> {
    MODULES.iter().find(|m| m.name == name).map(|m| m.call)
}

/// Look up an item by exact path.
pub fn lookup_item(path: &[&str]) -> Option<ItemHandler> {
    ITEMS.iter().find(|i| i.path == path).map(|i| i.handler)
}

/// True iff `path` is a built-in: either `[module, _]` / `[std, module, _]`
/// against a registered module, or an exact match for a registered item.
pub fn is_builtin(path: &[&str]) -> bool {
    match path {
        [m, _] | ["std", m, _] => {
            if lookup_module(m).is_some() {
                return true;
            }
        }
        _ => {}
    }
    lookup_item(path).is_some()
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

static MODULES: &[Module] = &[
    Module {
        name: "math",
        call: crate::stdlib::math::call,
    },
    Module {
        name: "fs",
        call: crate::stdlib::fs::call,
    },
    Module {
        name: "io",
        call: crate::stdlib::io::call,
    },
    #[cfg(feature = "db")]
    Module {
        name: "db",
        call: crate::stdlib::db::call,
    },
    Module {
        name: "env",
        call: crate::stdlib::env::call,
    },
    Module {
        name: "process",
        call: crate::stdlib::process::call,
    },
    Module {
        name: "regex",
        call: crate::stdlib::regex::call,
    },
    Module {
        name: "net",
        call: crate::stdlib::net::call,
    },
    Module {
        name: "time",
        call: crate::stdlib::time::call,
    },
    Module {
        name: "rand",
        call: crate::stdlib::rand::call,
    },
    Module {
        name: "json",
        call: crate::stdlib::json::call,
    },
    Module {
        name: "http",
        call: crate::stdlib::http::call,
    },
];

static ITEMS: &[Item] = &[
    Item {
        path: &["String", "from"],
        handler: string_from,
    },
    Item {
        path: &["HashMap", "new"],
        handler: hashmap_new,
    },
    Item {
        path: &["HashSet", "new"],
        handler: hashset_new,
    },
    Item {
        path: &["BTreeMap", "new"],
        handler: btreemap_new,
    },
    Item {
        path: &["BTreeSet", "new"],
        handler: btreeset_new,
    },
    Item {
        path: &["BinaryHeap", "new"],
        handler: binaryheap_new,
    },
    Item {
        path: &["VecDeque", "new"],
        handler: vecdeque_new,
    },
    Item {
        path: &["ListNode", "new"],
        handler: listnode_new,
    },
    Item {
        path: &["TreeNode", "new"],
        handler: treenode_new,
    },
    Item {
        path: &["int", "parse"],
        handler: int_parse,
    },
    Item {
        path: &["float", "parse"],
        handler: float_parse,
    },
    Item {
        path: &["char", "from_code"],
        handler: char_from_code,
    },
    Item {
        path: &["Regex", "new"],
        handler: regex_new,
    },
    Item {
        path: &["std", "regex", "Regex", "new"],
        handler: regex_new,
    },
    Item {
        path: &["std", "env", "args"],
        handler: std_env_args,
    },
];

// ---------------------------------------------------------------------------
// Item handlers
// ---------------------------------------------------------------------------

fn string_from(args: &[Value]) -> Result<Value, String> {
    let s = args.first().map(|v| format!("{}", v)).unwrap_or_default();
    Ok(Value::String(s))
}

fn hashmap_new(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::HashMap(Rc::new(RefCell::new(HashMap::new()))))
}

fn hashset_new(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::HashSet(Rc::new(RefCell::new(HashSet::new()))))
}

fn btreemap_new(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::BTreeMap(Rc::new(RefCell::new(BTreeMap::new()))))
}

fn btreeset_new(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::BTreeSet(Rc::new(RefCell::new(BTreeSet::new()))))
}

fn binaryheap_new(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::BinaryHeap(Rc::new(RefCell::new(BinaryHeap::new()))))
}

fn vecdeque_new(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::VecDeque(Rc::new(RefCell::new(VecDeque::new()))))
}

fn listnode_new(args: &[Value]) -> Result<Value, String> {
    let val = args.first().cloned().unwrap_or(Value::Unit);
    let mut fields = HashMap::new();
    fields.insert("val".to_string(), val);
    fields.insert("next".to_string(), Value::none());
    Ok(Value::Struct {
        name: "ListNode".to_string(),
        fields,
    })
}

fn treenode_new(args: &[Value]) -> Result<Value, String> {
    let val = args.first().cloned().unwrap_or(Value::Unit);
    let mut fields = HashMap::new();
    fields.insert("val".to_string(), val);
    fields.insert("left".to_string(), Value::none());
    fields.insert("right".to_string(), Value::none());
    Ok(Value::Struct {
        name: "TreeNode".to_string(),
        fields,
    })
}

fn int_parse(args: &[Value]) -> Result<Value, String> {
    let s = args.first().map(|v| v.to_string()).unwrap_or_default();
    let trimmed = s.trim();
    let result = if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        i64::from_str_radix(&trimmed[2..], 16).map_err(|_| ())
    } else {
        trimmed.parse::<i64>().map_err(|_| ())
    };
    match result {
        Ok(n) => Ok(Value::ok(Value::I64(n))),
        Err(_) => Ok(Value::err(Value::String(format!(
            "cannot parse \"{s}\" as integer"
        )))),
    }
}

fn float_parse(args: &[Value]) -> Result<Value, String> {
    let s = args.first().map(|v| v.to_string()).unwrap_or_default();
    match s.trim().parse::<f64>() {
        Ok(n) => Ok(Value::ok(Value::F64(n))),
        Err(_) => Ok(Value::err(Value::String(format!(
            "cannot parse \"{s}\" as float"
        )))),
    }
}

fn char_from_code(args: &[Value]) -> Result<Value, String> {
    let n = args
        .first()
        .and_then(|v| match v {
            Value::I64(n) => Some(*n as u32),
            _ => None,
        })
        .unwrap_or(0);
    match char::from_u32(n) {
        Some(c) => Ok(Value::Char(c)),
        None => Err(format!("char::from_code: invalid code point {n}")),
    }
}

fn regex_new(args: &[Value]) -> Result<Value, String> {
    let pattern = args
        .first()
        .map(|v| match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .unwrap_or_default();
    if let Err(e) = regex::Regex::new(&pattern) {
        return Err(format!("Regex::new: invalid pattern: {}", e));
    }
    let mut fields: HashMap<String, Value> = HashMap::new();
    fields.insert("pattern".to_string(), Value::String(pattern));
    Ok(Value::Struct {
        name: "Regex".to_string(),
        fields,
    })
}

fn std_env_args(_args: &[Value]) -> Result<Value, String> {
    // Test/REPL stub — return an empty argv.
    Ok(Value::Vec(Rc::new(RefCell::new(Vec::new()))))
}
