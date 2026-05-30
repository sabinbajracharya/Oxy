//! Expression lowering for `IrGen` — part of the AST → register IR
//! lowering pass. See `mod.rs` for the `IrGen` struct and state.

use super::*;

impl IrGen {
    pub(super) fn gen_expr(&mut self, expr: &Expr) -> Reg {
        match expr {
            Expr::IntLiteral(n, ..) => {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstInt(r, *n));
                r
            }
            Expr::FloatLiteral(n, ..) => {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstFloat(r, *n));
                r
            }
            Expr::BoolLiteral(b, ..) => {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstBool(r, *b));
                r
            }
            Expr::StringLiteral(s, ..) => {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstString(r, s.clone()));
                r
            }
            Expr::CharLiteral(c, ..) => {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstChar(r, *c));
                r
            }
            Expr::Ident(name, ..) => {
                if let Some(slot) = self.lookup_local(name) {
                    let r = self.alloc_reg();
                    self.emit(IrOp::LoadLocal(r, slot));
                    r
                } else if let Some(const_val) = self.global_consts.get(name).cloned() {
                    // Inline const value at use site
                    self.gen_expr(&const_val)
                } else if let Some(enum_name) = self.variant_to_enum.get(name).cloned() {
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_const_enum_variant",
                        args: vec![],
                        immediates: vec![],
                        strings: vec![enum_name, name.clone()],
                    });
                    r
                } else if self
                    .unit_structs
                    .contains(&self.resolve_module_path(&[name.clone()]).join("::"))
                {
                    // A bare reference to a unit struct (`struct Thing;` then
                    // `let t = Thing;`) constructs an empty struct value so
                    // method dispatch (`t.method()`) resolves against `Thing`.
                    let resolved = self.resolve_module_path(&[name.clone()]).join("::");
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_struct_init",
                        args: vec![],
                        immediates: vec![0],
                        strings: vec![resolved, String::new()],
                    });
                    r
                } else {
                    // A bare identifier in value position that is neither a
                    // local, a const, nor an enum variant is a reference to a
                    // named function (e.g. `apply(square, 5)` or `vec![square,
                    // neg]`). The type checker has already validated the name,
                    // so resolve it the same way the Call path does and build a
                    // `Value::Function` via `oxy_push_named_fn` so it can be
                    // invoked through the unified `oxy_call_closure` path.
                    let resolved = self.resolve_callable_name(name);
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_push_named_fn",
                        args: vec![],
                        immediates: vec![],
                        strings: vec![resolved],
                    });
                    r
                }
            }
            Expr::Block(block) => self.gen_block_stmts(block).unwrap_or_else(|| {
                let r = self.alloc_reg();
                self.emit(IrOp::ConstUnit(r));
                r
            }),
            Expr::BinaryOp {
                left, op, right, ..
            } => {
                // `&&` / `||` must short-circuit: the right operand is only
                // evaluated when the left doesn't already decide the result.
                // Lower with branching rather than an eager And/Or op so side
                // effects in the right operand are skipped.
                if matches!(op, BinOp::And | BinOp::Or) {
                    return self.gen_short_circuit(*op, left, right);
                }
                let lhs = self.gen_expr(left);
                let rhs = self.gen_expr(right);
                let r = self.alloc_reg();
                match op {
                    BinOp::Add => self.emit(IrOp::Add(r, lhs, rhs)),
                    BinOp::Sub => self.emit(IrOp::Sub(r, lhs, rhs)),
                    BinOp::Mul => self.emit(IrOp::Mul(r, lhs, rhs)),
                    BinOp::Div => self.emit(IrOp::Div(r, lhs, rhs)),
                    BinOp::Mod => self.emit(IrOp::Rem(r, lhs, rhs)),
                    BinOp::Eq => self.emit(IrOp::Eq(r, lhs, rhs)),
                    BinOp::NotEq => self.emit(IrOp::Neq(r, lhs, rhs)),
                    BinOp::Lt => self.emit(IrOp::Lt(r, lhs, rhs)),
                    BinOp::Gt => self.emit(IrOp::Gt(r, lhs, rhs)),
                    BinOp::LtEq => self.emit(IrOp::Le(r, lhs, rhs)),
                    BinOp::GtEq => self.emit(IrOp::Ge(r, lhs, rhs)),
                    BinOp::And => self.emit(IrOp::And(r, lhs, rhs)),
                    BinOp::Or => self.emit(IrOp::Or(r, lhs, rhs)),
                    BinOp::BitAnd => self.emit(IrOp::BitAnd(r, lhs, rhs)),
                    BinOp::BitOr => self.emit(IrOp::BitOr(r, lhs, rhs)),
                    BinOp::BitXor => self.emit(IrOp::BitXor(r, lhs, rhs)),
                    BinOp::Shl => self.emit(IrOp::Shl(r, lhs, rhs)),
                    BinOp::Shr => self.emit(IrOp::Shr(r, lhs, rhs)),
                }
                r
            }
            Expr::UnaryOp { op, expr, .. } => {
                let val = self.gen_expr(expr);
                let r = self.alloc_reg();
                match op {
                    UnaryOp::Neg => self.emit(IrOp::Neg(r, val)),
                    UnaryOp::Not => self.emit(IrOp::Not(r, val)),
                    UnaryOp::BitNot => self.emit(IrOp::BitNot(r, val)),
                }
                r
            }
            Expr::Call {
                callee,
                args,
                turbofish,
                ..
            } => {
                // Build fname for special-form detection (enum constructors,
                // spawn/sleep/select). Regular calls go through the unified
                // oxy_call_closure path regardless of whether the callee is a
                // named function, closure, or parameter.
                let fname = match callee.as_ref() {
                    Expr::Ident(name, ..) => self.resolve_callable_name(name),
                    Expr::Path { segments, .. } => {
                        let resolved = self.resolve_module_path(segments);
                        self.resolve_fn_alias(&resolved.join("::"))
                    }
                    _ => String::new(),
                };
                // A turbofish on a generic function selects a monomorphized
                // copy (emitted on demand) whose body has the type parameters
                // substituted, so `T::method()` resolves to the concrete impl.
                let fname = match turbofish {
                    Some(tf) => self.monomorphize_if_generic(&fname, tf),
                    None => fname,
                };

                // Evaluate arguments first — the callee register depends on
                // whether it's a local or a named reference.
                let mut arg_regs = Vec::new();
                for a in args {
                    arg_regs.push(self.gen_expr(a));
                }

                // Route enum variant constructors.
                let enum_ctor = 'ctor: {
                    if let Some(enum_name) = self.variant_to_enum.get(&fname) {
                        break 'ctor Some((enum_name.clone(), fname.clone()));
                    }
                    if let Some((enum_name, variant)) = fname.rsplit_once("::") {
                        if self
                            .variant_to_enum
                            .get(variant)
                            .map_or(false, |e| e == enum_name)
                        {
                            break 'ctor Some((enum_name.to_string(), variant.to_string()));
                        }
                    }
                    None
                };
                if let Some((enum_name, variant)) = enum_ctor {
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_make_enum_variant",
                        args: arg_regs,
                        immediates: vec![args.len()],
                        strings: vec![enum_name, variant],
                    });
                    return r;
                }

                // Route tuple-struct constructors: `WrappedInt(17)` builds a
                // `Value::Struct` with positional field names "0", "1", … so
                // that `.0` access (oxy_field_access reads fields["0"]) works.
                // Named-field structs use Expr::StructInit instead; tuple
                // structs reach us here because they are called like functions.
                let tuple_ctor: Option<String> = if self.tuple_structs.contains_key(&fname) {
                    Some(fname.clone())
                } else if let Expr::Ident(name, ..) = callee.as_ref() {
                    let resolved = self.resolve_module_path(&[name.clone()]).join("::");
                    self.tuple_structs
                        .contains_key(&resolved)
                        .then_some(resolved)
                } else {
                    None
                };
                if let Some(struct_name) = tuple_ctor {
                    let names_joined = (0..args.len())
                        .map(|i| i.to_string())
                        .collect::<Vec<_>>()
                        .join("\0");
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_struct_init",
                        args: arg_regs,
                        immediates: vec![args.len()],
                        strings: vec![struct_name, names_joined],
                    });
                    return r;
                }

                // Route spawn / sleep / select to their FFI functions.
                if let Some((ffi_func, immediates)) = match fname.as_str() {
                    "spawn" => Some(("oxy_spawn_ffi", vec![])),
                    "sleep" => Some(("oxy_sleep_ffi", vec![])),
                    "select" => Some(("oxy_select_ffi", vec![args.len()])),
                    _ => None,
                } {
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: ffi_func,
                        args: arg_regs,
                        immediates,
                        strings: vec![],
                    });
                    return r;
                }

                // Produce a register holding the callee as a Value::Function.
                // Locals already hold a function value; named functions need
                // oxy_push_named_fn to create one. Inline expressions (closures,
                // parenthesized exprs) are generated directly.
                let callee_reg = if let Expr::Ident(name, _) = callee.as_ref() {
                    if let Some(slot) = self.lookup_local(name) {
                        let r = self.alloc_reg();
                        self.emit(IrOp::LoadLocalRaw(r, slot));
                        r
                    } else {
                        let r = self.alloc_reg();
                        self.emit(IrOp::CallBuiltin {
                            result: r,
                            func: "oxy_push_named_fn",
                            args: vec![],
                            immediates: vec![],
                            strings: vec![fname],
                        });
                        r
                    }
                } else if matches!(callee.as_ref(), Expr::Path { .. }) {
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_push_named_fn",
                        args: vec![],
                        immediates: vec![],
                        strings: vec![fname],
                    });
                    r
                } else {
                    // Inline expression (closure, parenthesized expr, etc.).
                    // Generate it directly — it produces a Value::Function on the stack.
                    self.gen_expr(callee)
                };

                // Unified call: everything goes through oxy_call_closure.
                let mut all_regs = vec![callee_reg];
                all_regs.extend(arg_regs);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_call_closure",
                    args: all_regs,
                    immediates: vec![args.len()],
                    strings: vec![],
                });
                r
            }
            Expr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                // For local-variable receivers, use LoadLocalRaw to preserve
                // Cell wrapping so mutations through method calls are visible.
                //
                // A `mut self` method mutates its receiver in place (Oxy has no
                // ownership, so `mut self` behaves like `&mut self`). Structs are
                // value types, so a plain copy of the receiver would discard those
                // mutations once the method returns. Promote a mutable local
                // receiver to a shared `Value::Cell` first (idempotent per slot via
                // `celled_slots`, exactly like closure capture): the method then
                // stores the updated struct back through the cell, which the caller
                // observes. Non-`mut` receivers and non-mutating methods are
                // unaffected — they simply never write through the shared cell.
                let obj_reg = if let Expr::Ident(name, ..) = object.as_ref() {
                    if let Some(slot) = self.lookup_local(name) {
                        if self.mut_slots.contains(&slot) && self.celled_slots.insert(slot) {
                            self.emit(IrOp::MakeCell(slot));
                        }
                        let r = self.alloc_reg();
                        self.emit(IrOp::LoadLocalRaw(r, slot));
                        r
                    } else {
                        self.gen_expr(object)
                    }
                } else {
                    self.gen_expr(object)
                };
                let mut arg_regs = vec![obj_reg];
                for a in args {
                    arg_regs.push(self.gen_expr(a));
                }
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_method_call",
                    args: arg_regs,
                    immediates: vec![args.len()],
                    strings: vec![method.clone()],
                });
                r
            }
            Expr::If {
                condition,
                then_block,
                else_block,
                ..
            } => self.gen_if(condition, then_block, else_block.as_deref()),
            Expr::IfLet {
                pattern,
                expr,
                then_block,
                else_block,
                guard,
                ..
            } => {
                if let Some(guard_expr) = guard {
                    self.gen_if_let_guarded(
                        pattern,
                        expr,
                        guard_expr,
                        then_block,
                        else_block.as_deref(),
                    )
                } else {
                    self.gen_if_let(pattern, expr, then_block, else_block.as_deref())
                }
            }
            Expr::Match { expr, arms, .. } => self.gen_match(expr, arms),
            Expr::StructInit {
                name, fields, base, ..
            } => {
                let mut arg_regs = Vec::new();
                let mut field_names = Vec::new();
                for (fname, val) in fields {
                    arg_regs.push(self.gen_expr(val));
                    field_names.push(fname.clone());
                }
                // Join field names with \0 for the FFI to parse.
                let names_joined = field_names.join("\0");
                // `Self { .. }` inside a method body builds a value of the impl's
                // concrete type, so dispatch (`v.method()`) resolves against it.
                // `current_self_type` is already the fully-qualified dispatch key
                // (the same string used to name the methods), so use it directly.
                // Otherwise resolve the struct name through use_aliases and module
                // context so module-level types get their full path (e.g.
                // "counter::Counter" instead of just "Counter").
                let resolved_name = if name == "Self" {
                    match &self.current_self_type {
                        Some(t) => t.clone(),
                        None => self.resolve_module_path(&[name.clone()]).join("::"),
                    }
                } else {
                    self.resolve_module_path(&[name.clone()]).join("::")
                };
                if let Some(base_expr) = base {
                    // Struct update: Point { x: 1, ..base }
                    let base_reg = self.gen_expr(base_expr);
                    arg_regs.push(base_reg);
                    let r = self.alloc_reg();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_struct_update",
                        args: arg_regs,
                        immediates: vec![fields.len()],
                        strings: vec![names_joined],
                    });
                    r
                } else {
                    // Check if this is a struct-style enum variant constructor
                    // (e.g. Message::Move { x: 10, y: 20 }). If so, route to
                    // oxy_make_enum_variant to produce Value::EnumVariant instead
                    // of Value::Struct.
                    let enum_ctor: Option<(String, String)> = 'ctor: {
                        if let Some((enum_name, variant)) = resolved_name.rsplit_once("::") {
                            if self
                                .variant_to_enum
                                .get(variant)
                                .map_or(false, |e| e == enum_name)
                            {
                                break 'ctor Some((enum_name.to_string(), variant.to_string()));
                            }
                        }
                        None
                    };
                    let r = self.alloc_reg();
                    if let Some((enum_name, variant)) = enum_ctor {
                        self.emit(IrOp::CallBuiltin {
                            result: r,
                            func: "oxy_make_enum_variant",
                            args: arg_regs,
                            immediates: vec![fields.len()],
                            strings: vec![enum_name, variant],
                        });
                    } else {
                        self.emit(IrOp::CallBuiltin {
                            result: r,
                            func: "oxy_struct_init",
                            args: arg_regs,
                            immediates: vec![fields.len()],
                            strings: vec![resolved_name, names_joined],
                        });
                    }
                    r
                }
            }
            Expr::FieldAccess { object, field, .. } => {
                let obj = self.gen_expr(object);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_field_access",
                    args: vec![obj],
                    immediates: vec![],
                    strings: vec![field.clone()],
                });
                r
            }
            Expr::PathCall {
                path,
                turbofish: _,
                args,
                ..
            } => {
                // Inside a monomorphized body, a leading type parameter resolves
                // to its concrete type so `T::zero()` → `int::zero()`.
                let path: Vec<String> = match path.split_first() {
                    Some((first, rest)) if self.type_subst.contains_key(first) => {
                        let mut p = vec![self.type_subst[first].clone()];
                        p.extend(rest.iter().cloned());
                        p
                    }
                    _ => path.clone(),
                };
                let resolved_path = self.resolve_module_path(&path);
                let resolved_path = self.resolve_type_alias_in_path(&resolved_path);
                // A path call from inside a module to a *sibling* top-level
                // module (e.g. `crate_lib::get_value()` from within `other_mod`)
                // must not get the current module prefix prepended. If
                // module-prefixing produced an unknown function but the path as
                // written names a known one, use the written path instead.
                let final_segments = if self.fn_names.contains(&resolved_path.join("::")) {
                    resolved_path
                } else if self.fn_names.contains(&path.join("::")) {
                    path.clone()
                } else {
                    resolved_path
                };
                let mut arg_regs = Vec::new();
                for a in args {
                    arg_regs.push(self.gen_expr(a));
                }
                // Route enum-variant constructors (e.g. `Color::Red(1)` or the
                // module-qualified `mymath::Operation::Add(3, 4)`) to
                // oxy_make_enum_variant with the fully-qualified enum name,
                // mirroring the Expr::Call path. The last segment is the variant
                // and the preceding segments are the enum's qualified name; a
                // match against `variant_to_enum` (which stores the same
                // qualified name the pattern side uses) confirms it's really a
                // constructor before we commit. Without this, a multi-segment
                // path would emit oxy_path_call_builtin, whose positional
                // fallback only recognizes the bare 2-segment `Enum::Variant`
                // form and silently mis-handles module-qualified variants.
                if final_segments.len() >= 2 {
                    let variant = final_segments[final_segments.len() - 1].clone();
                    let enum_name = final_segments[..final_segments.len() - 1].join("::");
                    if self
                        .variant_to_enum
                        .get(&variant)
                        .map_or(false, |e| e == &enum_name)
                    {
                        let r = self.alloc_reg();
                        self.emit(IrOp::CallBuiltin {
                            result: r,
                            func: "oxy_make_enum_variant",
                            args: arg_regs,
                            immediates: vec![args.len()],
                            strings: vec![enum_name, variant],
                        });
                        return r;
                    }
                }
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_path_call_builtin",
                    args: arg_regs,
                    immediates: vec![args.len()],
                    strings: vec![final_segments.join("\0")],
                });
                r
            }
            Expr::Array { elements, .. } => {
                let mut regs = Vec::new();
                for e in elements {
                    regs.push(self.gen_expr(e));
                }
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_make_array",
                    args: regs,
                    immediates: vec![elements.len()],
                    strings: vec![],
                });
                r
            }
            Expr::Tuple { elements, .. } => {
                let mut regs = Vec::new();
                for e in elements {
                    regs.push(self.gen_expr(e));
                }
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_make_tuple",
                    args: regs,
                    immediates: vec![elements.len()],
                    strings: vec![],
                });
                r
            }
            Expr::Index { object, index, .. } => {
                let obj = self.gen_expr(object);
                let idx = self.gen_expr(index);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_vec_index",
                    args: vec![obj, idx],
                    immediates: vec![],
                    strings: vec![],
                });
                r
            }
            Expr::Try { expr, .. } => {
                let val = self.gen_expr(expr);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_try_pop",
                    args: vec![val],
                    immediates: vec![],
                    strings: vec![],
                });
                // oxy_try_pop calls set_error on Err/None, so CheckError detects it.
                let err = self.alloc_reg();
                self.emit(IrOp::CheckError(err));
                let continue_id = self.alloc_block();
                let return_id = self.alloc_block();
                self.terminate(Terminator::Branch {
                    cond: err,
                    then_block: return_id,
                    else_block: continue_id,
                });
                // Return block: Halt. oxy_error_discriminant returns 2 (set_error
                // was called), disc=2 with empty error_msg signals ? propagation.
                self.start_block(return_id);
                self.terminate(Terminator::Halt);
                // Continue block: r holds the unwrapped value.
                self.start_block(continue_id);
                r
            }
            Expr::FString { parts, .. } => {
                let mut regs = Vec::new();
                for part in parts {
                    match part {
                        crate::ast::FStringPart::Literal(s) => {
                            let r = self.alloc_reg();
                            self.emit(IrOp::ConstString(r, s.clone()));
                            regs.push(r);
                        }
                        crate::ast::FStringPart::Expr(e) => {
                            regs.push(self.gen_expr(e));
                        }
                    }
                }
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_fstring_concat",
                    args: regs,
                    immediates: vec![parts.len()],
                    strings: vec![],
                });
                r
            }
            Expr::Assign { target, value, .. } => {
                let val_reg = self.gen_expr(value);
                self.gen_store_lvalue(target, val_reg);
                val_reg
            }
            Expr::CompoundAssign {
                target, op, value, ..
            } => {
                let val_reg = self.gen_expr(value);
                let target_reg = self.gen_expr(target);
                let r = self.alloc_reg();
                match op {
                    BinOp::Add => self.emit(IrOp::Add(r, target_reg, val_reg)),
                    BinOp::Sub => self.emit(IrOp::Sub(r, target_reg, val_reg)),
                    BinOp::Mul => self.emit(IrOp::Mul(r, target_reg, val_reg)),
                    BinOp::Div => self.emit(IrOp::Div(r, target_reg, val_reg)),
                    BinOp::Mod => self.emit(IrOp::Rem(r, target_reg, val_reg)),
                    BinOp::BitAnd => self.emit(IrOp::BitAnd(r, target_reg, val_reg)),
                    BinOp::BitOr => self.emit(IrOp::BitOr(r, target_reg, val_reg)),
                    BinOp::BitXor => self.emit(IrOp::BitXor(r, target_reg, val_reg)),
                    BinOp::Shl => self.emit(IrOp::Shl(r, target_reg, val_reg)),
                    BinOp::Shr => self.emit(IrOp::Shr(r, target_reg, val_reg)),
                    _ => {
                        self.emit(IrOp::Copy(r, val_reg));
                    }
                }
                if let Expr::Ident(name, ..) = target.as_ref() {
                    if let Some(slot) = self.lookup_local(name) {
                        let coerced = if self.local_types.get(&slot).map_or(false, |t| t == "byte")
                        {
                            let cr = self.alloc_reg();
                            self.emit(IrOp::CallBuiltin {
                                result: cr,
                                func: "oxy_cast_byte",
                                args: vec![r],
                                immediates: vec![],
                                strings: vec![],
                            });
                            cr
                        } else {
                            r
                        };
                        self.emit(IrOp::StoreLocal(slot, coerced));
                    }
                } else {
                    // Field / index targets: write the recomputed value back
                    // through the lvalue chain (same root as plain assignment).
                    self.gen_store_lvalue(target, r);
                }
                r
            }
            Expr::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                let start_reg = start.as_ref().map(|s| self.gen_expr(s)).unwrap_or_else(|| {
                    let r = self.alloc_reg();
                    self.emit(IrOp::ConstInt(r, 0));
                    r
                });
                let end_reg = end.as_ref().map(|e| self.gen_expr(e)).unwrap_or_else(|| {
                    let r = self.alloc_reg();
                    // i64::MAX sentinel for unbounded — avoids conflicting with
                    // legitimate -1 as a range endpoint.
                    self.emit(IrOp::ConstInt(r, i64::MAX));
                    r
                });
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_make_range",
                    args: vec![start_reg, end_reg],
                    immediates: vec![*inclusive as usize],
                    strings: vec![],
                });
                r
            }
            Expr::Closure {
                params,
                body,
                is_async,
                ..
            } => self.gen_closure(params, body, *is_async, false),
            Expr::Path { segments, .. } => {
                // A multi-segment path is a unit enum variant (Color::Red,
                // module::Color::Red) only when its last segment is a known
                // variant of the named enum. Otherwise it's a module-level
                // constant such as `math::PI` — route those to oxy_module_const
                // rather than fabricating a bogus enum variant.
                let resolved = self.resolve_module_path(segments);
                let resolved = self.resolve_type_alias_in_path(&resolved);
                let variant = resolved.last().cloned().unwrap_or_default();
                let r = self.alloc_reg();
                if resolved.len() > 1 {
                    let enum_name = resolved[..resolved.len() - 1].join("::");
                    let is_enum_variant = self
                        .variant_to_enum
                        .get(&variant)
                        .map_or(false, |e| e == &enum_name);
                    if is_enum_variant {
                        self.emit(IrOp::CallBuiltin {
                            result: r,
                            func: "oxy_const_enum_variant",
                            args: vec![],
                            immediates: vec![],
                            strings: vec![enum_name, variant],
                        });
                    } else {
                        self.emit(IrOp::CallBuiltin {
                            result: r,
                            func: "oxy_module_const",
                            args: vec![],
                            immediates: vec![],
                            strings: vec![resolved.join("::")],
                        });
                    }
                } else {
                    // Single-segment variant resolved via the enum map.
                    let enum_name = self
                        .variant_to_enum
                        .get(&variant)
                        .cloned()
                        .unwrap_or_default();
                    self.emit(IrOp::CallBuiltin {
                        result: r,
                        func: "oxy_const_enum_variant",
                        args: vec![],
                        immediates: vec![],
                        strings: vec![enum_name, variant],
                    });
                }
                r
            }
            Expr::SelfRef(..) => {
                // self parameter — load from local slot 0
                let r = self.alloc_reg();
                self.emit(IrOp::LoadLocal(r, 0));
                r
            }
            Expr::Grouped(inner, _) => self.gen_expr(inner),
            Expr::MacroCall { name, args, .. } => {
                let mut arg_regs = Vec::new();
                for a in args {
                    arg_regs.push(self.gen_expr(a));
                }
                let r = self.alloc_reg();
                let (func, strings, extra_immediates) = match name.as_str() {
                    "println" => ("oxy_println_val", vec![], vec![args.len()]),
                    "print" => ("oxy_print_val", vec![], vec![args.len()]),
                    "format" => ("oxy_format", vec![], vec![args.len()]),
                    "vec" => ("oxy_make_array", vec![], vec![args.len()]),
                    "dbg" => ("oxy_dbg", vec![], vec![args.len()]),
                    "panic" => ("oxy_panic", vec![], vec![]),
                    _ => (
                        "oxy_path_call_builtin",
                        vec![name.clone()],
                        vec![args.len()],
                    ),
                };
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func,
                    args: arg_regs,
                    immediates: extra_immediates,
                    strings,
                });
                r
            }
            Expr::Repeat { value, count, .. } => {
                let val_reg = self.gen_expr(value);
                let count_reg = self.gen_expr(count);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_make_repeat",
                    args: vec![val_reg, count_reg],
                    immediates: vec![],
                    strings: vec![],
                });
                r
            }
            Expr::AsyncBlock { body, .. } => {
                // Generate as a closure-like async function
                let params: Vec<ClosureParam> = Vec::new();
                let body_expr = Expr::Block(body.clone());
                self.gen_closure(&params, &body_expr, true, true)
            }
            Expr::Await { expr, .. } => {
                let val = self.gen_expr(expr);
                let r = self.alloc_reg();
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func: "oxy_await_ffi",
                    args: vec![val],
                    immediates: vec![],
                    strings: vec![],
                });
                r
            }
            Expr::Return { value, .. } => {
                let reg = value.as_ref().map(|v| self.gen_expr(v)).unwrap_or_else(|| {
                    let r = self.alloc_reg();
                    self.emit(IrOp::ConstUnit(r));
                    r
                });
                self.terminate(Terminator::Return(reg));
                reg
            }
            Expr::As {
                expr, type_name, ..
            } => {
                let val = self.gen_expr(expr);
                let r = self.alloc_reg();
                let func = match type_name.as_str() {
                    "int" => "oxy_cast_int",
                    "byte" => "oxy_cast_byte",
                    "float" => "oxy_cast_float",
                    "char" => "oxy_cast_to_char",
                    _ => "oxy_cast_int",
                };
                self.emit(IrOp::CallBuiltin {
                    result: r,
                    func,
                    args: vec![val],
                    immediates: vec![],
                    strings: vec![],
                });
                r
            } // Unreachable: all Expr variants are handled above.
        }
    }

    // ── Control flow helpers ───────────────────────────────────────────

    /// Short-circuiting `&&` / `||`. Equivalent to `if a { b } else { false }`
    /// for `&&` and `if a { true } else { b }` for `||`, so the right operand
    /// `b` is evaluated only in the branch that needs it. Uses the same
    /// Phi-isolation continuation trick as `gen_if` so the result reg is defined
    /// in a fresh block and nested control flow in `b` is handled correctly.
    pub(super) fn gen_short_circuit(&mut self, op: BinOp, left: &Expr, right: &Expr) -> Reg {
        let lhs = self.gen_expr(left);
        let then_id = self.alloc_block();
        let else_id = self.alloc_block();
        let merge_id = self.alloc_block();
        self.terminate(Terminator::Branch {
            cond: lhs,
            then_block: then_id,
            else_block: else_id,
        });

        self.start_block(then_id);
        let then_reg = if matches!(op, BinOp::And) {
            self.gen_expr(right)
        } else {
            let r = self.alloc_reg();
            self.emit(IrOp::ConstBool(r, true));
            r
        };
        if self.current.blocks[self.current_block]
            .terminator
            .is_default()
        {
            self.terminate(Terminator::Jump(merge_id));
        }

        self.start_block(else_id);
        let else_reg = if matches!(op, BinOp::And) {
            let r = self.alloc_reg();
            self.emit(IrOp::ConstBool(r, false));
            r
        } else {
            self.gen_expr(right)
        };
        if self.current.blocks[self.current_block]
            .terminator
            .is_default()
        {
            self.terminate(Terminator::Jump(merge_id));
        }

        self.start_block(merge_id);
        let r = self.alloc_reg();
        self.emit(IrOp::Phi(r, then_reg, else_reg));
        let phi_temp = self.alloc_local("__phi_tmp");
        self.emit(IrOp::StoreLocal(phi_temp, r));
        let cont = self.alloc_block();
        self.terminate(Terminator::Jump(cont));
        self.start_block(cont);
        let r2 = self.alloc_reg();
        self.emit(IrOp::LoadLocal(r2, phi_temp));
        r2
    }
}
