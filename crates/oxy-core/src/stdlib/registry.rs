//! Single source of truth for built-in path dispatch.
//!
//! The register-IR backends lower every `Type::method(...)` / `module::fn(...)`
//! path call to the shared `oxy_call_path` FFI (`jit/ffi/mod.rs`), which
//! resolves it against this registry at runtime — exact item handlers first,
//! then user-defined functions, then module dispatch. There is no separate
//! compile-time whitelist: an unrecognised path simply fails to resolve at
//! runtime. Adding a new built-in needs ONE registration here plus its
//! implementation.
//!
//! # Two kinds of entries
//!
//! - **Module** (`crate::stdlib::math::call` and friends): `name::any_fn(args)`
//!   and `std::name::any_fn(args)` both route to `call("any_fn", args)`.
//!   Any function name passes the compiler's whitelist; the module's `call`
//!   function returns an error at runtime if it doesn't recognise the name.
//!
//! - **Item**: a full path like `["Map", "new"]` or
//!   `["std", "regex", "Regex", "new"]` dispatches to one specific handler.
//!   Used for constructors and one-off built-ins that don't fit the module
//!   shape.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
use std::rc::Rc;

use crate::errors::PipelineError;
use crate::lexer::Span;
use crate::types::Value;

/// Callback used by stdlib modules to invoke a user-supplied closure (for
/// route handlers, async continuations, etc.). Module implementations that
/// don't need to call back into user code can ignore this parameter.
pub type ClosureInvoker<'a> = &'a mut dyn FnMut(&Value, &[Value]) -> Result<Value, String>;

pub type ModuleCall =
    for<'a> fn(&str, &[Value], &Span, ClosureInvoker<'a>) -> Result<Value, PipelineError>;
pub type ItemHandler = fn(&[Value]) -> Result<Value, String>;
/// Resolves a module-level constant (e.g. `math::PI`) by name. Modules that
/// expose no constants register [`no_constants`].
pub type ConstantLookup = fn(&str) -> Option<Value>;

pub struct Module {
    pub name: &'static str,
    pub call: ModuleCall,
    /// Module-level constant resolver — keeps `module::CONST` table-driven
    /// instead of special-casing module names inside [`lookup_constant`].
    pub constants: ConstantLookup,
}

pub struct Item {
    pub path: &'static [&'static str],
    pub handler: ItemHandler,
}

/// Default `constants` resolver for modules that expose no constants.
fn no_constants(_name: &str) -> Option<Value> {
    None
}

/// Look up a module by name. Returns `Some(call)` if `name` is a registered
/// stdlib module.
pub fn lookup_module(name: &str) -> Option<ModuleCall> {
    MODULES.iter().find(|m| m.name == name).map(|m| m.call)
}

/// Look up a module-level constant such as `math::PI`. Returns the value if the
/// named module exposes a constant of that name.
pub fn lookup_constant(module: &str, name: &str) -> Option<Value> {
    MODULES
        .iter()
        .find(|m| m.name == module)
        .and_then(|m| (m.constants)(name))
}

/// Look up an item by path.
///
/// Tries an exact match first. On a miss, retries against the trailing
/// `Type::method` segments: a `use`-resolved path such as
/// `std::collections::HashMap::new` canonicalizes to the registered short form
/// `HashMap::new`. The std-module prefix is just the import path — the
/// `[TypeName, method]` tail is what identifies the associated function.
///
/// The fallback only fires for paths longer than two segments, so it never
/// shadows a bare item, and the registered 2-segment items are all reserved
/// CamelCase builtin type names (`HashMap`, `String`, `Regex`, …) that a user
/// cannot redefine — so the tail match can't collide with a user module.
pub fn lookup_item(path: &[&str]) -> Option<ItemHandler> {
    if let Some(handler) = ITEMS.iter().find(|i| i.path == path).map(|i| i.handler) {
        return Some(handler);
    }
    // Flatten each segment on `::` — a `use`-resolved segment can itself be a
    // qualified name (e.g. `"std::collections::Map"`) — then retry against
    // the trailing `Type::method` pair.
    let flat: Vec<&str> = path.iter().flat_map(|s| s.split("::")).collect();
    if flat.len() > 2 {
        let tail = &flat[flat.len() - 2..];
        return ITEMS.iter().find(|i| i.path == tail).map(|i| i.handler);
    }
    None
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

static MODULES: &[Module] = &[
    Module {
        name: "math",
        call: crate::stdlib::math::call,
        constants: crate::stdlib::math::constant,
    },
    Module {
        name: "fs",
        call: crate::stdlib::fs::call,
        constants: no_constants,
    },
    Module {
        name: "io",
        call: crate::stdlib::io::call,
        constants: no_constants,
    },
    #[cfg(feature = "db")]
    Module {
        name: "db",
        call: crate::stdlib::db::call,
        constants: no_constants,
    },
    Module {
        name: "env",
        call: crate::stdlib::env::call,
        constants: no_constants,
    },
    Module {
        name: "args",
        call: crate::stdlib::args::call,
        constants: no_constants,
    },
    Module {
        name: "path",
        call: crate::stdlib::path::call,
        constants: no_constants,
    },
    Module {
        name: "process",
        call: crate::stdlib::process::call,
        constants: no_constants,
    },
    Module {
        name: "regex",
        call: crate::stdlib::regex::call,
        constants: no_constants,
    },
    Module {
        name: "net",
        call: crate::stdlib::net::call,
        constants: no_constants,
    },
    Module {
        name: "time",
        call: crate::stdlib::time::call,
        constants: no_constants,
    },
    Module {
        name: "rand",
        call: crate::stdlib::rand::call,
        constants: no_constants,
    },
    Module {
        name: "json",
        call: crate::stdlib::json::call,
        constants: no_constants,
    },
    Module {
        name: "http",
        call: crate::stdlib::http::call,
        constants: no_constants,
    },
    #[cfg(feature = "server")]
    Module {
        name: "server",
        call: crate::stdlib::server::call,
        constants: no_constants,
    },
];

static ITEMS: &[Item] = &[
    Item {
        path: &["String", "from"],
        handler: string_from,
    },
    Item {
        path: &["Map", "new"],
        handler: hashmap_new,
    },
    Item {
        path: &["Set", "new"],
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
        path: &["Int", "parse"],
        handler: int_parse,
    },
    Item {
        path: &["Float", "parse"],
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
        path: &["assert_eq"],
        handler: assert_eq_handler,
    },
    Item {
        path: &["assert_ne"],
        handler: assert_ne_handler,
    },
    Item {
        path: &["assert"],
        handler: assert_handler,
    },
    Item {
        path: &["io", "println"],
        handler: io_println_handler,
    },
    Item {
        path: &["io", "print"],
        handler: io_print_handler,
    },
    Item {
        path: &["io", "dbg"],
        handler: io_dbg_handler,
    },
    Item {
        path: &["string", "format"],
        handler: string_format_handler,
    },
    Item {
        path: &["panic"],
        handler: panic_handler,
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
    let value = args.first().cloned().unwrap_or(Value::Unit);
    let mut fields = HashMap::new();
    fields.insert("value".to_string(), value);
    fields.insert("next".to_string(), Value::none());
    Ok(Value::Struct {
        name: "ListNode".to_string(),
        fields,
    })
}

fn treenode_new(args: &[Value]) -> Result<Value, String> {
    let value = args.first().cloned().unwrap_or(Value::Unit);
    let mut fields = HashMap::new();
    fields.insert("value".to_string(), value);
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
            "cannot parse \"{s}\" as Float"
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

fn assert_eq_handler(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!("assert_eq! takes 2 arguments, got {}", args.len()));
    }
    if args[0] != args[1] {
        return Err(format!(
            "assertion failed: `(left == right)`\n  left: `{:?}`\n right: `{:?}`",
            args[0], args[1]
        ));
    }
    Ok(Value::Unit)
}

fn assert_ne_handler(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!("assert_ne! takes 2 arguments, got {}", args.len()));
    }
    if args[0] == args[1] {
        return Err(format!(
            "assertion failed: `(left != right)`\n  left: `{:?}`\n right: `{:?}`",
            args[0], args[1]
        ));
    }
    Ok(Value::Unit)
}

fn assert_handler(args: &[Value]) -> Result<Value, String> {
    let cond = args.first().cloned().unwrap_or(Value::Unit);
    if !cond.is_truthy() {
        // `assert!(cond, "msg", fmt_args...)` — the optional message is a
        // format template (matching Rust). Reuse the same template engine as
        // `format!`/`println!` so `{}`/`{:?}` behave identically.
        if args.len() >= 2 {
            let template = args[1].to_string();
            return Err(crate::types::format_template(&template, &args[2..]));
        }
        return Err(format!("assertion failed: `{:?}` is not truthy", cond));
    }
    Ok(Value::Unit)
}

fn panic_handler(args: &[Value]) -> Result<Value, String> {
    let msg = args.first().map(|v| v.to_string()).unwrap_or_default();
    Err(msg)
}

fn io_println_handler(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        println!();
    } else {
        let template = args[0].to_string();
        let rendered = render_template(&template, &args[1..]);
        println!("{}", rendered);
    }
    Ok(Value::Unit)
}

fn io_print_handler(args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        let template = args[0].to_string();
        let rendered = render_template(&template, &args[1..]);
        print!("{}", rendered);
    }
    Ok(Value::Unit)
}

fn io_dbg_handler(args: &[Value]) -> Result<Value, String> {
    for (i, val) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{:?}", val);
    }
    println!();
    Ok(Value::Unit)
}

fn string_format_handler(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(Value::String(String::new()));
    }
    let template = args[0].to_string();
    let rendered = render_template(&template, &args[1..]);
    Ok(Value::String(rendered))
}

/// Simple template rendering: replaces {} placeholders with args.
fn render_template(template: &str, args: &[Value]) -> String {
    let mut result = String::new();
    let mut remaining = template;
    let mut arg_idx = 0;
    while let Some(pos) = remaining.find("{}") {
        result.push_str(&remaining[..pos]);
        if arg_idx < args.len() {
            result.push_str(&args[arg_idx].to_string());
            arg_idx += 1;
        } else {
            result.push_str("{}");
        }
        remaining = &remaining[pos + 2..];
    }
    result.push_str(remaining);
    result
}
