//! Unified resolution for `Expr::PathCall`.
//!
//! ```text
//! path_resolution.rs  ── impl Compiler { resolve_path_call }
//!   used by: expr.rs (Expr::PathCall arm)
//! ```
//!
//! A path call like `Foo::bar(args)` can mean many different things —
//! enum-variant construction, a direct call to a top-level fn, a method
//! on a generic type via turbofish, a call through a `use` alias, a
//! sibling-module call, a built-in dispatched by the VM, or a placeholder
//! for an unmonomorphized generic. Previously each rule was an inline
//! `if let Some(target) = self.functions.get(...)` branch and the
//! cascade grew every time a new rule was needed.
//!
//! This module collapses every rule into one helper that returns a
//! `PathResolution`. The caller becomes a flat match on the result.
//!
//! ## Why an ordered candidate list
//!
//! Almost every resolution rule ends in the same lookup:
//! `self.functions.get(&qualified)`. The only thing that changes is
//! *which* qualified name to try. So we build an ordered list of
//! candidate names (most-specific first, fall-back last) and walk it.
//! New rules add an entry to the list, not a new branch.

use crate::ast::TypeAnnotation;

use super::Compiler;

/// Outcome of resolving a `Foo::bar(args)` path call.
pub(crate) enum PathResolution {
    /// `Enum::Variant(...)` constructs an enum value.
    EnumVariant { enum_name: String, variant: String },
    /// Path resolves to a user-defined function at `target`.
    ///
    /// `qualified` is the canonical name we found in `self.functions`.
    /// `is_direct` is `true` when the user-written path is the exact
    /// canonical name — visibility must walk every intermediate module
    /// segment. It is `false` when we reached this target via aliasing
    /// or module-prefix qualification — only the leaf needs checking.
    Function {
        qualified: String,
        target: usize,
        is_direct: bool,
    },
    /// Built-in path handled by the VM via `OpCode::PathCallBuiltin`.
    Builtin,
    /// Path's head is a generic type parameter; the generic body is
    /// never executed directly — emit a panic stub instructing the user
    /// to monomorphize via turbofish.
    GenericPlaceholder {
        type_param: String,
        method_name: String,
    },
    /// No rule resolved the path.
    Unknown,
}

struct Candidate {
    qualified: String,
    is_direct: bool,
}

impl Compiler {
    /// Resolve a `Foo::bar(args)`-style path call. Pure lookup — does not
    /// emit any bytecode. The caller dispatches on the result.
    pub(crate) fn resolve_path_call(
        &self,
        path: &[String],
        turbofish: &Option<Vec<TypeAnnotation>>,
    ) -> PathResolution {
        let type_args_suffix = turbofish_suffix(turbofish);

        // 1. Enum-variant constructor (`Option::Some(...)` etc.).
        if path.len() == 2 {
            let resolved_enum = self.resolve_enum_name(&path[0]);
            if self.enum_defs.contains_key(&resolved_enum) {
                return PathResolution::EnumVariant {
                    enum_name: resolved_enum,
                    variant: path[1].clone(),
                };
            }
        }

        // 2. User functions: walk the candidate list, first hit wins.
        for cand in self.candidate_qualified_names(path, &type_args_suffix) {
            if let Some(&target) = self.functions.get(&cand.qualified) {
                return PathResolution::Function {
                    qualified: cand.qualified,
                    target,
                    is_direct: cand.is_direct,
                };
            }
        }

        // 3. Built-in path (stdlib module fn, constructor, etc.).
        //    Checked AFTER user fns so user code can shadow builtins.
        if super::helpers::is_builtin_path(path) {
            return PathResolution::Builtin;
        }

        // 4. Generic-type-param placeholder for an unmonomorphized call.
        if path.len() == 2
            && self
                .current_generic_params
                .iter()
                .any(|p| p.name == path[0])
        {
            return PathResolution::GenericPlaceholder {
                type_param: path[0].clone(),
                method_name: path[1].clone(),
            };
        }

        PathResolution::Unknown
    }

    /// Resolve the head of an enum path (`Foo` in `Foo::Variant`) through
    /// type/use aliases and the current module prefix.
    fn resolve_enum_name(&self, name: &str) -> String {
        if let Some(aliased) = self.type_aliases.get(name).cloned() {
            return aliased;
        }
        if let Some(aliased) = self.use_aliases.get(name).cloned() {
            return aliased;
        }
        let module_prefix = self.module_stack.join("::");
        if !module_prefix.is_empty() {
            let qualified = format!("{}::{}", module_prefix, name);
            if self.enum_defs.contains_key(&qualified) {
                return qualified;
            }
        }
        name.to_string()
    }

    /// Build the ordered list of candidate qualified names to look up in
    /// `self.functions`. Order matters — the first hit wins.
    fn candidate_qualified_names(&self, path: &[String], type_args_suffix: &str) -> Vec<Candidate> {
        let mut out = Vec::new();
        let module_prefix = self.module_stack.join("::");

        if path.len() == 2 {
            let head = &path[0];
            let tail = &path[1];

            // (a) Direct: `Head<turbofish>::tail`.
            out.push(Candidate {
                qualified: format!("{}{}::{}", head, type_args_suffix, tail),
                is_direct: true,
            });

            // (b) Head resolved through type/use aliases.
            if let Some(rp) = self
                .type_aliases
                .get(head)
                .or_else(|| self.use_aliases.get(head))
            {
                out.push(Candidate {
                    qualified: format!("{}{}::{}", rp, type_args_suffix, tail),
                    is_direct: false,
                });
            }

            // (c) Full path resolved through use_aliases (pub-use re-exports).
            let full = format!("{}::{}", head, tail);
            if let Some(aliased) = self.use_aliases.get(&full) {
                out.push(Candidate {
                    qualified: aliased.clone(),
                    is_direct: false,
                });
            }

            // (d) Module-qualified (sibling-module call).
            if !module_prefix.is_empty() {
                out.push(Candidate {
                    qualified: format!("{}::{}{}::{}", module_prefix, head, type_args_suffix, tail),
                    is_direct: false,
                });
            }
        } else if path.len() >= 3 {
            // Multi-segment paths: try direct then module-qualified.
            let joined = path.join("::");
            out.push(Candidate {
                qualified: joined.clone(),
                is_direct: true,
            });
            if !module_prefix.is_empty() {
                out.push(Candidate {
                    qualified: format!("{}::{}", module_prefix, joined),
                    is_direct: false,
                });
            }
        }

        out
    }
}

fn turbofish_suffix(turbofish: &Option<Vec<TypeAnnotation>>) -> String {
    turbofish
        .as_ref()
        .map(|ts| {
            let names: Vec<&str> = ts.iter().map(|ta| ta.name()).collect();
            format!("<{}>", names.join(", "))
        })
        .unwrap_or_default()
}
