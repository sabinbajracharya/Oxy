//! Type inference for leaf/value expressions: identifiers, `self`, and
//! bare paths.
//!
//! Part of `check_expr` — see that module for the `infer_expr` dispatcher.

use super::*;

impl TypeChecker {
    pub(super) fn infer_ident(&mut self, name: &str, _span: &Span) -> Result<TypeInfo, FerriError> {
        if let Some(ty) = self.env.borrow().get(name) {
            return Ok(ty);
        }
        // An ident that names a function used as a value (e.g.
        // `apply(square, 5)`) must infer to a fn-pointer type, NOT
        // the function's return type. Previously this branch
        // returned the return type alone, so `square` was typed as
        // `int` and the apply(fn(int) -> int, …) signature
        // rejected it.
        if let Some(ret) = self.fn_return_types.get(name) {
            let params = self.fn_param_types.get(name).cloned().unwrap_or_default();
            return Ok(TypeInfo::Function {
                params,
                ret: Box::new(ret.clone()),
            });
        }
        // Try module-qualified function name
        {
            let module_prefix = self.module_stack.join("::");
            if !module_prefix.is_empty() {
                let qualified = format!("{}::{}", module_prefix, name);
                if let Some(ret) = self.fn_return_types.get(&qualified) {
                    let params = self
                        .fn_param_types
                        .get(&qualified)
                        .cloned()
                        .unwrap_or_default();
                    return Ok(TypeInfo::Function {
                        params,
                        ret: Box::new(ret.clone()),
                    });
                }
            }
        }
        // Try use_aliases -> struct_defs
        if let Some(resolved) = self.use_aliases.get(name) {
            if self.struct_defs.contains_key(resolved) {
                return Ok(TypeInfo::user_struct(resolved.clone()));
            }
        }
        // Try module-qualified struct name
        let resolved = self.resolve_struct_name(name);
        if self.struct_defs.contains_key(&resolved) {
            return Ok(TypeInfo::user_struct(resolved));
        }
        // A name that matches any known symbol — a function (possibly
        // reached via `use`/glob/`super`/re-export, so matched by short
        // name), struct, enum, enum variant, or `const` — is a value
        // reference, not an undefined variable. Function/path resolution
        // for callees happens in `Expr::Call`; this guard only prevents
        // the value-position fallback from flagging a legitimate name.
        if self.name_matches_known_symbol(name) {
            return Ok(TypeInfo::Unknown);
        }
        // Nothing resolved — this is a genuine undefined variable.
        // Offer a "did you mean" hint from names in scope plus
        // top-level functions, mirroring the old interpreter's DX.
        let mut candidates: Vec<String> = Vec::new();
        self.env.borrow().collect_names(&mut candidates);
        candidates.extend(self.fn_return_types.keys().cloned());
        let suggestion = crate::errors::suggest_name(name, candidates.iter().map(|s| s.as_str()));
        let message = match suggestion {
            Some(s) => format!("undefined variable '{name}'; did you mean '{s}'?"),
            None => format!("undefined variable '{name}'"),
        };
        Err(FerriError::TypeError {
            message,
            line: _span.line,
            column: _span.column,
        })
    }

    pub(super) fn infer_self_ref(&mut self) -> Result<TypeInfo, FerriError> {
        if let Some(ref impl_type) = self.current_impl_type {
            Ok(TypeInfo::user_struct(impl_type.clone()))
        } else {
            Ok(TypeInfo::Unknown)
        }
    }

    pub(super) fn infer_path(&mut self, segments: &[String]) -> Result<TypeInfo, FerriError> {
        let qualified = segments.join("::");
        if let Some(ret) = self.fn_return_types.get(&qualified) {
            return Ok(ret.clone());
        }
        if self.struct_defs.contains_key(&qualified) {
            return Ok(TypeInfo::user_struct(qualified));
        }
        // Try through use_aliases for the first segment
        if segments.len() == 2 {
            if let Some(resolved) = self.use_aliases.get(&segments[0]) {
                let full = format!("{}::{}", resolved, segments[1]);
                if self.struct_defs.contains_key(&full) {
                    return Ok(TypeInfo::user_struct(full));
                }
            }
        }
        Ok(TypeInfo::Unknown)
    }

    /// Whether `name` (a bare identifier) corresponds to any program-level
    /// symbol: a function, struct, enum, enum variant, `const`, or re-export.
    /// Functions/types may be reached through `use`/glob/`super`/re-export, so
    /// they are matched by short name (the last `::` segment) — being permissive
    /// here only risks *not* flagging a typo that happens to collide with a real
    /// symbol name, never a false "undefined" on legitimate code.
    fn name_matches_known_symbol(&self, name: &str) -> bool {
        if self.enum_variant_names.contains(name) || self.const_names.contains(name) {
            return true;
        }
        let suffix = format!("::{name}");
        let short_match = |k: &String| k == name || k.ends_with(&suffix);
        self.fn_return_types.keys().any(short_match)
            || self.struct_defs.keys().any(short_match)
            || self.enum_defs.iter().any(short_match)
            || self.reexports.keys().any(short_match)
    }
}
