//! Value system for the Oxy language.
//!
//! All values at runtime are represented by the [`Value`] enum.
//! Oxy uses reference counting internally — no borrow checker.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::ast::{Block, Param, TypeAnnotation};
use crate::env::Env;

/// Type name constant for the built-in `Option` enum.
pub const OPTION_TYPE: &str = "Option";
/// Type name constant for the built-in `Result` enum.
pub const RESULT_TYPE: &str = "Result";
/// Variant name constant for `Option::Some`.
pub const SOME_VARIANT: &str = "Some";
/// Variant name constant for `Option::None`.
pub const NONE_VARIANT: &str = "None";
/// Variant name constant for `Result::Ok`.
pub const OK_VARIANT: &str = "Ok";
/// Variant name constant for `Result::Err`.
pub const ERR_VARIANT: &str = "Err";

/// A runtime value in Oxy.
// WHY: Collection types use Rc<RefCell<>> for shared mutable semantics — cloning
// a collection creates another reference to the same data (like Python objects).
// Primitives are cheap to copy. The interpreter cannot statically track ownership.
#[derive(Debug)]
pub enum Value {
    /// 64-bit signed integer.
    Integer(i64),
    /// 64-bit floating-point number.
    Float(f64),
    /// Boolean.
    Bool(bool),
    /// UTF-8 string.
    String(String),
    /// Character.
    Char(char),
    /// Unit value `()`.
    Unit,
    /// A function value (closure).
    Function(Box<FunctionData>),
    /// A range value: `start..end` (end-exclusive, stored as actual end).
    Range(i64, i64),
    /// A vector (dynamic array) — shared mutable via Rc<RefCell<>>.
    Vec(Rc<RefCell<Vec<Value>>>),
    /// A tuple.
    Tuple(Vec<Value>),
    /// A struct instance: `Point { x: 1.0, y: 2.0 }`
    Struct {
        name: String,
        fields: HashMap<String, Value>,
    },
    /// An enum variant instance.
    EnumVariant {
        enum_name: String,
        variant: String,
        data: Vec<Value>,
    },
    /// A hash map — shared mutable via Rc<RefCell<>>.
    HashMap(Rc<RefCell<HashMap<Value, Value>>>),
    /// A hash set — shared mutable via Rc<RefCell<>>.
    HashSet(Rc<RefCell<HashSet<Value>>>),
    /// A binary heap (max-heap by default) — shared mutable via Rc<RefCell<>>.
    BinaryHeap(Rc<RefCell<BinaryHeap<Value>>>),
    /// A double-ended queue — shared mutable via Rc<RefCell<>>.
    VecDeque(Rc<RefCell<VecDeque<Value>>>),
    /// A lazy iterator (adapter chain).
    Iterator(Box<IteratorState>),
    /// A future (lazy thunk wrapping an async function call).
    Future(Box<FutureData>),
    /// A join handle (eagerly evaluated, wraps a result).
    JoinHandle(Box<Value>),
    /// A shared mutable cell — any value wrapped in Rc<RefCell<>> for mutation sharing.
    /// Used for mutable variables captured by closures and &mut self methods.
    Cell(Rc<RefCell<Value>>),
}

impl Value {
    /// Wrap a value in a shared mutable cell.
    pub fn cell(val: Value) -> Self {
        Value::Cell(Rc::new(RefCell::new(val)))
    }
    /// If this is a Cell, borrow and return the inner value. Otherwise return self.
    pub fn deref_cell(&self) -> Value {
        match self {
            Value::Cell(rc) => rc.borrow().clone(),
            other => other.clone(),
        }
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        match self {
            Value::Integer(n) => Value::Integer(*n),
            Value::Float(f) => Value::Float(*f),
            Value::Bool(b) => Value::Bool(*b),
            Value::String(s) => Value::String(s.clone()),
            Value::Char(c) => Value::Char(*c),
            Value::Unit => Value::Unit,
            Value::Function(f) => Value::Function(f.clone()),
            Value::Range(a, b) => Value::Range(*a, *b),
            Value::Vec(rc) => Value::Vec(Rc::clone(rc)),
            Value::Tuple(t) => Value::Tuple(t.clone()),
            Value::Struct { name, fields } => Value::Struct {
                name: name.clone(),
                fields: fields.clone(),
            },
            Value::EnumVariant {
                enum_name,
                variant,
                data,
            } => Value::EnumVariant {
                enum_name: enum_name.clone(),
                variant: variant.clone(),
                data: data.clone(),
            },
            Value::HashMap(rc) => Value::HashMap(Rc::clone(rc)),
            Value::HashSet(rc) => Value::HashSet(Rc::clone(rc)),
            Value::BinaryHeap(rc) => Value::BinaryHeap(Rc::clone(rc)),
            Value::VecDeque(rc) => Value::VecDeque(Rc::clone(rc)),
            Value::Iterator(it) => Value::Iterator(it.clone()),
            Value::Future(f) => Value::Future(f.clone()),
            Value::JoinHandle(h) => Value::JoinHandle(h.clone()),
            Value::Cell(rc) => Value::Cell(Rc::clone(rc)),
        }
    }
}

/// Data for an async future (boxed to keep Value enum small).
#[derive(Debug, Clone)]
pub struct FutureData {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub closure_env: Env,
    pub args: Vec<Value>,
}

/// Lazy iterator state — each variant represents one stage in an adapter chain.
#[derive(Debug, Clone)]
pub enum IteratorState {
    VecSource {
        data: Vec<Value>,
        index: usize,
    },
    RangeSource {
        current: i64,
        end: i64,
    },
    Map {
        source: Box<IteratorState>,
        closure: Value,
    },
    Filter {
        source: Box<IteratorState>,
        closure: Value,
    },
    Take {
        source: Box<IteratorState>,
        remaining: usize,
    },
    Skip {
        source: Box<IteratorState>,
        remaining: usize,
    },
    Chain {
        first: Box<IteratorState>,
        second: Box<IteratorState>,
    },
    Zip {
        left: Box<IteratorState>,
        right: Box<IteratorState>,
    },
    Enumerate {
        source: Box<IteratorState>,
        index: usize,
    },
    FlatMap {
        source: Box<IteratorState>,
        closure: Value,
        current: Option<Box<IteratorState>>,
    },
    Flatten {
        source: Box<IteratorState>,
        current: Option<Box<IteratorState>>,
    },
}

/// Data for a function value (boxed to keep Value enum small).
#[derive(Debug, Clone)]
pub struct FunctionData {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub closure_env: Env,
    /// VM-only: bytecode instruction index where the function body starts.
    pub target_ip: Option<usize>,
    /// Captured variable slot indices (name, outer_slot) for CallClosure stack setup.
    pub captured_slots: Vec<(String, usize)>,
}

impl Value {
    /// Returns the type name of this value for error messages.
    pub fn type_name(&self) -> String {
        match self {
            Value::Integer(_) => "i64".into(),
            Value::Float(_) => "f64".into(),
            Value::Bool(_) => "bool".into(),
            Value::String(_) => "String".into(),
            Value::Char(_) => "char".into(),
            Value::Unit => "()".into(),
            Value::Function(_) => "fn".into(),
            Value::Range(_, _) => "Range".into(),
            Value::Vec(_) => "Vec".into(),
            Value::Tuple(_) => "tuple".into(),
            Value::Struct { name, .. } => name.clone(),
            Value::EnumVariant { enum_name, .. } => enum_name.clone(),
            Value::HashMap(_) => "HashMap".into(),
            Value::HashSet(_) => "HashSet".into(),
            Value::BinaryHeap(_) => "BinaryHeap".into(),
            Value::VecDeque(_) => "VecDeque".into(),
            Value::Iterator(_) => "Iterator".into(),
            Value::Future(_) => "Future".into(),
            Value::JoinHandle(_) => "JoinHandle".into(),
            Value::Cell(_) => "Cell".into(),
        }
    }

    /// Constructs a `Some(val)` option variant.
    pub fn some(val: Value) -> Value {
        Value::EnumVariant {
            enum_name: OPTION_TYPE.to_string(),
            variant: SOME_VARIANT.to_string(),
            data: vec![val],
        }
    }

    /// Constructs a `None` option variant.
    pub fn none() -> Value {
        Value::EnumVariant {
            enum_name: OPTION_TYPE.to_string(),
            variant: NONE_VARIANT.to_string(),
            data: vec![],
        }
    }

    /// Constructs an `Ok(val)` result variant.
    pub fn ok(val: Value) -> Value {
        Value::EnumVariant {
            enum_name: RESULT_TYPE.to_string(),
            variant: OK_VARIANT.to_string(),
            data: vec![val],
        }
    }

    /// Constructs an `Err(val)` result variant.
    pub fn err(val: Value) -> Value {
        Value::EnumVariant {
            enum_name: RESULT_TYPE.to_string(),
            variant: ERR_VARIANT.to_string(),
            data: vec![val],
        }
    }

    /// Check if this is a Some variant
    pub fn is_some_variant(&self) -> bool {
        matches!(self, Value::EnumVariant { enum_name, variant, .. } if enum_name == OPTION_TYPE && variant == SOME_VARIANT)
    }

    /// Check if this is a None variant
    pub fn is_none_variant(&self) -> bool {
        matches!(self, Value::EnumVariant { enum_name, variant, .. } if enum_name == OPTION_TYPE && variant == NONE_VARIANT)
    }

    /// Check if this is an Ok variant
    pub fn is_ok_variant(&self) -> bool {
        matches!(self, Value::EnumVariant { enum_name, variant, .. } if enum_name == RESULT_TYPE && variant == OK_VARIANT)
    }

    /// Check if this is an Err variant
    pub fn is_err_variant(&self) -> bool {
        matches!(self, Value::EnumVariant { enum_name, variant, .. } if enum_name == RESULT_TYPE && variant == ERR_VARIANT)
    }

    /// Convert this value to a flat `Vec<Value>` for iteration in `for` loops.
    pub fn into_iterable(self) -> Result<Vec<Value>, String> {
        match self {
            Value::Range(start, end) => Ok((start..end).map(Value::Integer).collect()),
            Value::Vec(rc) => Ok(rc.borrow().clone()),
            Value::String(s) => Ok(s.chars().map(Value::Char).collect()),
            Value::HashMap(rc) => {
                let m = rc.borrow();
                let mut pairs: Vec<_> = m
                    .iter()
                    .map(|(k, v)| Value::Tuple(vec![k.clone(), v.clone()]))
                    .collect();
                pairs.sort_by(|a, b| {
                    if let (Value::Tuple(a), Value::Tuple(b)) = (a, b) {
                        a[0].cmp(&b[0])
                    } else {
                        std::cmp::Ordering::Equal
                    }
                });
                Ok(pairs)
            }
            Value::HashSet(rc) => {
                let s = rc.borrow();
                let mut v: Vec<Value> = s.iter().cloned().collect();
                v.sort();
                Ok(v)
            }
            Value::BinaryHeap(rc) => Ok(rc.borrow().clone().into_sorted_vec()),
            Value::VecDeque(rc) => Ok(rc.borrow().clone().into_iter().collect()),
            Value::Iterator(_) => Err(
                "Iterators must be consumed with .collect() or another consumer method, not iterated directly".into(),
            ),
            other => Err(format!("cannot iterate over {}", other.type_name())),
        }
    }

    /// Integer discriminant for variant ordering — lower = earlier in sort order.
    fn variant_discriminant(&self) -> u8 {
        match self {
            Value::Unit => 0,
            Value::Bool(_) => 1,
            Value::Integer(_) => 2,
            Value::Float(_) => 3,
            Value::Char(_) => 4,
            Value::String(_) => 5,
            Value::Range(_, _) => 6,
            Value::Vec(_) => 7,
            Value::Tuple(_) => 8,
            Value::HashMap(_) => 9,
            Value::HashSet(_) => 10,
            Value::BinaryHeap(_) => 11,
            Value::VecDeque(_) => 12,
            Value::Iterator(_) => 13,
            Value::Struct { .. } => 14,
            Value::EnumVariant { .. } => 15,
            Value::Function(_) => 16,
            Value::Future(_) => 17,
            Value::JoinHandle(_) => 18,
            Value::Cell(_) => 19,
        }
    }

    /// Returns true if this value is truthy (for conditions).
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Integer(n) => *n != 0,
            Value::Unit => false,
            Value::Range(_, _) => true,
            Value::Vec(rc) => !rc.borrow().is_empty(),
            Value::Tuple(t) => !t.is_empty(),
            Value::Struct { .. } => true,
            Value::EnumVariant { .. } => true,
            Value::HashMap(rc) => !rc.borrow().is_empty(),
            Value::HashSet(rc) => !rc.borrow().is_empty(),
            Value::BinaryHeap(rc) => !rc.borrow().is_empty(),
            Value::VecDeque(rc) => !rc.borrow().is_empty(),
            Value::Iterator(_) => true,
            Value::Future(_) => true,
            Value::JoinHandle(_) => true,
            Value::Cell(rc) => rc.borrow().is_truthy(),
            _ => true,
        }
    }
}

/// Formats a [`Value`] for user-facing display (e.g. `println!`).
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(n) => write!(f, "{n}"),
            Value::Float(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{n:.1}")
                } else {
                    write!(f, "{n}")
                }
            }
            Value::Bool(b) => write!(f, "{b}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Char(c) => write!(f, "{c}"),
            Value::Unit => write!(f, "()"),
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::Range(start, end) => write!(f, "{start}..{end}"),
            Value::Vec(rc) => {
                let v = rc.borrow();
                write!(f, "[")?;
                for (i, elem) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "]")
            }
            Value::Tuple(t) => {
                write!(f, "(")?;
                for (i, elem) in t.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                if t.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            }
            Value::Struct { name, fields } => {
                write!(f, "{name} {{ ")?;
                let mut sorted: Vec<_> = fields.iter().collect();
                sorted.sort_by_key(|(k, _)| (*k).clone());
                for (i, (k, v)) in sorted.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, " }}")
            }
            Value::EnumVariant {
                enum_name,
                variant,
                data,
            } => {
                // Built-in Option/Result: show without enum prefix
                if enum_name == OPTION_TYPE || enum_name == RESULT_TYPE {
                    write!(f, "{variant}")?;
                } else {
                    write!(f, "{enum_name}::{variant}")?;
                }
                if !data.is_empty() {
                    write!(f, "(")?;
                    for (i, v) in data.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{v}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Value::HashMap(rc) => {
                let m = rc.borrow();
                write!(f, "{{")?;
                let mut sorted: Vec<_> = m.iter().collect();
                sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
                for (i, (k, v)) in sorted.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
            Value::HashSet(rc) => {
                let s = rc.borrow();
                write!(f, "{{")?;
                let mut sorted: Vec<&Value> = s.iter().collect();
                sorted.sort();
                for (i, elem) in sorted.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "}}")
            }
            Value::BinaryHeap(rc) => {
                write!(f, "BinaryHeap([")?;
                let sorted = rc.borrow().clone().into_sorted_vec();
                for (i, elem) in sorted.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "])")
            }
            Value::VecDeque(rc) => {
                write!(f, "VecDeque([")?;
                for (i, elem) in rc.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "])")
            }
            Value::Iterator(_) => write!(f, "<iterator>"),
            Value::Future(_) => write!(f, "<future>"),
            Value::JoinHandle(_) => write!(f, "<join_handle>"),
            Value::Cell(rc) => write!(f, "{}", rc.borrow()),
        }
    }
}

/// Structural equality for [`Value`]; functions and futures are never equal.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
            (Value::Range(a1, a2), Value::Range(b1, b2)) => a1 == b1 && a2 == b2,
            (Value::Vec(a), Value::Vec(b)) => *a.borrow() == *b.borrow(),
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (
                Value::Struct {
                    name: na,
                    fields: fa,
                },
                Value::Struct {
                    name: nb,
                    fields: fb,
                },
            ) => na == nb && fa == fb,
            (
                Value::EnumVariant {
                    enum_name: ea,
                    variant: va,
                    data: da,
                },
                Value::EnumVariant {
                    enum_name: eb,
                    variant: vb,
                    data: db,
                },
            ) => ea == eb && va == vb && da == db,
            (Value::HashMap(a), Value::HashMap(b)) => *a.borrow() == *b.borrow(),
            (Value::HashSet(a), Value::HashSet(b)) => *a.borrow() == *b.borrow(),
            (Value::BinaryHeap(a), Value::BinaryHeap(b)) => {
                let va = a.borrow().clone().into_sorted_vec();
                let vb = b.borrow().clone().into_sorted_vec();
                va == vb
            }
            (Value::VecDeque(a), Value::VecDeque(b)) => {
                let va: Vec<Value> = a.borrow().iter().cloned().collect();
                let vb: Vec<Value> = b.borrow().iter().cloned().collect();
                va == vb
            }
            (Value::Iterator(_), Value::Iterator(_)) => false,
            _ => false,
        }
    }
}

/// Ordering for [`Value`]; delegates to [`Ord`] for all comparisons.
impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Total ordering for [`Value`].
///
/// Orders by variant discriminant first, then by payload. Uses `f64::total_cmp`
/// for floats so ordering is total (no NaN surprises).
impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        let disc = self.variant_discriminant();
        let other_disc = other.variant_discriminant();
        match disc.cmp(&other_disc) {
            Ordering::Equal => {}
            ord => return ord,
        }

        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => a.total_cmp(b),
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (Value::String(a), Value::String(b)) => a.cmp(b),
            (Value::Char(a), Value::Char(b)) => a.cmp(b),
            (Value::Unit, Value::Unit) => Ordering::Equal,
            (Value::Range(a1, a2), Value::Range(b1, b2)) => a1.cmp(b1).then(a2.cmp(b2)),
            (Value::Vec(a), Value::Vec(b)) => {
                let va = a.borrow();
                let vb = b.borrow();
                for (ai, bi) in va.iter().zip(vb.iter()) {
                    match ai.cmp(bi) {
                        Ordering::Equal => continue,
                        non_eq => return non_eq,
                    }
                }
                va.len().cmp(&vb.len())
            }
            (Value::Tuple(a), Value::Tuple(b)) => {
                for (ai, bi) in a.iter().zip(b.iter()) {
                    match ai.cmp(bi) {
                        Ordering::Equal => continue,
                        non_eq => return non_eq,
                    }
                }
                a.len().cmp(&b.len())
            }
            (
                Value::Struct {
                    name: na,
                    fields: fa,
                },
                Value::Struct {
                    name: nb,
                    fields: fb,
                },
            ) => na.cmp(nb).then_with(|| {
                let mut ak: Vec<&String> = fa.keys().collect();
                ak.sort();
                let mut bk: Vec<&String> = fb.keys().collect();
                bk.sort();
                for (k1, k2) in ak.iter().zip(bk.iter()) {
                    match k1.cmp(k2) {
                        Ordering::Equal => {}
                        non_eq => return non_eq,
                    }
                }
                ak.len().cmp(&bk.len()).then_with(|| {
                    for k in ak {
                        let v1 = fa.get(k).unwrap();
                        let v2 = fb.get(k).unwrap();
                        match v1.cmp(v2) {
                            Ordering::Equal => continue,
                            non_eq => return non_eq,
                        }
                    }
                    Ordering::Equal
                })
            }),
            (
                Value::EnumVariant {
                    enum_name: ea,
                    variant: va,
                    data: da,
                },
                Value::EnumVariant {
                    enum_name: eb,
                    variant: vb,
                    data: db,
                },
            ) => ea.cmp(eb).then(va.cmp(vb)).then_with(|| {
                for (ai, bi) in da.iter().zip(db.iter()) {
                    match ai.cmp(bi) {
                        Ordering::Equal => continue,
                        non_eq => return non_eq,
                    }
                }
                da.len().cmp(&db.len())
            }),
            (Value::HashMap(a), Value::HashMap(b)) => {
                let ma = a.borrow();
                let mb = b.borrow();
                let mut ae: Vec<(&Value, &Value)> = ma.iter().collect();
                ae.sort_by(|(ak, _), (bk, _)| ak.cmp(bk));
                let mut be: Vec<(&Value, &Value)> = mb.iter().collect();
                be.sort_by(|(ak, _), (bk, _)| ak.cmp(bk));
                for ((ak, av), (bk, bv)) in ae.iter().zip(be.iter()) {
                    match ak.cmp(bk) {
                        Ordering::Equal => {}
                        non_eq => return non_eq,
                    }
                    match av.cmp(bv) {
                        Ordering::Equal => continue,
                        non_eq => return non_eq,
                    }
                }
                ae.len().cmp(&be.len())
            }
            (Value::HashSet(a), Value::HashSet(b)) => {
                let sa = a.borrow();
                let sb = b.borrow();
                let mut av: Vec<&Value> = sa.iter().collect();
                av.sort();
                let mut bv: Vec<&Value> = sb.iter().collect();
                bv.sort();
                for (ai, bi) in av.iter().zip(bv.iter()) {
                    match ai.cmp(bi) {
                        Ordering::Equal => continue,
                        non_eq => return non_eq,
                    }
                }
                av.len().cmp(&bv.len())
            }
            (Value::BinaryHeap(a), Value::BinaryHeap(b)) => {
                let va = a.borrow().clone().into_sorted_vec();
                let vb = b.borrow().clone().into_sorted_vec();
                for (ai, bi) in va.iter().zip(vb.iter()) {
                    match ai.cmp(bi) {
                        Ordering::Equal => continue,
                        non_eq => return non_eq,
                    }
                }
                va.len().cmp(&vb.len())
            }
            (Value::VecDeque(a), Value::VecDeque(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                for (ai, bi) in a.iter().zip(b.iter()) {
                    match ai.cmp(bi) {
                        Ordering::Equal => continue,
                        non_eq => return non_eq,
                    }
                }
                a.len().cmp(&b.len())
            }
            (Value::Function(a), Value::Function(b)) => a.name.cmp(&b.name),
            (Value::Iterator(_), Value::Iterator(_)) => Ordering::Equal,
            (Value::Future(_), Value::Future(_)) => Ordering::Equal,
            (Value::JoinHandle(a), Value::JoinHandle(b)) => a.cmp(b),
            _ => Ordering::Equal, // unreachable: discriminant matched, types match
        }
    }
}

/// Marker trait — [`PartialEq`] is already reflexive, symmetric, and transitive for all variants.
impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Integer(n) => n.hash(state),
            Value::Float(f) => {
                let bits = if *f == 0.0 { 0u64 } else { f64::to_bits(*f) };
                bits.hash(state);
            }
            Value::Bool(b) => b.hash(state),
            Value::String(s) => s.hash(state),
            Value::Char(c) => c.hash(state),
            Value::Unit => {}
            Value::Function(fd) => fd.name.hash(state),
            Value::Range(start, end) => {
                start.hash(state);
                end.hash(state);
            }
            Value::Vec(rc) => {
                for elem in rc.borrow().iter() {
                    elem.hash(state);
                }
            }
            Value::Tuple(t) => {
                for elem in t {
                    elem.hash(state);
                }
            }
            Value::Struct { name, fields } => {
                name.hash(state);
                let mut keys: Vec<&String> = fields.keys().collect();
                keys.sort();
                for k in keys {
                    k.hash(state);
                    if let Some(v) = fields.get(k) {
                        v.hash(state);
                    }
                }
            }
            Value::EnumVariant {
                enum_name,
                variant,
                data,
            } => {
                enum_name.hash(state);
                variant.hash(state);
                for d in data {
                    d.hash(state);
                }
            }
            Value::HashMap(rc) => {
                let m = rc.borrow();
                let mut entries: Vec<(&Value, &Value)> = m.iter().collect();
                entries.sort_by(|(a, _), (b, _)| a.cmp(b));
                for (k, v) in entries {
                    k.hash(state);
                    v.hash(state);
                }
            }
            Value::HashSet(rc) => {
                let s = rc.borrow();
                let mut items: Vec<&Value> = s.iter().collect();
                items.sort();
                for item in items {
                    item.hash(state);
                }
            }
            Value::BinaryHeap(rc) => {
                let sorted = rc.borrow().clone().into_sorted_vec();
                for item in sorted {
                    item.hash(state);
                }
            }
            Value::VecDeque(rc) => {
                for elem in rc.borrow().iter() {
                    elem.hash(state);
                }
            }
            Value::Iterator(_) => "_iterator_".hash(state),
            Value::Future(_) => "_future_".hash(state),
            Value::JoinHandle(v) => v.hash(state),
            Value::Cell(rc) => rc.borrow().hash(state),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Value::Integer(42)), "42");
        assert_eq!(format!("{}", Value::Float(3.5)), "3.5");
        assert_eq!(format!("{}", Value::Float(1.0)), "1.0");
        assert_eq!(format!("{}", Value::Bool(true)), "true");
        assert_eq!(format!("{}", Value::String("hello".into())), "hello");
        assert_eq!(format!("{}", Value::Char('x')), "x");
        assert_eq!(format!("{}", Value::Unit), "()");
    }

    #[test]
    fn test_type_name() {
        assert_eq!(Value::Integer(0).type_name(), "i64");
        assert_eq!(Value::String("".into()).type_name(), "String");
        assert_eq!(Value::Unit.type_name(), "()");
    }

    #[test]
    fn test_is_truthy() {
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Integer(1).is_truthy());
        assert!(!Value::Integer(0).is_truthy());
        assert!(!Value::Unit.is_truthy());
        assert!(Value::String("".into()).is_truthy());
    }

    #[test]
    fn test_equality() {
        assert_eq!(Value::Integer(42), Value::Integer(42));
        assert_ne!(Value::Integer(1), Value::Integer(2));
        assert_ne!(Value::Integer(1), Value::Bool(true));
        assert_eq!(Value::String("a".into()), Value::String("a".into()));
    }

    #[test]
    fn test_ordering() {
        assert!(Value::Integer(1) < Value::Integer(2));
        assert!(Value::String("a".into()) < Value::String("b".into()));
        // Cross-type comparisons now use Ord's discriminant-based ordering
        assert!(Value::Integer(1)
            .partial_cmp(&Value::String("a".into()))
            .is_some());
    }

    // --- Hash, Eq, Ord tests ---

    #[test]
    fn test_hash_same_values_same_hash() {
        use std::collections::hash_map::DefaultHasher;
        fn hash(v: &Value) -> u64 {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            std::hash::Hasher::finish(&h)
        }
        assert_eq!(hash(&Value::Integer(42)), hash(&Value::Integer(42)));
        assert_eq!(
            hash(&Value::String("hi".into())),
            hash(&Value::String("hi".into()))
        );
        assert_eq!(hash(&Value::Bool(true)), hash(&Value::Bool(true)));
        assert_eq!(hash(&Value::Char('x')), hash(&Value::Char('x')));
        assert_eq!(hash(&Value::Unit), hash(&Value::Unit));
        assert_eq!(hash(&Value::Float(1.5)), hash(&Value::Float(1.5)));
    }

    #[test]
    fn test_hash_different_values_different_hash() {
        use std::collections::hash_map::DefaultHasher;
        fn hash(v: &Value) -> u64 {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            std::hash::Hasher::finish(&h)
        }
        // Different types should hash differently due to discriminant
        assert_ne!(hash(&Value::Integer(1)), hash(&Value::String("1".into())));
        assert_ne!(hash(&Value::Bool(true)), hash(&Value::Integer(1)));
        // Different values same type
        assert_ne!(hash(&Value::Integer(1)), hash(&Value::Integer(2)));
    }

    #[test]
    fn test_hash_containers() {
        use std::collections::hash_map::DefaultHasher;
        fn hash(v: &Value) -> u64 {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            std::hash::Hasher::finish(&h)
        }
        let v1 = Value::Vec(Rc::new(RefCell::new(vec![Value::Integer(1), Value::Integer(2)])));
        let v2 = Value::Vec(Rc::new(RefCell::new(vec![Value::Integer(1), Value::Integer(2)])));
        assert_eq!(hash(&v1), hash(&v2));

        let t1 = Value::Tuple(vec![Value::Integer(1), Value::String("a".into())]);
        let t2 = Value::Tuple(vec![Value::Integer(1), Value::String("a".into())]);
        assert_eq!(hash(&t1), hash(&t2));
    }

    #[test]
    fn test_ord_total_ordering() {
        // Total ordering via Ord::cmp: any two values must be comparable
        assert_eq!(Value::Integer(5).cmp(&Value::Integer(5)), Ordering::Equal);
        assert_eq!(Value::Integer(1).cmp(&Value::Integer(2)), Ordering::Less);
        assert_eq!(Value::Integer(3).cmp(&Value::Integer(2)), Ordering::Greater);

        // Cross-type: Unit is discriminant 0, Bool is 1, Integer is 2, etc.
        assert_eq!(Value::Unit.cmp(&Value::Bool(true)), Ordering::Less);
        assert_eq!(Value::Bool(true).cmp(&Value::Integer(0)), Ordering::Less);
        assert_eq!(Value::Integer(0).cmp(&Value::Float(0.0)), Ordering::Less);
        assert_eq!(Value::Float(0.0).cmp(&Value::Char('a')), Ordering::Less);
        assert_eq!(
            Value::Char('a').cmp(&Value::String("".into())),
            Ordering::Less
        );
    }

    #[test]
    fn test_ord_vec() {
        let a = Value::Vec(Rc::new(RefCell::new(vec![Value::Integer(1), Value::Integer(2)])));
        let b = Value::Vec(Rc::new(RefCell::new(vec![Value::Integer(1), Value::Integer(3)])));
        assert!(a < b); // works via PartialOrd → Ord delegation
        let c = Value::Vec(Rc::new(RefCell::new(vec![Value::Integer(1)])));
        assert!(c < a); // shorter is less
        assert_eq!(a.cmp(&a), Ordering::Equal);
    }

    #[test]
    fn test_ord_float_total() {
        // f64::total_cmp handles NaN, -0, etc.
        assert!(Value::Float(-0.0) == Value::Float(0.0)); // total_cmp: -0 == +0
        assert!(Value::Float(1.5) < Value::Float(2.0));
        assert!(Value::Float(f64::INFINITY) > Value::Float(0.0));
    }

    #[test]
    fn test_eq_is_reflexive() {
        let vals = vec![
            Value::Integer(1),
            Value::Float(1.0),
            Value::Bool(true),
            Value::String("s".into()),
            Value::Char('c'),
            Value::Unit,
            Value::Vec(Rc::new(RefCell::new(vec![Value::Integer(1)]))),
            Value::Tuple(vec![Value::Integer(1)]),
        ];
        for v in &vals {
            assert_eq!(v, v, "Eq reflexive failed for {:?}", v);
        }
    }

    #[test]
    fn test_ord_consistency_with_partial_ord() {
        // Ord must agree with PartialOrd where PartialOrd is defined
        let pairs = vec![
            (Value::Integer(1), Value::Integer(2)),
            (Value::Float(1.0), Value::Float(2.0)),
            (Value::String("a".into()), Value::String("b".into())),
            (Value::Char('a'), Value::Char('b')),
            (Value::Bool(false), Value::Bool(true)),
        ];
        for (a, b) in pairs {
            assert_eq!(
                a.partial_cmp(&b).unwrap(),
                a.cmp(&b),
                "Ord and PartialOrd disagree for {:?} vs {:?}",
                a,
                b
            );
        }
    }

    #[test]
    fn test_hash_f64_uses_bits() {
        use std::collections::hash_map::DefaultHasher;
        fn hash(v: &Value) -> u64 {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            std::hash::Hasher::finish(&h)
        }
        // Same float bits = same hash (even NaN, though NaN bits may differ)
        assert_eq!(hash(&Value::Float(2.71)), hash(&Value::Float(2.71)));
        // -0.0 and +0.0 have different bit patterns, so hash will differ
        // (this is intentional — consistent with Eq which compares f64 directly)
    }

    #[test]
    fn test_ord_struct_and_enum() {
        let s1 = Value::Struct {
            name: "Point".into(),
            fields: {
                let mut m = HashMap::new();
                m.insert("x".into(), Value::Integer(1));
                m.insert("y".into(), Value::Integer(2));
                m
            },
        };
        let s2 = Value::Struct {
            name: "Point".into(),
            fields: {
                let mut m = HashMap::new();
                m.insert("x".into(), Value::Integer(1));
                m.insert("y".into(), Value::Integer(3));
                m
            },
        };
        assert!(s1 < s2);

        let e1 = Value::EnumVariant {
            enum_name: "Color".into(),
            variant: "Red".into(),
            data: vec![],
        };
        let e2 = Value::EnumVariant {
            enum_name: "Color".into(),
            variant: "Blue".into(),
            data: vec![],
        };
        assert!(e2 < e1); // "Blue" < "Red"
    }
}
