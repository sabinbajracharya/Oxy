//! Completion, hover, and go-to-definition data for the Oxy LSP.
//!
//! These functions generate completion items, hover text, and resolve
//! definitions by walking the parsed AST and querying the symbol tables
//! defined in `oxy_core::symbols`.

use oxy_core::ast::{Item, Program, UseTree};
use tower_lsp::lsp_types::*;

// ---------------------------------------------------------------------------
// Completion data
// ---------------------------------------------------------------------------

pub(crate) fn keyword_completions() -> Vec<CompletionItem> {
    oxy_core::symbols::KEYWORDS
        .iter()
        .map(|kw| CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        })
        .collect()
}

pub(crate) fn type_completions() -> Vec<CompletionItem> {
    oxy_core::symbols::PRIMITIVE_TYPES
        .iter()
        .map(|(name, detail)| CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::CLASS),
            detail: Some(detail.to_string()),
            ..Default::default()
        })
        .collect()
}

pub(crate) fn builtin_function_completions() -> Vec<CompletionItem> {
    oxy_core::symbols::ALL_MACROS
        .iter()
        .map(|m| CompletionItem {
            label: m.name.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(m.detail.to_string()),
            ..Default::default()
        })
        .collect()
}

pub(crate) fn module_completions() -> Vec<CompletionItem> {
    oxy_core::symbols::ALL_MODULES
        .iter()
        .map(|m| CompletionItem {
            label: m.path.to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some(m.detail.to_string()),
            ..Default::default()
        })
        .collect()
}

pub(crate) fn snippet_completions() -> Vec<CompletionItem> {
    let snippets: &[(&str, &str, &str)] = &[
        ("fn main", "fn main() {\n    $0\n}", "Main function"),
        (
            "fn",
            "fn ${1:name}(${2:params}) {\n    $0\n}",
            "Function definition",
        ),
        (
            "struct",
            "struct ${1:Name} {\n    $0\n}",
            "Struct definition",
        ),
        ("enum", "enum ${1:Name} {\n    $0\n}", "Enum definition"),
        ("impl", "impl ${1:Type} {\n    $0\n}", "Impl block"),
        (
            "match",
            "match ${1:expr} {\n    ${2:pattern} => $0,\n}",
            "Match expression",
        ),
        ("for", "for ${1:item} in ${2:iter} {\n    $0\n}", "For loop"),
        ("while", "while ${1:condition} {\n    $0\n}", "While loop"),
        (
            "if let",
            "if let ${1:pattern} = ${2:expr} {\n    $0\n}",
            "If let binding",
        ),
    ];
    snippets
        .iter()
        .map(|(label, body, detail)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some(detail.to_string()),
            insert_text: Some(body.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Hover data
// ---------------------------------------------------------------------------

pub(crate) fn keyword_hover(word: &str) -> Option<String> {
    oxy_core::symbols::keyword_hover_text(word).map(|s| s.to_string())
}

pub(crate) fn builtin_hover(word: &str) -> Option<String> {
    // Check primitive types
    for &(name, _detail) in oxy_core::symbols::PRIMITIVE_TYPES {
        if word == name {
            for ty in oxy_core::symbols::ALL_TYPES {
                if ty.name == name {
                    return Some(ty.hover_text.to_string());
                }
            }
            // For int/float types not in ALL_TYPES, provide a basic hover
            return Some(format!("**{name}** — numeric type"));
        }
    }
    // Check ALL_TYPES for richer hover
    for ty in oxy_core::symbols::ALL_TYPES {
        if word == ty.name {
            return Some(ty.hover_text.to_string());
        }
    }
    // Check macros
    for m in oxy_core::symbols::ALL_MACROS {
        if word == m.name {
            return Some(m.hover_text.to_string());
        }
    }
    // Built-in functions (not macros)
    match word {
        "spawn" => Some("**spawn(async_fn)** — Spawn an async task".to_string()),
        "sleep" => Some("**sleep(ms)** — Sleep for the given milliseconds".to_string()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Dot / :: detection
// ---------------------------------------------------------------------------

/// Check if the cursor position is immediately after a dot.
pub(crate) fn is_after_dot(source: &str, position: Position) -> bool {
    let line = match source.lines().nth(position.line as usize) {
        Some(l) => l,
        None => return false,
    };
    let col = position.character as usize;
    if col == 0 || col > line.len() {
        return false;
    }
    let bytes = line.as_bytes();
    // Check if the character before cursor is a dot
    bytes.get(col.saturating_sub(1)) == Some(&b'.')
}

/// Completions for method calls after a dot.
pub(crate) fn method_completions() -> Vec<CompletionItem> {
    let mut seen = std::collections::HashSet::new();
    let mut items = Vec::new();
    for ty in oxy_core::symbols::ALL_TYPES {
        for m in ty.methods {
            if seen.insert(m.name) {
                items.push(CompletionItem {
                    label: m.name.to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    detail: Some(m.detail.to_string()),
                    ..Default::default()
                });
            }
        }
    }
    // Generic methods (clone, to_string, to_json, to_json_pretty)
    for m in oxy_core::symbols::GENERIC_TYPE_METHODS {
        if seen.insert(m.name) {
            items.push(CompletionItem {
                label: m.name.to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some(m.detail.to_string()),
                ..Default::default()
            });
        }
    }
    items
}

/// Check if cursor is immediately after `::`.
pub(crate) fn is_after_colon_colon(source: &str, position: Position) -> bool {
    let line = match source.lines().nth(position.line as usize) {
        Some(l) => l,
        None => return false,
    };
    let col = position.character as usize;
    if col < 2 {
        return false;
    }
    let bytes = line.as_bytes();
    bytes.get(col.saturating_sub(1)) == Some(&b':')
        && bytes.get(col.saturating_sub(2)) == Some(&b':')
}

/// Extract the identifier prefix before `::` on the current line.
pub(crate) fn extract_prefix_before_colon_colon(source: &str, position: Position) -> String {
    let line = match source.lines().nth(position.line as usize) {
        Some(l) => l,
        None => return String::new(),
    };
    let col = position.character as usize;
    if col < 2 {
        return String::new();
    }
    // Scan backwards from before the ::
    let scan_start = col.saturating_sub(2);
    let before = &line[..scan_start];
    // Extract the last identifier-like segment before ::
    before
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| before[i + 1..].to_string())
        .unwrap_or_else(|| before.to_string())
}

// ---------------------------------------------------------------------------
// AST-aware completions
// ---------------------------------------------------------------------------

/// Collect user-defined items from the top-level AST for completions.
pub(crate) fn user_defined_completions(program: &Program) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    collect_scope_items(&program.items, &mut items);
    items
}

fn collect_scope_items(ast_items: &[Item], out: &mut Vec<CompletionItem>) {
    for item in ast_items {
        match item {
            Item::Function(f) => {
                out.push(CompletionItem {
                    label: f.name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some(format!(
                        "fn({})",
                        f.params
                            .iter()
                            .map(|p| p.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                    ..Default::default()
                });
            }
            Item::Struct(s) => {
                out.push(CompletionItem {
                    label: s.name.clone(),
                    kind: Some(CompletionItemKind::STRUCT),
                    detail: Some("struct".to_string()),
                    ..Default::default()
                });
            }
            Item::Enum(e) => {
                out.push(CompletionItem {
                    label: e.name.clone(),
                    kind: Some(CompletionItemKind::ENUM),
                    detail: Some("enum".to_string()),
                    ..Default::default()
                });
            }
            Item::Trait(t) => {
                out.push(CompletionItem {
                    label: t.name.clone(),
                    kind: Some(CompletionItemKind::INTERFACE),
                    detail: Some("trait".to_string()),
                    ..Default::default()
                });
            }
            Item::TypeAlias { name, .. } => {
                out.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::CLASS),
                    detail: Some("type alias".to_string()),
                    ..Default::default()
                });
            }
            Item::Const { name, .. } => {
                out.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    detail: Some("const".to_string()),
                    ..Default::default()
                });
            }
            Item::Impl(i) => {
                for method in &i.methods {
                    out.push(CompletionItem {
                        label: format!("{}::{}", i.type_name, method.name),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some("method".to_string()),
                        ..Default::default()
                    });
                }
            }
            Item::ImplTrait(i) => {
                for method in &i.methods {
                    out.push(CompletionItem {
                        label: format!("{}::{}", i.type_name, method.name),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some(format!("{}::{}", i.trait_name, method.name)),
                        ..Default::default()
                    });
                }
            }
            Item::Module(m) => {
                out.push(CompletionItem {
                    label: m.name.clone(),
                    kind: Some(CompletionItemKind::MODULE),
                    detail: Some("mod".to_string()),
                    ..Default::default()
                });
                if let Some(body) = &m.body {
                    collect_scope_items(body, out);
                }
            }
            _ => {}
        }
    }
}

/// Get completions for members of a module or type (after `::`).
pub(crate) fn module_member_completions(program: &Program, prefix: &str) -> Vec<CompletionItem> {
    if prefix.is_empty() {
        return Vec::new();
    }
    let mut items = Vec::new();
    find_module_members(&program.items, prefix, &mut items);
    items
}

fn find_module_members(items: &[Item], prefix: &str, out: &mut Vec<CompletionItem>) {
    for item in items {
        if let Item::Module(m) = item {
            if m.name == prefix {
                if let Some(body) = &m.body {
                    for child in body {
                        match child {
                            Item::Function(f) => {
                                if f.visibility.is_pub() {
                                    out.push(CompletionItem {
                                        label: f.name.clone(),
                                        kind: Some(CompletionItemKind::FUNCTION),
                                        detail: Some("fn".to_string()),
                                        ..Default::default()
                                    });
                                }
                            }
                            Item::Struct(s) if s.visibility.is_pub() => {
                                out.push(CompletionItem {
                                    label: s.name.clone(),
                                    kind: Some(CompletionItemKind::STRUCT),
                                    detail: Some("struct".to_string()),
                                    ..Default::default()
                                });
                            }
                            Item::Enum(e) if e.visibility.is_pub() => {
                                out.push(CompletionItem {
                                    label: e.name.clone(),
                                    kind: Some(CompletionItemKind::ENUM),
                                    detail: Some("enum".to_string()),
                                    ..Default::default()
                                });
                            }
                            Item::Const { name, .. } => {
                                out.push(CompletionItem {
                                    label: name.clone(),
                                    kind: Some(CompletionItemKind::CONSTANT),
                                    detail: Some("const".to_string()),
                                    ..Default::default()
                                });
                            }
                            _ => {}
                        }
                    }
                }
                return;
            }
            if let Some(body) = &m.body {
                find_module_members(body, prefix, out);
            }
        }
    }
    // Also look for impl methods on the type name
    for item in items {
        if let Item::Impl(i) = item {
            if i.type_name == prefix {
                for method in &i.methods {
                    out.push(CompletionItem {
                        label: method.name.clone(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some("fn".to_string()),
                        ..Default::default()
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Type inference for method completions
// ---------------------------------------------------------------------------

/// Try to infer the type of the receiver before a dot.
pub(crate) fn try_infer_receiver_type(source: &str, position: Position) -> Option<String> {
    let line = source.lines().nth(position.line as usize)?;
    let col = position.character as usize;
    if col == 0 {
        return None;
    }
    let before = &line[..col.saturating_sub(1)];
    // Find the last identifier before the dot
    let ident = before
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| before[i + 1..].to_string())
        .unwrap_or_else(|| before.to_string());
    if ident.is_empty() {
        return None;
    }
    // Try to find a let binding with type annotation
    infer_type_from_binding(source, &ident)
}

fn infer_type_from_binding(source: &str, var_name: &str) -> Option<String> {
    // Parse the source and search for `let var_name: Type = ...`
    let program = oxy_core::parser::parse(source).ok()?;
    find_var_type_in_items(&program.items, var_name)
}

fn find_var_type_in_items(items: &[Item], var_name: &str) -> Option<String> {
    for item in items {
        if let Item::Function(f) = item {
            // Check params
            for param in &f.params {
                if param.name == var_name {
                    return Some(param.type_ann.name().to_string());
                }
            }
            // Check body for let bindings
            if let Some(ty) = find_let_type_in_block(&f.body, var_name) {
                return Some(ty);
            }
        }
        if let Item::Module(m) = item {
            if let Some(body) = &m.body {
                if let Some(ty) = find_var_type_in_items(body, var_name) {
                    return Some(ty);
                }
            }
        }
    }
    // Check top-level const/static
    for item in items {
        match item {
            Item::Const { name, type_ann, .. } if name == var_name => {
                return type_ann.as_ref().map(|t| t.name().to_string());
            }
            _ => {}
        }
    }
    None
}

fn find_let_type_in_block(block: &oxy_core::ast::Block, var_name: &str) -> Option<String> {
    for stmt in &block.stmts {
        if let oxy_core::ast::Stmt::Let {
            name,
            type_ann,
            value,
            ..
        } = stmt
        {
            if name == var_name {
                if let Some(ann) = type_ann {
                    return Some(ann.name().to_string());
                }
                // Try to infer from value (simple cases)
                if let Some(expr) = value {
                    if let Some(ty) = infer_simple_expr_type(expr) {
                        return Some(ty);
                    }
                }
            }
        }
    }
    None
}

fn infer_simple_expr_type(expr: &oxy_core::ast::Expr) -> Option<String> {
    match expr {
        oxy_core::ast::Expr::StructInit { name, .. } => Some(name.clone()),
        oxy_core::ast::Expr::IntLiteral(..) => Some(oxy_core::symbols::I64_TYPE.to_string()),
        oxy_core::ast::Expr::FloatLiteral(..) => Some(oxy_core::symbols::F64_TYPE.to_string()),
        oxy_core::ast::Expr::StringLiteral(..) => Some(oxy_core::symbols::STRING_TYPE.to_string()),
        oxy_core::ast::Expr::BoolLiteral(..) => Some(oxy_core::symbols::BOOL_TYPE.to_string()),
        oxy_core::ast::Expr::Ident(name, _) => {
            if name.starts_with(|c: char| c.is_uppercase()) {
                Some(name.clone())
            } else {
                None
            }
        }
        oxy_core::ast::Expr::PathCall { path, .. } => path.first().cloned(),
        oxy_core::ast::Expr::Call { callee, .. } => {
            if let oxy_core::ast::Expr::Ident(name, _) = callee.as_ref() {
                match name.as_str() {
                    "Some" => Some(oxy_core::symbols::OPTION_TYPE.to_string()),
                    "Ok" => Some(oxy_core::symbols::RESULT_TYPE.to_string()),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Find user-defined impl methods for a given type name.
pub(crate) fn find_methods_for_type(program: &Program, type_name: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    search_impl_methods(&program.items, type_name, &mut items);
    items
}

fn search_impl_methods(ast_items: &[Item], type_name: &str, out: &mut Vec<CompletionItem>) {
    for item in ast_items {
        match item {
            Item::Impl(i) if i.type_name == type_name => {
                for method in &i.methods {
                    out.push(CompletionItem {
                        label: method.name.clone(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some(format!(
                            "fn({})",
                            method
                                .params
                                .iter()
                                .map(|p| p.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )),
                        ..Default::default()
                    });
                }
            }
            Item::Module(m) => {
                if let Some(body) = &m.body {
                    search_impl_methods(body, type_name, out);
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Item info for hover / goto-def
// ---------------------------------------------------------------------------

pub(crate) fn item_name(item: &Item) -> Option<&str> {
    match item {
        Item::Function(f) => Some(&f.name),
        Item::Struct(s) => Some(&s.name),
        Item::Enum(e) => Some(&e.name),
        Item::Trait(t) => Some(&t.name),
        Item::Module(m) => Some(&m.name),
        Item::Const { name, .. } => Some(name),
        Item::TypeAlias { name, .. } => Some(name),
        _ => None,
    }
}

pub(crate) fn item_hover_info(item: &Item, name: &str) -> Option<String> {
    if item_name(item) != Some(name) {
        return None;
    }
    match item {
        Item::Function(f) => {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.type_ann.name()))
                .collect();
            let ret = f
                .return_type
                .as_ref()
                .map(|t| format!(" -> {}", t.name()))
                .unwrap_or_default();
            let asyncness = if f.is_async { "async " } else { "" };
            Some(format!(
                "```oxy\n{}fn {}({}){}\n```",
                asyncness,
                f.name,
                params.join(", "),
                ret
            ))
        }
        Item::Struct(s) => Some(format!("```oxy\nstruct {}\n```", s.name)),
        Item::Enum(e) => {
            let variants: Vec<&str> = e.variants.iter().map(|v| v.name.as_str()).collect();
            Some(format!(
                "```oxy\nenum {} {{ {} }}\n```",
                e.name,
                variants.join(", ")
            ))
        }
        Item::Trait(t) => Some(format!("```oxy\ntrait {}\n```", t.name)),
        _ => Some(format!("**{}**", name)),
    }
}

// ---------------------------------------------------------------------------
// Go-to-definition helpers
// ---------------------------------------------------------------------------

/// Resolve a name through `use` imports. Returns the resolved name if found,
/// otherwise the original name unchanged.
pub(crate) fn resolve_use_import(items: &[Item], name: &str) -> String {
    for item in items {
        if let Item::Use(use_def) = item {
            let resolved = resolve_use_tree(&use_def.tree, &use_def.path, name);
            if resolved != name {
                return resolved;
            }
        }
    }
    name.to_string()
}

fn resolve_use_tree(tree: &UseTree, path: &[String], name: &str) -> String {
    match tree {
        UseTree::Simple(alias) => {
            let last_seg = path.last().cloned().unwrap_or_default();
            let local = alias.as_ref().unwrap_or(&last_seg);
            if local == name {
                return path.join("::");
            }
        }
        UseTree::Group(items) => {
            for (item_name, alias) in items {
                let local = alias.as_ref().unwrap_or(item_name);
                if local == name {
                    return format!("{}::{}", path.join("::"), item_name);
                }
            }
        }
        UseTree::Glob => {}
    }
    name.to_string()
}

/// Search all items recursively (including inside modules) for a definition span.
pub(crate) fn find_definition_span(items: &[Item], name: &str) -> Option<oxy_core::lexer::Span> {
    for item in items {
        if item_name(item) == Some(name) {
            return Some(item.span());
        }
        if let Item::Module(m) = item {
            if let Some(body) = &m.body {
                if let Some(span) = find_definition_span(body, name) {
                    return Some(span);
                }
            }
        }
    }
    None
}

/// Search all items recursively for hover info.
pub(crate) fn find_item_hover(items: &[Item], name: &str) -> Option<String> {
    for item in items {
        if let Some(desc) = item_hover_info(item, name) {
            return Some(desc);
        }
        if let Item::Module(m) = item {
            if let Some(body) = &m.body {
                if let Some(desc) = find_item_hover(body, name) {
                    return Some(desc);
                }
            }
        }
    }
    None
}
