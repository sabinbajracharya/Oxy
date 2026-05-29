//! Canonical IR snapshot serializer.
//!
//! Implements the format specified in `IR_SNAPSHOT_FORMAT.md`.
//! This serializer operates pre-codegen on `IrFunction` values and produces
//! a deterministic, diff-stable text representation suitable for golden tests.
//!
//! Do NOT use `{:?}` formatting or the existing `IrFunction::dump()` / `Display`
//! impls from `ir.rs` — those are non-canonical trace helpers.

use std::collections::HashMap;

use super::ir::{BasicBlock, BlockId, IrFunction, IrOp, Reg, Terminator};

// ── §8: forbidden codegen-synthesized FFI names ────────────────────────────

const CODEGEN_FFI_BLACKLIST: &[&str] = &[
    "oxy_push_int",
    "oxy_push_bool",
    "oxy_push_float",
    "oxy_push_char",
    "oxy_push_string",
    "oxy_push_unit",
    "oxy_load_local",
    "oxy_load_local_raw",
    "oxy_store_local",
    "oxy_read_local_i64",
    "oxy_set_result_i64",
    "oxy_return",
    "oxy_error_discriminant",
    "oxy_panic",
];

// ── Public(crate) API ──────────────────────────────────────────────────────

/// Serialize a full program (slice of functions) to the canonical snapshot format.
/// Functions are sorted by (name, fn_index) per §2.1.
pub(crate) fn serialize_program(functions: &[IrFunction]) -> String {
    let mut sorted: Vec<&IrFunction> = functions.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name).then(a.fn_index.cmp(&b.fn_index)));

    let parts: Vec<String> = sorted.iter().map(|f| serialize_function(f)).collect();
    let mut out = parts.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Serialize a single `IrFunction` to the canonical snapshot format.
pub(crate) fn serialize_function(func: &IrFunction) -> String {
    let canonical_order = canonical_block_order(func);
    let reg_names = assign_register_names(func, &canonical_order);
    let pred_map = compute_predecessors(func, &canonical_order);

    let mut out = String::new();

    // ── Function header (§5) ──
    out.push_str("fn ");
    out.push_str(&func.name);
    out.push('(');
    let params: Vec<String> = func
        .params
        .iter()
        .map(|(name, ty)| format!("{}: {}", name, ty.display_name()))
        .collect();
    out.push_str(&params.join(", "));
    out.push_str(") -> ");
    out.push_str(&func.return_type.display_name());
    if func.is_async {
        out.push_str(" async");
    }
    out.push_str(" {\n");

    out.push_str(&format!("  locals: {}\n", func.local_count));

    if !func.captures.is_empty() {
        let caps: Vec<String> = func
            .captures
            .iter()
            .map(|(name, slot)| format!("{}@${}", name, slot))
            .collect();
        out.push_str(&format!("  captures: {}\n", caps.join(", ")));
    }

    // ── Blocks in canonical order ──
    for (canonical_idx, &raw_id) in canonical_order.iter().enumerate() {
        if canonical_idx > 0 {
            out.push('\n');
        }
        let block = &func.blocks[raw_id];
        out.push_str(&serialize_block(
            block,
            canonical_idx,
            raw_id,
            func.entry,
            &canonical_order,
            &pred_map,
            &reg_names,
        ));
    }

    out.push_str("}\n");
    out
}

// ── Block serialization ────────────────────────────────────────────────────

fn serialize_block(
    block: &BasicBlock,
    canonical_idx: usize,
    raw_id: BlockId,
    entry: BlockId,
    canonical_order: &[BlockId],
    pred_map: &HashMap<BlockId, Vec<usize>>, // raw_id → canonical indices of preds
    reg_names: &HashMap<Reg, usize>,
) -> String {
    let mut out = String::new();

    // ── Block header with tag ──
    let tag = if raw_id == entry {
        "entry".to_string()
    } else {
        let preds = pred_map
            .get(&raw_id)
            .map(|idxs| {
                idxs.iter()
                    .map(|&ci| format!("bb{}", ci))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        format!("preds: {}", preds)
    };
    out.push_str(&format!("  bb{}({}):\n", canonical_idx, tag));

    // ── Instructions ──
    for op in &block.ops {
        out.push_str("    ");
        out.push_str(&serialize_op(op, reg_names));
        out.push('\n');
    }

    // ── Terminator ──
    out.push_str("    ");
    out.push_str(&serialize_terminator(
        &block.terminator,
        reg_names,
        canonical_order,
    ));
    out.push('\n');

    out
}

// ── Op serialization (§6 table) ───────────────────────────────────────────

fn serialize_op(op: &IrOp, reg_names: &HashMap<Reg, usize>) -> String {
    // Helper: resolve a register to its canonical vN name.
    let r = |reg: &Reg| -> String {
        match reg_names.get(reg) {
            Some(n) => format!("v{}", n),
            None => format!("<undef:r{}>", reg),
        }
    };

    match op {
        IrOp::ConstInt(dst, n) => format!("{} = const.int {}", r(dst), n),
        IrOp::ConstFloat(dst, n) => format!("{} = const.float {}", r(dst), fmt_float(*n)),
        IrOp::ConstBool(dst, b_val) => format!("{} = const.bool {}", r(dst), b_val),
        IrOp::ConstChar(dst, c) => format!("{} = const.char '{}'", r(dst), escape_char(*c)),
        IrOp::ConstUnit(dst) => format!("{} = const.unit", r(dst)),
        IrOp::ConstString(dst, s) => format!("{} = const.str \"{}\"", r(dst), escape_string(s)),

        IrOp::LoadLocal(dst, slot) => format!("{} = load.local ${}", r(dst), slot),
        IrOp::LoadLocalRaw(dst, slot) => format!("{} = load.local.raw ${}", r(dst), slot),
        IrOp::StoreLocal(slot, src) => format!("store.local ${}, {}", slot, r(src)),

        IrOp::Add(dst, a, bv) => format!("{} = add {}, {}", r(dst), r(a), r(bv)),
        IrOp::Sub(dst, a, bv) => format!("{} = sub {}, {}", r(dst), r(a), r(bv)),
        IrOp::Mul(dst, a, bv) => format!("{} = mul {}, {}", r(dst), r(a), r(bv)),
        IrOp::Div(dst, a, bv) => format!("{} = div {}, {}", r(dst), r(a), r(bv)),
        IrOp::Rem(dst, a, bv) => format!("{} = rem {}, {}", r(dst), r(a), r(bv)),

        IrOp::Eq(dst, a, bv) => format!("{} = eq {}, {}", r(dst), r(a), r(bv)),
        IrOp::Neq(dst, a, bv) => format!("{} = ne {}, {}", r(dst), r(a), r(bv)),
        IrOp::Lt(dst, a, bv) => format!("{} = lt {}, {}", r(dst), r(a), r(bv)),
        IrOp::Gt(dst, a, bv) => format!("{} = gt {}, {}", r(dst), r(a), r(bv)),
        IrOp::Le(dst, a, bv) => format!("{} = le {}, {}", r(dst), r(a), r(bv)),
        IrOp::Ge(dst, a, bv) => format!("{} = ge {}, {}", r(dst), r(a), r(bv)),

        IrOp::And(dst, a, bv) => format!("{} = and {}, {}", r(dst), r(a), r(bv)),
        IrOp::Or(dst, a, bv) => format!("{} = or {}, {}", r(dst), r(a), r(bv)),

        IrOp::BitAnd(dst, a, bv) => format!("{} = bitand {}, {}", r(dst), r(a), r(bv)),
        IrOp::BitOr(dst, a, bv) => format!("{} = bitor {}, {}", r(dst), r(a), r(bv)),
        IrOp::BitXor(dst, a, bv) => format!("{} = bitxor {}, {}", r(dst), r(a), r(bv)),
        IrOp::Shl(dst, a, bv) => format!("{} = shl {}, {}", r(dst), r(a), r(bv)),
        IrOp::Shr(dst, a, bv) => format!("{} = shr {}, {}", r(dst), r(a), r(bv)),

        IrOp::Neg(dst, a) => format!("{} = neg {}", r(dst), r(a)),
        IrOp::Not(dst, a) => format!("{} = not {}", r(dst), r(a)),
        IrOp::BitNot(dst, a) => format!("{} = bitnot {}", r(dst), r(a)),

        IrOp::Copy(dst, a) => format!("{} = copy {}", r(dst), r(a)),
        IrOp::Phi(dst, a, bv) => format!("{} = phi {}, {}", r(dst), r(a), r(bv)),

        IrOp::ReadResult(dst) => format!("{} = read.result", r(dst)),
        IrOp::WriteResult(src) => format!("write.result {}", r(src)),
        IrOp::SetError(src) => format!("set.error {}", r(src)),
        IrOp::CheckError(dst) => format!("{} = check.error", r(dst)),

        IrOp::CallBuiltin {
            result,
            func,
            args,
            immediates,
            strings,
        } => {
            // §8 check
            if CODEGEN_FFI_BLACKLIST.contains(func) {
                return format!("<malformed: codegen-only @{}>", func);
            }

            let arg_list: Vec<String> = args.iter().map(&r).collect();
            let mut s = format!("{} = call @{}({})", r(result), func, arg_list.join(", "));

            if !immediates.is_empty() {
                let imm_str: Vec<String> = immediates.iter().map(|i| i.to_string()).collect();
                s.push_str(&format!(" imm[{}]", imm_str.join(", ")));
            }
            if !strings.is_empty() {
                let str_parts: Vec<String> = strings
                    .iter()
                    .map(|st| format!("\"{}\"", escape_string(st)))
                    .collect();
                s.push_str(&format!(" str[{}]", str_parts.join(", ")));
            }
            s
        }
    }
}

fn serialize_terminator(
    term: &Terminator,
    reg_names: &HashMap<Reg, usize>,
    canonical_order: &[BlockId],
) -> String {
    let r = |reg: &Reg| -> String {
        match reg_names.get(reg) {
            Some(n) => format!("v{}", n),
            None => format!("<undef:r{}>", reg),
        }
    };
    let b = |raw: &BlockId| -> String {
        match canonical_order.iter().position(|&id| id == *raw) {
            Some(ci) => format!("bb{}", ci),
            None => format!("<dead:block{}>", raw),
        }
    };

    match term {
        Terminator::Return(reg) => format!("ret {}", r(reg)),
        Terminator::Jump(target) => format!("jump {}", b(target)),
        Terminator::Branch {
            cond,
            then_block,
            else_block,
        } => {
            format!("branch {} -> {}, {}", r(cond), b(then_block), b(else_block))
        }
        Terminator::Halt => "halt".to_string(),
        Terminator::Panic(reg) => format!("panic {}", r(reg)),
    }
}

// ── §2.2 RPO block ordering ────────────────────────────────────────────────

fn canonical_block_order(func: &IrFunction) -> Vec<BlockId> {
    let n = func.blocks.len();
    if n == 0 {
        return vec![];
    }

    let mut visited = vec![false; n];
    let mut postorder: Vec<BlockId> = Vec::with_capacity(n);

    // Iterative DFS to avoid stack overflow on deep IR.
    let mut stack: Vec<(BlockId, usize)> = vec![(func.entry, 0)];
    visited[func.entry] = true;

    while let Some((id, child_idx)) = stack.last_mut() {
        let succs = block_successors(&func.blocks[*id].terminator);
        if *child_idx < succs.len() {
            let s = succs[*child_idx];
            *child_idx += 1;
            if s < n && !visited[s] {
                visited[s] = true;
                stack.push((s, 0));
            }
        } else {
            let id = *id;
            stack.pop();
            postorder.push(id);
        }
    }

    postorder.reverse(); // RPO = reverse of postorder

    // Append unreachable blocks in ascending raw-id order.
    for (i, is_visited) in visited.iter().enumerate().take(n) {
        if !is_visited {
            postorder.push(i);
        }
    }

    postorder
}

fn block_successors(term: &Terminator) -> Vec<BlockId> {
    match term {
        Terminator::Jump(t) => vec![*t],
        Terminator::Branch {
            then_block,
            else_block,
            ..
        } => vec![*then_block, *else_block],
        Terminator::Return(_) | Terminator::Halt | Terminator::Panic(_) => vec![],
    }
}

// ── §2.6 Recomputed predecessors ──────────────────────────────────────────

/// Returns a map from raw BlockId → sorted canonical indices of predecessors.
fn compute_predecessors(
    func: &IrFunction,
    canonical_order: &[BlockId],
) -> HashMap<BlockId, Vec<usize>> {
    let mut map: HashMap<BlockId, Vec<usize>> = HashMap::new();

    // Initialise empty vecs for all blocks
    for &raw_id in canonical_order {
        map.entry(raw_id).or_default();
    }

    // Walk all blocks, add successor edges
    for (ci, &raw_id) in canonical_order.iter().enumerate() {
        let block = &func.blocks[raw_id];
        for succ in block_successors(&block.terminator) {
            map.entry(succ).or_default().push(ci);
        }
    }

    // Sort predecessor lists so output is deterministic
    for preds in map.values_mut() {
        preds.sort_unstable();
        preds.dedup();
    }

    map
}

// ── §3 Two-pass register naming ────────────────────────────────────────────

/// Returns a map from raw Reg id → canonical vN index.
/// Uses the §6.1 def/use table (overrides result_reg()).
fn assign_register_names(func: &IrFunction, canonical_order: &[BlockId]) -> HashMap<Reg, usize> {
    let mut map: HashMap<Reg, usize> = HashMap::new();
    let mut next = 0usize;

    for &raw_id in canonical_order {
        let block = &func.blocks[raw_id];
        for op in &block.ops {
            if let Some(def) = op_defined_reg(op) {
                map.entry(def).or_insert_with(|| {
                    let n = next;
                    next += 1;
                    n
                });
            }
        }
        // Terminators never define registers, but we still traverse for completeness.
    }

    map
}

/// §6.1: which register an op *defines* for naming purposes.
/// Overrides `IrOp::result_reg()` for WriteResult, SetError (they are uses).
fn op_defined_reg(op: &IrOp) -> Option<Reg> {
    match op {
        // Explicit non-defs (§6.1 overrides)
        IrOp::StoreLocal(_, _) => None,
        IrOp::WriteResult(_) => None,
        IrOp::SetError(_) => None,
        // Everything else that carries a result register
        IrOp::ConstInt(r, _)
        | IrOp::ConstFloat(r, _)
        | IrOp::ConstBool(r, _)
        | IrOp::ConstChar(r, _)
        | IrOp::ConstUnit(r)
        | IrOp::ConstString(r, _)
        | IrOp::LoadLocal(r, _)
        | IrOp::LoadLocalRaw(r, _)
        | IrOp::Add(r, _, _)
        | IrOp::Sub(r, _, _)
        | IrOp::Mul(r, _, _)
        | IrOp::Div(r, _, _)
        | IrOp::Rem(r, _, _)
        | IrOp::Eq(r, _, _)
        | IrOp::Neq(r, _, _)
        | IrOp::Lt(r, _, _)
        | IrOp::Gt(r, _, _)
        | IrOp::Le(r, _, _)
        | IrOp::Ge(r, _, _)
        | IrOp::And(r, _, _)
        | IrOp::Or(r, _, _)
        | IrOp::BitAnd(r, _, _)
        | IrOp::BitOr(r, _, _)
        | IrOp::BitXor(r, _, _)
        | IrOp::Shl(r, _, _)
        | IrOp::Shr(r, _, _)
        | IrOp::Neg(r, _)
        | IrOp::Not(r, _)
        | IrOp::BitNot(r, _)
        | IrOp::Copy(r, _)
        | IrOp::Phi(r, _, _)
        | IrOp::ReadResult(r)
        | IrOp::CheckError(r) => Some(*r),
        IrOp::CallBuiltin { result, .. } => Some(*result),
    }
}

// ── §7 Format helpers ──────────────────────────────────────────────────────

/// §7.3: canonical float representation.
fn fmt_float(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        return if n > 0.0 {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }
    // Detect -0.0
    if n == 0.0 && n.is_sign_negative() {
        return "-0.0".to_string();
    }
    let s = format!("{}", n);
    // Append .0 if no decimal point or exponent present
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s
    } else {
        format!("{}.0", s)
    }
}

/// §7.1: escape a string for canonical snapshot output (double-quoted content).
fn escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{{{:x}}}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// §7.2: escape a char for canonical snapshot output (single-quoted content).
fn escape_char(c: char) -> String {
    match c {
        '\\' => "\\\\".to_string(),
        '\'' => "\\'".to_string(),
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        '\0' => "\\0".to_string(),
        c if (c as u32) < 0x20 || c as u32 == 0x7f => {
            format!("\\u{{{:x}}}", c as u32)
        }
        c => c.to_string(),
    }
}

// ── Unit tests for helpers ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt_float_normal() {
        // §7.3: bare integer-looking floats get .0 appended for clarity
        assert_eq!(fmt_float(1.0), "1.0");
        assert_eq!(fmt_float(0.0), "0.0");
        assert_eq!(fmt_float(1.5), "1.5");
        assert_eq!(fmt_float(1.23e10), "12300000000.0");
        // Very large floats: Rust Display gives decimal (no e), gets .0 appended
        assert!(fmt_float(1e100).ends_with(".0"));
    }

    #[test]
    fn test_fmt_float_specials() {
        assert_eq!(fmt_float(f64::INFINITY), "inf");
        assert_eq!(fmt_float(f64::NEG_INFINITY), "-inf");
        assert_eq!(fmt_float(f64::NAN), "NaN");
        assert_eq!(fmt_float(-0.0_f64), "-0.0");
    }

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("a\nb"), "a\\nb");
        assert_eq!(escape_string("a\"b"), "a\\\"b");
        assert_eq!(escape_string("a\\b"), "a\\\\b");
        assert_eq!(escape_string("a\tb"), "a\\tb");
    }

    #[test]
    fn test_escape_char() {
        assert_eq!(escape_char('a'), "a");
        assert_eq!(escape_char('\''), "\\'");
        assert_eq!(escape_char('\n'), "\\n");
        assert_eq!(escape_char('\\'), "\\\\");
    }

    #[test]
    fn test_canonical_block_order_straight_line() {
        use super::super::ir::IrFunction;
        let mut f = IrFunction::new("test".to_string(), 0, 0, 0);
        let b0 = f.add_block();
        let b1 = f.add_block();
        f.block_mut(b0).terminate(Terminator::Jump(b1));
        f.block_mut(b1).terminate(Terminator::Halt);
        let order = canonical_block_order(&f);
        assert_eq!(order, vec![0, 1]);
    }

    #[test]
    fn test_canonical_block_order_branch() {
        use super::super::ir::{IrFunction, IrOp};
        let mut f = IrFunction::new("test".to_string(), 0, 1, 0);
        let b0 = f.add_block(); // entry: branch
        let b1 = f.add_block(); // then
        let b2 = f.add_block(); // else
        let b3 = f.add_block(); // merge
        f.block_mut(b0).push(IrOp::ConstBool(0, true));
        f.block_mut(b0).terminate(Terminator::Branch {
            cond: 0,
            then_block: b1,
            else_block: b2,
        });
        f.block_mut(b1).terminate(Terminator::Jump(b3));
        f.block_mut(b2).terminate(Terminator::Jump(b3));
        f.block_mut(b3).terminate(Terminator::Halt);
        let order = canonical_block_order(&f);
        // Entry first, then RPO: [0, 1, 2, 3] or [0, 2, 1, 3] depending on DFS
        // With fixed successor order (then before else): entry=0, then=1, merge=3, else=2? No.
        // DFS: visit 0 -> push 1 (then), visit 1 -> push 3, visit 3 -> no succs, post=[3]
        //      back to 1, post=[3,1], back to 0, push 2, visit 2 -> jump 3 (visited), post=[3,1,2]
        //      back to 0, post=[3,1,2,0], reverse=[0,2,1,3]
        assert_eq!(order[0], 0); // entry is always first
    }
}
