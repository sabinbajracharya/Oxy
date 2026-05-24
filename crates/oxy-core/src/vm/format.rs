// vm/format.rs — Display/debug free functions for VM values and opcodes.
//
// These are standalone helpers (not impl Vm methods) used for tracing and
// disassembly. Extracted from vm/mod.rs to keep that file focused on the
// Vm struct and its execution loop.

use super::arith::{vm_add, vm_div, vm_mul, vm_rem, vm_sub};
use super::OpCode;
use crate::types::Value;

pub(super) fn trace_compact_val(v: &Value) -> String {
    match v {
        Value::Cell(rc) => format!("Cell({})", trace_compact_val(&rc.borrow())),
        Value::I64(n) => n.to_string(),
        Value::U8(n) => n.to_string(),
        Value::F64(n) => format!("{:.1}", n),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => format!("\"{:.20}\"", s),
        Value::Function(_) => "<fn>".into(),
        Value::Unit => "()".into(),
        _ => "?".into(),
    }
}

pub(super) fn trace_format_op(op: &OpCode) -> String {
    match op {
        OpCode::ConstInt(n, _) => format!("ConstInt({})", n),
        OpCode::LoadLocal(s) => format!("LoadLocal({})", s),
        OpCode::StoreLocal(s) => format!("StoreLocal({})", s),
        OpCode::Call { target, arg_count } => format!("Call({}, {})", target, arg_count),
        OpCode::Return => "Return".into(),
        OpCode::CallClosure { arg_count } => format!("CallClosure({})", arg_count),
        OpCode::Closure {
            target_ip,
            param_count,
            meta_idx,
        } => {
            format!(
                "Closure(ip={}, params={}, meta={})",
                target_ip, param_count, meta_idx
            )
        }
        OpCode::MakeCell(s) => format!("MakeCell({})", s),
        OpCode::Dup => "Dup".into(),
        OpCode::Pop => "Pop".into(),
        OpCode::Add => "Add".into(),
        OpCode::Sub => "Sub".into(),
        OpCode::Mul => "Mul".into(),
        OpCode::Div => "Div".into(),
        OpCode::Mod => "Mod".into(),
        OpCode::Eq => "Eq".into(),
        OpCode::Neq => "Neq".into(),
        OpCode::Lt => "Lt".into(),
        OpCode::Gt => "Gt".into(),
        OpCode::Le => "Le".into(),
        OpCode::Ge => "Ge".into(),
        OpCode::And => "And".into(),
        OpCode::Or => "Or".into(),
        OpCode::Neg => "Neg".into(),
        OpCode::Not => "Not".into(),
        OpCode::Jump(t) => format!("Jump({})", t),
        OpCode::JumpIfFalse(t) => format!("JumpIfFalse({})", t),
        OpCode::JumpIfTrue(t) => format!("JumpIfTrue({})", t),
        OpCode::ConstUnit => "ConstUnit".into(),
        OpCode::ConstBool(b) => format!("ConstBool({})", b),
        OpCode::ConstString(s) => format!("ConstString({:?})", s),
        OpCode::Print => "Print".into(),
        OpCode::PrintLn => "PrintLn".into(),
        OpCode::Halt => "Halt".into(),
        _ => format!("{:?}", op),
    }
}

/// Map a binary op function to the corresponding method name for trait dispatch.
pub(super) fn method_name_from_op(f: fn(Value, Value) -> Result<Value, String>) -> &'static str {
    if f as usize == vm_add as *const () as usize {
        return "add";
    }
    if f as usize == vm_sub as *const () as usize {
        return "sub";
    }
    if f as usize == vm_mul as *const () as usize {
        return "mul";
    }
    if f as usize == vm_div as *const () as usize {
        return "div";
    }
    if f as usize == vm_rem as *const () as usize {
        return "rem";
    }
    "add"
}

/// Debug format a value (like Rust's `{:?}`). Moved here from interpreter/format.rs.
pub(super) fn debug_format(val: &Value) -> String {
    match val {
        Value::String(s) => format!("\"{s}\""),
        Value::Char(c) => format!("'{c}'"),
        Value::Vec(rc) => {
            let items: Vec<String> = rc.borrow().iter().map(debug_format).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Tuple(t) => {
            let items: Vec<String> = t.iter().map(debug_format).collect();
            if t.len() == 1 {
                format!("({},)", items[0])
            } else {
                format!("({})", items.join(", "))
            }
        }
        Value::Struct { name, fields } => {
            let mut sorted: Vec<_> = fields.iter().collect();
            sorted.sort_by_key(|(k, _)| (*k).clone());
            let items: Vec<String> = sorted
                .iter()
                .map(|(k, v)| format!("{k}: {}", debug_format(v)))
                .collect();
            format!("{name} {{ {} }}", items.join(", "))
        }
        Value::EnumVariant {
            enum_name,
            variant,
            data,
        } => {
            let prefix = if enum_name == "Option" || enum_name == "Result" {
                String::new()
            } else {
                format!("{enum_name}::")
            };
            if data.is_empty() {
                format!("{prefix}{variant}")
            } else {
                let items: Vec<String> = data.iter().map(debug_format).collect();
                format!("{prefix}{variant}({})", items.join(", "))
            }
        }
        Value::HashMap(rc) => {
            let m = rc.borrow();
            let mut sorted: Vec<_> = m.iter().collect();
            sorted.sort_by_key(|(k, _)| (*k).clone());
            let items: Vec<String> = sorted
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}: {}",
                        debug_format(&Value::String(k.to_string())),
                        debug_format(v)
                    )
                })
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        Value::BTreeMap(rc) => {
            let m = rc.borrow();
            let items: Vec<String> = m
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}: {}",
                        debug_format(&Value::String(k.to_string())),
                        debug_format(v)
                    )
                })
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        Value::BTreeSet(rc) => {
            let items: Vec<String> = rc.borrow().iter().map(debug_format).collect();
            format!("{{{}}}", items.join(", "))
        }
        Value::BinaryHeap(rc) => {
            let sorted = rc.borrow().clone().into_sorted_vec();
            let items: Vec<String> = sorted.iter().map(debug_format).collect();
            format!("BinaryHeap([{}])", items.join(", "))
        }
        Value::VecDeque(rc) => {
            let items: Vec<String> = rc.borrow().iter().map(debug_format).collect();
            format!("VecDeque([{}])", items.join(", "))
        }
        Value::Future(f) => format!("Future<{}>", f.name),
        Value::JoinHandle(v) => format!("JoinHandle({})", debug_format(v)),
        Value::Cell(rc) => debug_format(&rc.borrow()),
        other => format!("{other}"),
    }
}
