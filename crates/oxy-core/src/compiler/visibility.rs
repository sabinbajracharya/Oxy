//! Visibility checking for module-level items, path segments, and struct fields.
//!
//! ```text
//! visibility.rs  ── impl Compiler { is_visible, check_path_visible_with_leaf, ... }
//!   uses: mod.rs (reads Compiler fields: functions, pub_vis, module_names, ...)
//!   used by: mod.rs (compile_use, preresolve_uses), expr.rs (PathCall, StructInit)
//! ```

use super::Compiler;
use crate::ast::*;
use crate::errors::FerriError;

impl Compiler {
    /// Check whether a qualified path refers to a publicly visible item.
    /// Returns true if the item exists and is pub, or if the item is not tracked
    /// (allowing built-in/stdlib paths through).
    pub(crate) fn is_visible(&self, qualified: &str) -> bool {
        let in_fns = self.functions.contains_key(qualified);
        let in_structs = self.struct_defs.contains_key(qualified);
        let in_enums = self.enum_defs.contains_key(qualified);
        let in_modules = self.module_names.contains(qualified);
        if !in_fns && !in_structs && !in_enums && !in_modules {
            // Check if it's a built-in path (e.g., "std::db::open").
            let segments: Vec<String> = qualified.split("::").map(|s| s.to_string()).collect();
            if super::helpers::is_builtin_path(&segments) {
                return true;
            }
            return false; // unknown name — reject
        }
        if let Some(vis) = self.pub_vis.get(qualified) {
            match vis {
                Visibility::Pub | Visibility::PubCrate => true,
                Visibility::PubSuper => {
                    // pub(super): visible to the parent module and all its descendants.
                    let defining_module = qualified.rsplit_once("::").map(|(m, _)| m).unwrap_or("");
                    let target_parent = defining_module
                        .rsplit_once("::")
                        .map(|(p, _)| p)
                        .unwrap_or("");
                    let current_module = self.module_stack.join("::");
                    if target_parent.is_empty() {
                        // At root level: pub(super) is equivalent to pub(crate)
                        true
                    } else {
                        current_module == target_parent
                            || current_module.starts_with(&format!("{}::", target_parent))
                    }
                }
                Visibility::Private => {
                    // Allow if parent is not a module (e.g. field on top-level struct)
                    let parent = qualified.rsplit_once("::").map(|(p, _)| p).unwrap_or("");
                    if parent.is_empty() || !self.module_names.contains(parent) {
                        return true;
                    }
                    // Allow access from within the same defining module
                    let current_module = self.module_stack.join("::");
                    parent == current_module
                }
            }
        } else {
            // Not in pub_vis — item exists but is private.
            // Allow if parent is not a module (e.g. method on top-level struct)
            // or if it's a top-level item (no parent module boundary).
            let parent = qualified.rsplit_once("::").map(|(p, _)| p).unwrap_or("");
            if parent.is_empty() || !self.module_names.contains(parent) {
                return true;
            }
            // Allow access from within the same defining module
            let current_module = self.module_stack.join("::");
            if parent == current_module {
                return true;
            }
            false
        }
    }

    /// Check that every segment of a path is visible: intermediate modules
    /// and the final item (function, struct, enum). `leaf_qualified` is the
    /// canonical name we resolved to — it may include type args
    /// (`Pair<i64, i64>::make`) that aren't in the raw path segments.
    pub(crate) fn check_path_visible_with_leaf(
        &self,
        path: &[String],
        leaf_qualified: &str,
        span: crate::lexer::Span,
    ) -> Result<(), FerriError> {
        let current_module = self.module_stack.join("::");
        // Check each intermediate prefix as a potential module
        for i in 1..path.len() {
            let prefix: String = path[..i].join("::");
            if self.module_names.contains(&prefix) {
                if let Some(vis) = self.pub_vis.get(&prefix) {
                    match vis {
                        Visibility::Pub | Visibility::PubCrate => {}
                        Visibility::PubSuper => {
                            let parent = prefix.rsplit_once("::").map(|(p, _)| p).unwrap_or("");
                            if !parent.is_empty()
                                && current_module != parent
                                && !current_module.starts_with(&format!("{}::", parent))
                            {
                                return Err(FerriError::Runtime {
                                    message: format!("module `{}` is private", prefix),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                        }
                        Visibility::Private => {
                            return Err(FerriError::Runtime {
                                message: format!("module `{}` is private", prefix),
                                line: span.line,
                                column: span.column,
                            });
                        }
                    }
                } else {
                    // Module is not pub — accessible only from parent or descendants
                    let parent = prefix.rsplit_once("::").map(|(p, _)| p).unwrap_or("");
                    if !parent.is_empty()
                        && current_module != parent
                        && !current_module.starts_with(&format!("{}::", parent))
                    {
                        return Err(FerriError::Runtime {
                            message: format!("module `{}` is private", prefix),
                            line: span.line,
                            column: span.column,
                        });
                    }
                }
            }
        }
        // Check the leaf item
        if !self.is_visible(leaf_qualified) {
            return Err(FerriError::Runtime {
                message: format!("`{}` is private", leaf_qualified),
                line: span.line,
                column: span.column,
            });
        }
        Ok(())
    }

    /// Check whether a field on a struct is visible from the current module context.
    /// Returns Ok(()) if the field can be accessed, or an error if it's private.
    pub(crate) fn check_field_visibility(
        &self,
        struct_name: &str,
        field_name: &str,
        span: crate::lexer::Span,
    ) -> Result<(), FerriError> {
        if let Some(struct_def) = self.struct_defs.get(struct_name) {
            if let StructKind::Named(fields) = &struct_def.kind {
                for field in fields {
                    if field.name == field_name {
                        if matches!(field.visibility, Visibility::Private) {
                            let struct_module =
                                struct_name.rsplit_once("::").map(|(m, _)| m).unwrap_or("");
                            let current_module = self.module_stack.join("::");
                            if struct_module != current_module {
                                return Err(FerriError::Runtime {
                                    message: format!(
                                        "field `{}` of struct `{}` is private",
                                        field_name, struct_name
                                    ),
                                    line: span.line,
                                    column: span.column,
                                });
                            }
                        }
                        return Ok(());
                    }
                }
            }
        }
        // Field not found in definition — allow (validation happens elsewhere)
        Ok(())
    }
}
