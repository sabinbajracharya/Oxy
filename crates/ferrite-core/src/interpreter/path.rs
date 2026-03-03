//! Path resolution and associated function dispatch.
//!
//! Handles `Type::method()` calls (associated functions), enum variant
//! constructors, module function calls, stdlib pseudo-module dispatch
//! (math::, rand::, time::, json::, http::), user-defined method dispatch
//! via impl blocks and trait impls, and `std::` path functions.

use std::collections::HashMap;

use crate::ast::*;
use crate::env::{Env, Environment};
use crate::errors::FerriError;
use crate::lexer::Span;
use crate::types::Value;

use super::Interpreter;

impl Interpreter {
    /// Execute an associated function (non-`self` method from an impl or trait impl block).
    pub(crate) fn call_associated_fn(
        &mut self,
        method_def: &FnDef,
        type_name: &str,
        args: &[Value],
        env: &Env,
    ) -> Result<Value, FerriError> {
        let func_env = Environment::child(env);
        for (param, arg) in method_def.params.iter().zip(args.iter()) {
            func_env
                .borrow_mut()
                .define(param.name.clone(), arg.clone(), true);
        }
        let prev_self_type = self.current_self_type.take();
        self.current_self_type = Some(type_name.to_string());
        let result = self.eval_block(&method_def.body, &func_env);
        self.current_self_type = prev_self_type;
        match result {
            Err(FerriError::Return(val)) => Ok(*val),
            other => other,
        }
    }

    /// Evaluate a path call like `Type::method(args)` or `module::func(args)`.
    ///
    /// Handles (in order): enum variant constructors, impl associated functions,
    /// trait impl associated functions, built-in types (String::from, HashMap::new),
    /// #[derive(Default)], json/http pseudo-modules, user modules, stdlib
    /// pseudo-modules (math, rand, time), 3-segment module paths, std:: paths.
    pub(crate) fn eval_path_call(
        &mut self,
        path: &[String],
        args: &[Value],
        span: &Span,
        env: &Env,
    ) -> Result<Value, FerriError> {
        // Expand use aliases: `env::args()` → `std::env::args()` if `use std::env;`
        let expanded;
        let path = if let Some(alias) = self.use_aliases.get(&path[0]) {
            expanded = alias
                .iter()
                .chain(path[1..].iter())
                .cloned()
                .collect::<Vec<_>>();
            &expanded
        } else {
            path
        };

        if path.len() == 2 {
            let type_name = &self.resolve_type_alias(&path[0]);
            let method_name = &path[1];

            // Check for enum variant constructor: `Shape::Circle(5.0)`
            if let Some(edef) = self.enum_defs.get(type_name).cloned() {
                for variant in &edef.variants {
                    if variant.name == *method_name {
                        return Ok(Value::EnumVariant {
                            enum_name: type_name.clone(),
                            variant: method_name.clone(),
                            data: args.to_vec(),
                        });
                    }
                }
            }

            // Check for associated function in impl: `Point::new(1.0, 2.0)`
            if let Some(methods) = self.impl_methods.get(type_name).cloned() {
                for method_def in &methods {
                    if method_def.name == *method_name {
                        // Check it's an associated function (first param is not `self`)
                        let is_method = method_def.params.first().is_some_and(|p| p.name == "self");
                        if is_method {
                            return Err(FerriError::Runtime {
                                message: format!(
                                    "`{type_name}::{method_name}` is a method, not an associated function — call with `.{method_name}()` on an instance"
                                ),
                                line: span.line,
                                column: span.column,
                            });
                        }

                        return self.call_associated_fn(method_def, type_name, args, env);
                    }
                }
            }

            // Check for associated functions in trait impls
            let trait_impl_keys: Vec<_> = self.trait_impls.keys().cloned().collect();
            for key in &trait_impl_keys {
                let (tn, _) = key;
                let methods = self.trait_impls[key].clone();
                if tn == type_name {
                    for method_def in methods {
                        if method_def.name == *method_name {
                            let is_method =
                                method_def.params.first().is_some_and(|p| p.name == "self");
                            if !is_method {
                                return self.call_associated_fn(&method_def, type_name, args, env);
                            }
                        }
                    }
                }
            }

            // Built-in String::from
            if type_name == "String" && method_name == "from" && args.len() == 1 {
                return Ok(Value::String(format!("{}", args[0])));
            }

            // Built-in HashMap::new
            if type_name == "HashMap" && method_name == "new" && args.is_empty() {
                return Ok(Value::HashMap(HashMap::new()));
            }

            // Built-in Type::default() for #[derive(Default)]
            if method_name == "default" && args.is_empty() && self.has_derive(type_name, "Default")
            {
                return self.create_default_value(type_name, span);
            }

            // Built-in json:: pseudo-module
            if type_name == "json" {
                return self.call_json_function(method_name, args, span);
            }

            // Built-in http:: pseudo-module
            if type_name == "http" {
                return self.call_http_function(method_name, args, span);
            }

            // Built-in Server::new() and Response::text/json/html/status
            if type_name == "Server" || type_name == "Response" {
                return self.call_server_path(type_name, method_name, args, span);
            }

            // Built-in Db::open() and Db::memory()
            if type_name == "Db" {
                return self.call_db_path(method_name, args, span);
            }

            // Check for module function call: `module::function(args)`
            if let Some(module) = self.modules.get(type_name).cloned() {
                if let Ok(val) = module.env.borrow().get(method_name) {
                    if let Value::Function(_) = &val {
                        return self.call_function(&val, args, span.line, span.column);
                    }
                }
                // Check for enum variant in module
                if let Some(edef) = module.enum_defs.get(method_name) {
                    // This handles module::EnumName — but the variant is the next segment
                    // For 2-segment, this is module::function, already handled above
                    let _ = edef; // suppress unused
                }
            }

            // Built-in math:: pseudo-module (after user module check)
            if type_name == "math" {
                return crate::stdlib::math::call(method_name, args, span);
            }

            // Built-in rand:: pseudo-module
            if type_name == "rand" {
                return crate::stdlib::rand::call(method_name, args, span);
            }

            // Built-in time:: pseudo-module
            if type_name == "time" {
                return crate::stdlib::time::call(method_name, args, span);
            }
        }

        // Handle 3-segment paths: `module::Type::method(args)`
        if path.len() == 3 {
            let mod_name = &path[0];
            let type_name = &path[1];
            let method_name = &path[2];

            if let Some(module) = self.modules.get(mod_name).cloned() {
                // Check for enum variant constructor in module
                if let Some(edef) = module.enum_defs.get(type_name) {
                    for variant in &edef.variants {
                        if variant.name == *method_name {
                            return Ok(Value::EnumVariant {
                                enum_name: type_name.clone(),
                                variant: method_name.clone(),
                                data: args.to_vec(),
                            });
                        }
                    }
                }
                // Check for associated function in module
                if let Some(methods) = module.impl_methods.get(type_name) {
                    for method_def in methods {
                        if method_def.name == *method_name {
                            return self.call_associated_fn(method_def, type_name, args, env);
                        }
                    }
                }
            }
        }

        // Handle std:: paths — delegate to stdlib modules (SRP)
        if path.len() == 3 && path[0] == "std" {
            let module = &path[1];
            let func = &path[2];
            return match module.as_str() {
                "fs" => crate::stdlib::fs::call(func, args, span),
                "env" if func == "args" => {
                    // args() needs interpreter state, handled here
                    let args_vec: Vec<Value> = self
                        .cli_args
                        .iter()
                        .map(|a| Value::String(a.clone()))
                        .collect();
                    Ok(Value::Vec(args_vec))
                }
                "env" => crate::stdlib::env::call(func, args, span),
                "process" => crate::stdlib::process::call(func, args, span),
                "regex" => crate::stdlib::regex::call(func, args, span),
                "net" => crate::stdlib::net::call(func, args, span),
                _ => Err(FerriError::Runtime {
                    message: format!("unknown std module `std::{module}`"),
                    line: span.line,
                    column: span.column,
                }),
            };
        }

        Err(FerriError::Runtime {
            message: format!("undefined path `{}`", path.join("::")),
            line: span.line,
            column: span.column,
        })
    }

    /// Resolve a path expression (without calling it) — for enum variants and constants.
    pub(crate) fn eval_path(&self, segments: &[String], span: &Span) -> Result<Value, FerriError> {
        if segments.len() == 2 {
            let type_name = &self.resolve_type_alias(&segments[0]);
            let variant_name = &segments[1];

            // Unit enum variant: `Color::Red`
            if let Some(edef) = self.enum_defs.get(type_name) {
                for variant in &edef.variants {
                    if variant.name == *variant_name {
                        if let EnumVariantKind::Unit = variant.kind {
                            return Ok(Value::EnumVariant {
                                enum_name: type_name.clone(),
                                variant: variant_name.clone(),
                                data: vec![],
                            });
                        }
                    }
                }
            }

            // Built-in math constants (PI, E, TAU, etc.)
            if type_name == "math" {
                if let Some(val) = crate::stdlib::math::constant(variant_name) {
                    return Ok(val);
                }
            }

            // Module value access: `module::value`
            if let Some(module) = self.modules.get(type_name) {
                if let Ok(val) = module.env.borrow().get(variant_name) {
                    return Ok(val);
                }
            }
        }

        // 3-segment: `module::Type::Variant`
        if segments.len() == 3 {
            let mod_name = &segments[0];
            let type_name = &segments[1];
            let variant_name = &segments[2];

            if let Some(module) = self.modules.get(mod_name) {
                if let Some(edef) = module.enum_defs.get(type_name) {
                    for variant in &edef.variants {
                        if variant.name == *variant_name {
                            if let EnumVariantKind::Unit = variant.kind {
                                return Ok(Value::EnumVariant {
                                    enum_name: type_name.clone(),
                                    variant: variant_name.clone(),
                                    data: vec![],
                                });
                            }
                        }
                    }
                }
            }
        }

        Err(FerriError::Runtime {
            message: format!("undefined path `{}`", segments.join("::")),
            line: span.line,
            column: span.column,
        })
    }

    /// Dispatch a method call on a user-defined type (struct/enum with impl blocks).
    ///
    /// Searches direct impl methods first, then trait impl methods,
    /// then falls back to built-in methods (to_json, clone).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn call_user_method(
        &mut self,
        receiver: Value,
        type_name: &str,
        method: &str,
        args: Vec<Value>,
        receiver_expr: &Expr,
        env: &Env,
        span: &Span,
    ) -> Result<Value, FerriError> {
        // First, search direct impl methods
        if let Some(methods) = self.impl_methods.get(type_name).cloned() {
            for method_def in &methods {
                if method_def.name == method {
                    return self.dispatch_method(
                        &method_def.clone(),
                        receiver,
                        type_name,
                        args,
                        receiver_expr,
                        env,
                        span,
                    );
                }
            }
        }

        // Then search trait impl methods
        if let Some(method_def) = self.find_trait_method(type_name, method) {
            return self.dispatch_method(
                &method_def,
                receiver,
                type_name,
                args,
                receiver_expr,
                env,
                span,
            );
        }

        // Built-in to_json / to_json_pretty
        if method == "to_json" || method == "to_json_pretty" {
            return self.try_to_json_method(receiver, method, span, type_name);
        }

        // Built-in .clone() for types with #[derive(Clone)]
        if method == "clone" && self.has_derive(type_name, "Clone") {
            return Ok(receiver.clone());
        }

        Err(FerriError::Runtime {
            message: format!("no method `{method}` found for type `{type_name}`"),
            line: span.line,
            column: span.column,
        })
    }

    /// Try to call `.to_json()` or `.to_json_pretty()` on any value.
    pub(crate) fn try_to_json_method(
        &self,
        receiver: Value,
        method: &str,
        span: &Span,
        type_name: &str,
    ) -> Result<Value, FerriError> {
        if method == "to_json" || method == "to_json_pretty" {
            let result = if method == "to_json" {
                crate::json::serialize(&receiver)
            } else {
                crate::json::serialize_pretty(&receiver)
            };
            return match result {
                Ok(json) => Ok(Value::ok(Value::String(json))),
                Err(e) => Ok(Value::err(Value::String(e))),
            };
        }
        Err(FerriError::Runtime {
            message: format!("no method `{method}` on {type_name}"),
            line: span.line,
            column: span.column,
        })
    }

    /// Create a default value for a struct with `#[derive(Default)]`.
    pub(crate) fn create_default_value(
        &self,
        type_name: &str,
        span: &Span,
    ) -> Result<Value, FerriError> {
        if let Some(sdef) = self.struct_defs.get(type_name).cloned() {
            if let StructKind::Named(ref fields) = sdef.kind {
                let mut field_map = HashMap::new();
                for field in fields {
                    let default = Self::default_for_type(&field.type_ann.name);
                    field_map.insert(field.name.clone(), default);
                }
                return Ok(Value::Struct {
                    name: type_name.to_string(),
                    fields: field_map,
                });
            }
        }
        Err(FerriError::Runtime {
            message: format!("cannot create default for `{type_name}`"),
            line: span.line,
            column: span.column,
        })
    }

    /// Return the default value for a type annotation name.
    fn default_for_type(type_name: &str) -> Value {
        match type_name {
            "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "usize" | "isize" => {
                Value::Integer(0)
            }
            "f64" | "f32" => Value::Float(0.0),
            "bool" => Value::Bool(false),
            "String" | "&str" => Value::String(String::new()),
            "char" => Value::Char('\0'),
            _ => Value::Unit,
        }
    }

    /// Search trait impls for a method, including default implementations.
    pub(crate) fn find_trait_method(&self, type_name: &str, method: &str) -> Option<FnDef> {
        // Search all trait impls for this type
        for ((tn, trait_name), methods) in &self.trait_impls {
            if tn == type_name {
                // Check explicit impl methods first
                for m in methods {
                    if m.name == method {
                        return Some(m.clone());
                    }
                }
                // Check default methods from the trait definition
                if let Some(trait_def) = self.trait_defs.get(trait_name) {
                    for m in &trait_def.default_methods {
                        if m.name == method {
                            return Some(m.clone());
                        }
                    }
                }
            }
        }
        None
    }

    /// Dispatch a method call: bind self, params, execute body, propagate mutations.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_method(
        &mut self,
        method_def: &FnDef,
        receiver: Value,
        type_name: &str,
        args: Vec<Value>,
        receiver_expr: &Expr,
        env: &Env,
        span: &Span,
    ) -> Result<Value, FerriError> {
        let func_env = Environment::child(env);

        // Bind `self`
        func_env
            .borrow_mut()
            .define("self".to_string(), receiver.clone(), true);

        // Bind remaining params (skip `self` in params)
        let non_self_params: Vec<_> = method_def
            .params
            .iter()
            .filter(|p| p.name != "self")
            .collect();

        for (param, arg) in non_self_params.iter().zip(args.iter()) {
            func_env
                .borrow_mut()
                .define(param.name.clone(), arg.clone(), true);
        }

        let prev_self_type = self.current_self_type.take();
        self.current_self_type = Some(type_name.to_string());
        let result = self.eval_block(&method_def.body, &func_env);

        // If method mutated `self`, propagate changes back
        if let Ok(updated_self) = func_env.borrow().get("self") {
            if updated_self != receiver {
                let _ = self.mutate_variable(receiver_expr, updated_self, env, span);
            }
        }

        self.current_self_type = prev_self_type;

        match result {
            Err(FerriError::Return(val)) => Ok(*val),
            other => other,
        }
    }
}
