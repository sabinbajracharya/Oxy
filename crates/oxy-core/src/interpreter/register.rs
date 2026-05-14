use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::env::Environment;
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::{FunctionData, Value};

use super::{Interpreter, ModuleData};

impl Interpreter {
    /// Register a single item in the current environment (for REPL).
    pub fn register_item(&mut self, item: &Item) -> Result<(), FerriError> {
        match item {
            Item::Function(f) => {
                let value = Value::Function(Box::new(FunctionData {
                    name: f.name.clone(),
                    params: f.params.clone(),
                    return_type: f.return_type.clone(),
                    body: f.body.clone(),
                    closure_env: Rc::clone(&self.env),
                    target_ip: None,
                }));
                self.env.borrow_mut().define(f.name.clone(), value, false);
                if f.is_async {
                    self.async_fns.insert(f.name.clone());
                }
                Ok(())
            }
            Item::Struct(s) => {
                self.register_derive_traits(&s.name, &s.attributes);
                self.struct_defs.insert(s.name.clone(), s.clone());
                Ok(())
            }
            Item::Enum(e) => {
                self.register_derive_traits(&e.name, &e.attributes);
                self.enum_defs.insert(e.name.clone(), e.clone());
                Ok(())
            }
            Item::Impl(i) => {
                let methods = self.impl_methods.entry(i.type_name.clone()).or_default();
                for method in &i.methods {
                    methods.retain(|m| m.name != method.name);
                    methods.push(method.clone());
                }
                Ok(())
            }
            Item::Trait(t) => {
                self.trait_defs.insert(t.name.clone(), t.clone());
                Ok(())
            }
            Item::ImplTrait(i) => {
                let key = (i.type_name.clone(), i.trait_name.clone());
                let methods = self.trait_impls.entry(key).or_default();
                for method in &i.methods {
                    methods.retain(|m| m.name != method.name);
                    methods.push(method.clone());
                }
                Ok(())
            }
            Item::Module(m) => self.register_module(m),
            Item::Use(u) => self.register_use(u),
            Item::TypeAlias { name, target, .. } => {
                self.type_aliases.insert(name.clone(), target.clone());
                Ok(())
            }
            Item::Const {
                name, value, span, ..
            } => {
                let val = self.eval_expr(value, &self.env.clone())?;
                self.env.borrow_mut().define(name.clone(), val, false);
                let _ = span;
                Ok(())
            }
        }
    }

    /// Register an inline or file-based module.
    pub(crate) fn register_module(&mut self, module: &ModuleDef) -> Result<(), FerriError> {
        let items = if let Some(body) = &module.body {
            body.clone()
        } else {
            let source = self.load_module_file(&module.name, module.span)?;
            let program = crate::parser::parse(&source)?;
            program.items
        };

        let mod_env = Environment::new();
        let mut mod_struct_defs = HashMap::new();
        let mut mod_enum_defs = HashMap::new();
        let mut mod_impl_methods: HashMap<String, Vec<FnDef>> = HashMap::new();
        let mut mod_trait_defs = HashMap::new();
        let mut mod_trait_impls: HashMap<(String, String), Vec<FnDef>> = HashMap::new();

        for item in &items {
            match item {
                Item::Function(f) => {
                    let value = Value::Function(Box::new(FunctionData {
                        name: f.name.clone(),
                        params: f.params.clone(),
                        return_type: f.return_type.clone(),
                        body: f.body.clone(),
                        closure_env: Rc::clone(&mod_env),
                        target_ip: None,
                    }));
                    mod_env.borrow_mut().define(f.name.clone(), value, false);
                    if f.is_async {
                        self.async_fns.insert(f.name.clone());
                    }
                }
                Item::Struct(s) => {
                    self.register_derive_traits(&s.name, &s.attributes);
                    mod_struct_defs.insert(s.name.clone(), s.clone());
                }
                Item::Enum(e) => {
                    self.register_derive_traits(&e.name, &e.attributes);
                    mod_enum_defs.insert(e.name.clone(), e.clone());
                }
                Item::Impl(i) => {
                    let methods = mod_impl_methods.entry(i.type_name.clone()).or_default();
                    for method in &i.methods {
                        methods.retain(|m| m.name != method.name);
                        methods.push(method.clone());
                    }
                }
                Item::Trait(t) => {
                    mod_trait_defs.insert(t.name.clone(), t.clone());
                }
                Item::ImplTrait(i) => {
                    let key = (i.type_name.clone(), i.trait_name.clone());
                    let methods = mod_trait_impls.entry(key).or_default();
                    for method in &i.methods {
                        methods.retain(|m| m.name != method.name);
                        methods.push(method.clone());
                    }
                }
                Item::Module(_) | Item::Use(_) | Item::TypeAlias { .. } | Item::Const { .. } => {}
            }
        }

        self.modules.insert(
            module.name.clone(),
            ModuleData {
                env: mod_env,
                struct_defs: mod_struct_defs,
                enum_defs: mod_enum_defs,
                impl_methods: mod_impl_methods,
                trait_defs: mod_trait_defs,
                trait_impls: mod_trait_impls,
            },
        );
        Ok(())
    }

    /// Load a file-based module's source code.
    pub(crate) fn load_module_file(&self, name: &str, span: Span) -> Result<String, FerriError> {
        let base = self.base_dir.as_deref().unwrap_or(".");

        let path1 = format!("{base}/{name}.ox");
        let path2 = format!("{base}/{name}/mod.ox");

        if let Ok(source) = std::fs::read_to_string(&path1) {
            return Ok(source);
        }
        if let Ok(source) = std::fs::read_to_string(&path2) {
            return Ok(source);
        }

        // Search installed packages
        if let Some((source, _pkg_name)) = crate::package::find_module_in_packages(name) {
            return Ok(source);
        }

        Err(FerriError::Runtime {
            message: format!("could not find module `{name}`: tried '{path1}' and '{path2}'"),
            line: span.line,
            column: span.column,
        })
    }

    /// Process a `use` declaration — import items from a module into current scope.
    pub(crate) fn register_use(&mut self, use_def: &UseDef) -> Result<(), FerriError> {
        let (mod_name, item_to_import) = match &use_def.tree {
            UseTree::Simple => {
                if use_def.path.len() < 2 {
                    return Ok(());
                }
                let mod_name = use_def.path[..use_def.path.len() - 1].join("::");
                let item_name = use_def.path.last().unwrap().clone();
                (mod_name, Some(item_name))
            }
            UseTree::Glob => {
                let mod_name = use_def.path.join("::");
                (mod_name, None)
            }
            UseTree::Group(_) => {
                let mod_name = use_def.path.join("::");
                (mod_name, None)
            }
        };

        let resolved_mod = mod_name
            .strip_prefix("crate::")
            .or_else(|| mod_name.strip_prefix("self::"))
            .unwrap_or(&mod_name)
            .to_string();

        let module = self.modules.get(&resolved_mod).cloned();
        let Some(module) = module else {
            if use_def.path.first().map(|s| s.as_str()) == Some("std") {
                match &use_def.tree {
                    UseTree::Simple => {
                        if let Some(last) = use_def.path.last() {
                            self.use_aliases.insert(last.clone(), use_def.path.clone());
                        }
                    }
                    UseTree::Group(names) => {
                        for name in names {
                            let mut full_path = use_def.path.clone();
                            full_path.push(name.clone());
                            self.use_aliases.insert(name.clone(), full_path);
                        }
                    }
                    UseTree::Glob => {}
                }
            }
            return Ok(());
        };

        match &use_def.tree {
            UseTree::Simple => {
                if let Some(name) = item_to_import {
                    self.import_item_from_module(&module, &name);
                }
            }
            UseTree::Glob => {
                self.import_all_from_module(&module);
            }
            UseTree::Group(names) => {
                for name in names {
                    self.import_item_from_module(&module, name);
                }
            }
        }

        Ok(())
    }

    /// Import a single named item from a module into the current scope.
    fn import_item_from_module(&mut self, module: &ModuleData, name: &str) {
        if let Ok(val) = module.env.borrow().get(name) {
            self.env.borrow_mut().define(name.to_string(), val, false);
        }
        if let Some(s) = module.struct_defs.get(name) {
            self.struct_defs.insert(name.to_string(), s.clone());
        }
        if let Some(e) = module.enum_defs.get(name) {
            self.enum_defs.insert(name.to_string(), e.clone());
        }
        if let Some(t) = module.trait_defs.get(name) {
            self.trait_defs.insert(name.to_string(), t.clone());
        }
        if let Some(methods) = module.impl_methods.get(name) {
            let entry = self.impl_methods.entry(name.to_string()).or_default();
            for m in methods {
                entry.retain(|existing| existing.name != m.name);
                entry.push(m.clone());
            }
        }
        for ((type_name, trait_name), methods) in &module.trait_impls {
            if type_name == name {
                let key = (type_name.clone(), trait_name.clone());
                let entry = self.trait_impls.entry(key).or_default();
                for m in methods {
                    entry.retain(|existing| existing.name != m.name);
                    entry.push(m.clone());
                }
            }
        }
    }

    /// Import all items from a module into the current scope.
    fn import_all_from_module(&mut self, module: &ModuleData) {
        let bindings: Vec<(String, Value)> =
            module.env.borrow().all_bindings().into_iter().collect();
        for (name, val) in bindings {
            self.env.borrow_mut().define(name, val, false);
        }
        for (name, s) in &module.struct_defs {
            self.struct_defs.insert(name.clone(), s.clone());
        }
        for (name, e) in &module.enum_defs {
            self.enum_defs.insert(name.clone(), e.clone());
        }
        for (name, t) in &module.trait_defs {
            self.trait_defs.insert(name.clone(), t.clone());
        }
        for (type_name, methods) in &module.impl_methods {
            let entry = self.impl_methods.entry(type_name.clone()).or_default();
            for m in methods {
                entry.retain(|existing| existing.name != m.name);
                entry.push(m.clone());
            }
        }
        for (key, methods) in &module.trait_impls {
            let entry = self.trait_impls.entry(key.clone()).or_default();
            for m in methods {
                entry.retain(|existing| existing.name != m.name);
                entry.push(m.clone());
            }
        }
    }
}
