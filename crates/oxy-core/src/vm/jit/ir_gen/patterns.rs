//! Pattern checking and binding for `IrGen` — part of the AST → register IR
//! lowering pass. See `mod.rs` for the `IrGen` struct and state.

use super::*;

impl IrGen {
    /// Emit a pattern-match check: returns a register that is truthy if pattern matches.
    pub(super) fn gen_pattern_check(&mut self, pattern: &Pattern, val_reg: Reg) -> Reg {
        let mut r = self.alloc_reg();
        match pattern {
            Pattern::Literal(lit_expr) => {
                let lit_reg = self.gen_expr(lit_expr);
                self.emit(IrOp::Eq(r, val_reg, lit_reg));
            }
            Pattern::Wildcard(..) | Pattern::Ident(..) | Pattern::Rest(..) => {
                self.emit(IrOp::ConstBool(r, true));
            }
            Pattern::EnumVariant {
                enum_name,
                variant,
                fields,
                ..
            } => {
                // Check variant discriminant, then recursively check inner field
                // patterns. Canonicalize the enum name so the arm matches the
                // value's constructed identity across module / use-alias bounds.
                let resolved_enum = self.resolve_pattern_enum_name(enum_name, variant);
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_enum_variant_equal",
                    args: vec![val_reg],
                    immediates: vec![],
                    strings: vec![resolved_enum, variant.clone()],
                });
                // If there are inner patterns, also check them
                for (i, inner) in fields.iter().enumerate() {
                    let inner_val = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: inner_val,
                        func: "oxy_enum_data_get",
                        args: vec![val_reg],
                        immediates: vec![i],
                        strings: vec![],
                    });
                    let inner_match = self.gen_pattern_check(inner, inner_val);
                    // AND the results: r = r && inner_match
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::And(new_r, r, inner_match));
                    r = new_r;
                }
            }
            Pattern::Tuple(patterns, ..) => {
                // Check each element recursively, AND all results
                self.emit(IrOp::ConstBool(r, true));
                for (i, p) in patterns.iter().enumerate() {
                    let elem_val = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: elem_val,
                        func: "oxy_enum_data_get",
                        args: vec![val_reg],
                        immediates: vec![i],
                        strings: vec![],
                    });
                    let elem_match = self.gen_pattern_check(p, elem_val);
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::And(new_r, r, elem_match));
                    r = new_r;
                }
            }
            Pattern::Struct { fields, .. } => {
                // Check each named field recursively
                self.emit(IrOp::ConstBool(r, true));
                for (_fname, p) in fields.iter() {
                    let field_val = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: field_val,
                        func: "oxy_field_access",
                        args: vec![val_reg],
                        immediates: vec![],
                        strings: vec![_fname.clone()],
                    });
                    let field_match = self.gen_pattern_check(p, field_val);
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::And(new_r, r, field_match));
                    r = new_r;
                }
            }
            Pattern::Or(patterns, ..) => {
                // Match if ANY sub-pattern matches
                self.emit(IrOp::ConstBool(r, false));
                for p in patterns {
                    let sub_match = self.gen_pattern_check(p, val_reg);
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::Or(new_r, r, sub_match));
                    r = new_r;
                }
            }
            Pattern::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                // Check if val_reg is within [start, end] or [start, end)
                self.emit(IrOp::ConstBool(r, true));
                if let Some(lo) = start {
                    let lo_reg = self.alloc_reg();
                    self.emit(IrOp::ConstInt(lo_reg, *lo));
                    let ge_check = self.alloc_reg();
                    self.emit(IrOp::Ge(ge_check, val_reg, lo_reg));
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::And(new_r, r, ge_check));
                    r = new_r;
                }
                if let Some(hi) = end {
                    let hi_reg = self.alloc_reg();
                    self.emit(IrOp::ConstInt(hi_reg, *hi));
                    let cmp = self.alloc_reg();
                    if *inclusive {
                        self.emit(IrOp::Le(cmp, val_reg, hi_reg));
                    } else {
                        self.emit(IrOp::Lt(cmp, val_reg, hi_reg));
                    }
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::And(new_r, r, cmp));
                    r = new_r;
                }
            }
            Pattern::Slice(patterns, ..) => {
                // Check each element against sub-patterns, skipping Rest (..).
                // No length check — we don't have a collection-len FFI function yet.
                self.emit(IrOp::ConstBool(r, true));
                let mut elem_idx = 0usize;
                for p in patterns {
                    if matches!(p, Pattern::Rest(..)) {
                        continue;
                    }
                    let elem_val = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: elem_val,
                        func: "oxy_enum_data_get",
                        args: vec![val_reg],
                        immediates: vec![elem_idx],
                        strings: vec![],
                    });
                    let sub = self.gen_pattern_check(p, elem_val);
                    let new_r = self.alloc_reg();
                    self.emit(IrOp::And(new_r, r, sub));
                    r = new_r;
                    elem_idx += 1;
                }
            }
        }
        r
    }

    pub(super) fn gen_pattern_bind(&mut self, pattern: &Pattern, val_reg: Reg) {
        match pattern {
            Pattern::Ident(name, ..) => {
                let slot = self.alloc_local(name);
                self.emit(IrOp::StoreLocal(slot, val_reg));
            }
            Pattern::Wildcard(..) => {}
            Pattern::EnumVariant { fields, .. } => {
                for (i, p) in fields.iter().enumerate() {
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_enum_data_get",
                        args: vec![val_reg],
                        immediates: vec![i],
                        strings: vec![],
                    });
                    self.gen_pattern_bind(p, r);
                }
            }
            Pattern::Struct { fields, .. } => {
                for (_fname, p) in fields.iter() {
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_field_access",
                        args: vec![val_reg],
                        immediates: vec![],
                        strings: vec![_fname.clone()],
                    });
                    self.gen_pattern_bind(p, r);
                }
            }
            Pattern::Tuple(patterns, ..) => {
                for (i, p) in patterns.iter().enumerate() {
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_enum_data_get",
                        args: vec![val_reg],
                        immediates: vec![i],
                        strings: vec![],
                    });
                    self.gen_pattern_bind(p, r);
                }
            }
            Pattern::Literal(..) => {}
            Pattern::Or(patterns, ..) => {
                // Bind from the first sub-pattern. The type checker ensures all
                // arms bind the same variables with the same types, so any arm
                // produces equivalent bindings.
                if let Some(first) = patterns.first() {
                    self.gen_pattern_bind(first, val_reg);
                }
            }
            Pattern::Rest(..) => {}
            Pattern::Slice(patterns, ..) => {
                let mut elem_idx = 0usize;
                for p in patterns {
                    if matches!(p, Pattern::Rest(..)) {
                        continue;
                    }
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_enum_data_get",
                        args: vec![val_reg],
                        immediates: vec![elem_idx],
                        strings: vec![],
                    });
                    self.gen_pattern_bind(p, r);
                    elem_idx += 1;
                }
            }
            Pattern::Range { .. } => {}
        }
    }
}
