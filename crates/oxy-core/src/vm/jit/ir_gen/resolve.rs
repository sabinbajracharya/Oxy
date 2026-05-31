//! Name, path, and type-alias resolution for `IrGen` — part of the AST → register IR
//! lowering pass. See `mod.rs` for the `IrGen` struct and state.

use super::*;

impl IrGen {
    /// Resolve `self`, `super`, `crate` path prefixes and `use_aliases` against
    /// the current module context, producing an absolute path segment list.
    pub(super) fn resolve_module_path(&self, path: &[String]) -> Vec<String> {
        let mut module_parts: Vec<&str> = self
            .current_module_prefix
            .split("::")
            .filter(|s| !s.is_empty())
            .collect();
        let mut iter = path.iter().peekable();
        let mut resolved: Vec<String> = Vec::new();

        // Consume leading self / super / crate prefixes.
        while let Some(seg) = iter.peek() {
            match seg.as_str() {
                "self" => {
                    iter.next();
                }
                "super" => {
                    module_parts.pop();
                    iter.next();
                }
                "crate" => {
                    module_parts.clear();
                    iter.next();
                }
                _ => break,
            }
        }

        // Resolve the first remaining segment through use_aliases.
        // If found, the alias provides a fully-qualified path so we skip
        // prepending current_module_prefix. Otherwise fall back to the
        // module prefix for relative resolution. The alias target is split
        // back into individual segments (a `use std::env` alias maps to the
        // string "std::env") so the path stays segment-aligned — downstream
        // FFI dispatch matches paths segment-by-segment (`["std", mod, fn]`),
        // which a single embedded-`::` segment would silently defeat.
        if let Some(first) = iter.next() {
            if let Some(resolved_first) = self.use_aliases.get(first) {
                resolved.extend(resolved_first.split("::").map(str::to_string));
            } else {
                resolved.extend(module_parts.iter().map(|s| s.to_string()));
                resolved.push(first.clone());
            }
        } else {
            // Entire path was self/super/crate — emit the module context.
            resolved.extend(module_parts.iter().map(|s| s.to_string()));
        }

        // Output remaining path segments.
        resolved.extend(iter.cloned());
        resolved
    }

    /// Replace a leading type-alias segment with its underlying type. Given an
    /// already module-resolved path whose first N-1 segments name a type (e.g.
    /// `["Direction", "Up"]` or `["P", "origin"]`), rewrite that type portion
    /// through the alias map so the call resolves to the real enum/struct
    /// (`["Dir", "Up"]`, `["PoInt", "origin"]`). Non-alias paths pass through.
    pub(super) fn resolve_type_alias_in_path(&self, segments: &[String]) -> Vec<String> {
        if segments.len() < 2 {
            return segments.to_vec();
        }
        let type_part = segments[..segments.len() - 1].join("::");
        if let Some(target) = self.type_aliases.get(&type_part) {
            let mut out: Vec<String> = target.split("::").map(str::to_string).collect();
            out.push(segments[segments.len() - 1].clone());
            out
        } else {
            segments.to_vec()
        }
    }

    /// Resolve a `use` declaration's path to an absolute segment list. Unlike
    /// reference sites (handled by [`resolve_module_path`], which are relative
    /// to the enclosing module), `use` paths are crate-absolute — `use a::b`
    /// inside `mod m` means `a::b`, not `m::a::b`. Only a leading `self` /
    /// `super` / `crate` prefix makes the path relative, and that prefix is
    /// resolved against the current module context.
    pub(super) fn resolve_use_path(&self, path: &[String]) -> Vec<String> {
        let mut module_parts: Vec<&str> = self
            .current_module_prefix
            .split("::")
            .filter(|s| !s.is_empty())
            .collect();
        let mut iter = path.iter().peekable();
        let mut had_prefix = false;
        while let Some(seg) = iter.peek() {
            match seg.as_str() {
                "self" => {
                    iter.next();
                    had_prefix = true;
                }
                "super" => {
                    module_parts.pop();
                    iter.next();
                    had_prefix = true;
                }
                "crate" => {
                    module_parts.clear();
                    iter.next();
                    had_prefix = true;
                }
                _ => break,
            }
        }
        let mut resolved: Vec<String> = Vec::new();
        // A self/super/crate prefix anchors the remaining path to the resolved
        // module context; a bare path is already crate-absolute.
        if had_prefix {
            resolved.extend(module_parts.iter().map(|s| s.to_string()));
        }
        resolved.extend(iter.cloned());
        resolved
    }

    /// Resolve a function name through the re-export alias chain. Follows
    /// chains like layer3::value → layer2::value → layer1::value, stopping on
    /// any revisited name so a self-referential alias (e.g. `pub use self::val`
    /// registering `m::val → m::val`) or a cycle terminates instead of looping.
    pub(super) fn resolve_fn_alias(&self, name: &str) -> String {
        let mut current = name.to_string();
        let mut seen = std::collections::HashSet::new();
        seen.insert(current.clone());
        while let Some(target) = self.fn_aliases.get(&current) {
            if !seen.insert(target.clone()) {
                break;
            }
            current = target.clone();
        }
        current
    }

    /// Resolve a bare identifier naming a function (callee or value reference) to
    /// the function's compiled name. Precedence: explicit `use` alias, then a
    /// sibling function in the current module (`prefix::name`) when one exists,
    /// then the bare name — each finally run through the re-export alias chain.
    /// The module-qualification step is what makes an unqualified sibling call
    /// inside a module (`private_fn()` within `mod api`) reach `api::private_fn`
    /// without breaking calls to top-level functions from inside a module.
    pub(super) fn resolve_callable_name(&self, name: &str) -> String {
        if let Some(alias) = self.use_aliases.get(name) {
            return self.resolve_fn_alias(alias);
        }
        let qualified = self.resolve_module_path(&[name.to_string()]).join("::");
        if qualified != name && self.fn_names.contains(&qualified) {
            return self.resolve_fn_alias(&qualified);
        }
        // Glob imports (`use mod::*`): try `glob_mod::name`, following any
        // re-export chain, and accept the first candidate that names a real
        // function. fn_names is fully populated by the pre-pass, so this works
        // whether the source module is defined before or after the glob.
        for glob_mod in &self.glob_mods {
            let candidate = format!("{glob_mod}::{name}");
            let resolved = self.resolve_fn_alias(&candidate);
            if self.fn_names.contains(&resolved) {
                return resolved;
            }
        }
        self.resolve_fn_alias(name)
    }

    /// Resolve an enum-variant reference written in a pattern (e.g. `Color::Red`,
    /// parsed with `enum_name = "Color"`) to the fully-qualified enum identity
    /// the value was constructed with (e.g. `"colors::Color"`). Equality in
    /// `oxy_enum_variant_equal` compares the full enum name, so a pattern arm
    /// inside module `colors` — or one referring to the enum through a `use`
    /// alias — must canonicalize the same way construction sites do.
    pub(super) fn resolve_pattern_enum_name(&self, enum_name: &str, variant: &str) -> String {
        // The variant→enum map holds the canonical (module-qualified) name that
        // construction also produces. Adopt it only when it agrees with what was
        // written (exact, or as a `::`-qualified suffix) so two distinct enums
        // sharing a variant name aren't conflated.
        if let Some(canonical) = self.variant_to_enum.get(variant) {
            if canonical == enum_name
                || canonical.rsplit("::").next() == Some(enum_name)
                || canonical.ends_with(&format!("::{enum_name}"))
            {
                return canonical.clone();
            }
        }
        // Fall back to module-path resolution, mirroring Expr::Path construction.
        let mut segments: Vec<String> = enum_name.split("::").map(|s| s.to_string()).collect();
        segments.push(variant.to_string());
        let resolved = self.resolve_module_path(&segments);
        if resolved.len() > 1 {
            resolved[..resolved.len() - 1].join("::")
        } else {
            enum_name.to_string()
        }
    }

    /// Convert a type annotation to TypeInfo (simple types only; complex types map to Unknown).
    pub(super) fn type_ann_to_type_info(ann: &TypeAnnotation) -> TypeInfo {
        match ann {
            TypeAnnotation::Named {
                name, generic_args, ..
            } => {
                if generic_args.is_empty() {
                    TypeInfo::from_name(name)
                } else {
                    // Parameterized types — map generics and construct
                    let args: Vec<TypeInfo> = generic_args
                        .iter()
                        .map(Self::type_ann_to_type_info)
                        .collect();
                    match name.as_str() {
                        "List" => TypeInfo::Vec(Box::new(
                            args.first().cloned().unwrap_or(TypeInfo::Unknown),
                        )),
                        "Map" => TypeInfo::HashMap(
                            Box::new(args.first().cloned().unwrap_or(TypeInfo::Unknown)),
                            Box::new(args.get(1).cloned().unwrap_or(TypeInfo::Unknown)),
                        ),
                        "Option" => TypeInfo::Option(Box::new(
                            args.first().cloned().unwrap_or(TypeInfo::Unknown),
                        )),
                        "Result" => TypeInfo::Result(
                            Box::new(args.first().cloned().unwrap_or(TypeInfo::Unknown)),
                            Box::new(args.get(1).cloned().unwrap_or(TypeInfo::Unknown)),
                        ),
                        _ => TypeInfo::UserStruct {
                            name: name.clone(),
                            generic_args: args,
                        },
                    }
                }
            }
            TypeAnnotation::Array { inner, size, .. } => {
                TypeInfo::Array(Box::new(Self::type_ann_to_type_info(inner)), *size)
            }
        }
    }
}
