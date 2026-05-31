//! Comparison, ordering, and hashing trait implementations for [`Value`].
//!
//! Extracted from [`super`] to keep the Value definition file under ~700 lines.

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use super::Value;

/// Structural equality for [`Value`]; functions and futures are never equal.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        // Numeric comparison: all integer variants compare by widening to i128,
        // all float variants compare by widening to f64.
        if let (Some(a), Some(b)) = (self.as_i128(), other.as_i128()) {
            return a == b;
        }
        if let (Some(a), Some(b)) = (self.as_f64(), other.as_f64()) {
            return a == b;
        }
        if let (Some(ia), Some(fb)) = (self.as_i128(), other.as_f64()) {
            return ia as f64 == fb;
        }
        if let (Some(fa), Some(ib)) = (self.as_f64(), other.as_i128()) {
            return fa == ib as f64;
        }
        match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
            (Value::Range(a1, a2), Value::Range(b1, b2)) => a1 == b1 && a2 == b2,
            (Value::Vec(a), Value::Vec(b)) => *a.borrow() == *b.borrow(),
            (Value::Array(a), Value::Array(b)) => a == b,
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
            (Value::BTreeMap(a), Value::BTreeMap(b)) => *a.borrow() == *b.borrow(),
            (Value::BTreeSet(a), Value::BTreeSet(b)) => *a.borrow() == *b.borrow(),
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
            (Value::AsyncResult { .. }, Value::AsyncResult { .. }) => false,
            (Value::Iterator(_), Value::Iterator(_)) => false,
            _ => false,
        }
    }
}

/// Ordering for [`Value`]; delegates to [`Ord`] for all comparisons.
impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
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
            // Numeric comparison via helpers (same discriminant for all ints=2, floats=3)
            _ if self.variant_discriminant() == 2 && other.variant_discriminant() == 2 => {
                self.as_i128().unwrap().cmp(&other.as_i128().unwrap())
            }
            _ if self.variant_discriminant() == 3 && other.variant_discriminant() == 3 => {
                self.as_f64().unwrap().total_cmp(&other.as_f64().unwrap())
            }
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
            (Value::Array(a), Value::Array(b)) => {
                for (ai, bi) in a.iter().zip(b.iter()) {
                    match ai.cmp(bi) {
                        Ordering::Equal => continue,
                        non_eq => return non_eq,
                    }
                }
                a.len().cmp(&b.len())
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
            (Value::BTreeMap(a), Value::BTreeMap(b)) => {
                let ma = a.borrow();
                let mb = b.borrow();
                for ((ak, av), (bk, bv)) in ma.iter().zip(mb.iter()) {
                    match ak.cmp(bk) {
                        Ordering::Equal => {}
                        non_eq => return non_eq,
                    }
                    match av.cmp(bv) {
                        Ordering::Equal => continue,
                        non_eq => return non_eq,
                    }
                }
                ma.len().cmp(&mb.len())
            }
            (Value::BTreeSet(a), Value::BTreeSet(b)) => {
                let sa = a.borrow();
                let sb = b.borrow();
                for (ai, bi) in sa.iter().zip(sb.iter()) {
                    match ai.cmp(bi) {
                        Ordering::Equal => continue,
                        non_eq => return non_eq,
                    }
                }
                sa.len().cmp(&sb.len())
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
            (Value::JoinHandle { task_id: a }, Value::JoinHandle { task_id: b }) => a.cmp(b),
            (Value::AsyncResult { .. }, Value::AsyncResult { .. }) => Ordering::Equal,
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
            Value::I64(n) => n.hash(state),
            Value::U8(n) => n.hash(state),
            Value::F64(x) => {
                let bits = if *x == 0.0 { 0 } else { f64::to_bits(*x) };
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
            Value::Array(a) => {
                for elem in a {
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
            Value::BTreeMap(rc) => {
                let m = rc.borrow();
                for (k, v) in m.iter() {
                    k.hash(state);
                    v.hash(state);
                }
            }
            Value::BTreeSet(rc) => {
                let s = rc.borrow();
                for item in s.iter() {
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
            Value::JoinHandle { task_id } => task_id.hash(state),
            Value::AsyncResult { .. } => "_async_result_".hash(state),
            Value::Cell(rc) => rc.borrow().hash(state),
        }
    }
}
