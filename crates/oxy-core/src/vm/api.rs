// vm/api.rs — Public crate entry points for compiling and running Oxy programs.
//
// Extracted from vm/mod.rs to keep that file focused on the Vm struct and its
// execution loop.

use std::collections::HashMap;
use std::path::PathBuf;

use super::VmResult;
use crate::types::Value;

/// Wrap a backend message string as a runtime `PipelineError` (line/column unknown).
fn runtime_error(message: String) -> crate::errors::PipelineError {
    crate::errors::PipelineError::Runtime {
        message,
        line: 0,
        column: 0,
    }
}

// ── JIT entry points (native only) ────────────────────────────────────
//
// Cranelift emits host machine code and is unavailable on `wasm32`, so the
// whole JIT surface is gated to non-wasm. On wasm, execution runs through the
// portable IR interpreter (see the `*_interp_*` entry points below); the public
// dispatchers at the bottom of this file pick the backend per target.

/// Compile and run using the Cranelift JIT backend.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_compiled_jit(source: &str) -> Result<Value, crate::errors::PipelineError> {
    run_compiled_jit_with_options(source, None, HashMap::new())
}

/// Compile and run with JIT, with optional source path and externs.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_compiled_jit_with_options(
    source: &str,
    source_path: Option<&str>,
    externs: HashMap<String, PathBuf>,
) -> Result<Value, crate::errors::PipelineError> {
    let mut jit_vm = super::jit::JitVm::compile_with_options(source, source_path, externs)
        .map_err(|e| crate::errors::PipelineError::Runtime {
            message: e,
            line: 0,
            column: 0,
        })?;
    match jit_vm.run() {
        VmResult::Value(v) => Ok(v),
        VmResult::Error(e) => Err(crate::errors::PipelineError::Runtime {
            message: e,
            line: 0,
            column: 0,
        }),
    }
}

/// Compile and run with JIT, capturing printed output.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_compiled_capturing_jit(
    source: &str,
) -> Result<(Value, Vec<String>), crate::errors::PipelineError> {
    let mut jit_vm =
        super::jit::JitVm::compile(source).map_err(|e| crate::errors::PipelineError::Runtime {
            message: e,
            line: 0,
            column: 0,
        })?;
    jit_vm.with_captured_output();
    match jit_vm.run() {
        VmResult::Value(v) => Ok((v, jit_vm.captured_output())),
        VmResult::Error(e) => Err(crate::errors::PipelineError::Runtime {
            message: e,
            line: 0,
            column: 0,
        }),
    }
}

/// JIT-based conformance alias for run.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_jit(source: &str) -> Result<Value, crate::errors::PipelineError> {
    run_compiled_jit(source)
}

/// Run all #[test] and #[compile_error] functions using the JIT backend.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_tests_jit(
    path: &str,
    source: &str,
) -> Result<Vec<TestResult>, crate::errors::PipelineError> {
    run_tests_jit_with_options(path, source, HashMap::new())
}

/// Same as run_tests_jit with externs.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_tests_jit_with_options(
    path: &str,
    source: &str,
    externs: HashMap<String, PathBuf>,
) -> Result<Vec<TestResult>, crate::errors::PipelineError> {
    let mut program = crate::parser::parse(source)?;
    let source_dir = std::path::Path::new(path).parent().and_then(|p| p.to_str());
    super::jit::resolve_modules(&mut program.items, source_dir, &externs).map_err(|e| {
        crate::errors::PipelineError::Runtime {
            message: e,
            line: 0,
            column: 0,
        }
    })?;

    let mut normal_items: Vec<crate::ast::Item> = Vec::new();
    let mut compile_error_fns: Vec<crate::ast::FnDef> = Vec::new();

    for item in program.items {
        if let crate::ast::Item::Function(ref f) = item {
            if f.attributes.iter().any(|a| a.name == "compile_error") {
                compile_error_fns.push(f.clone());
                continue;
            }
        }
        normal_items.push(item);
    }

    let mut normal_program = crate::ast::Program {
        items: normal_items,
        span: program.span,
    };
    super::jit::expand_derives(&mut normal_program);

    crate::type_checker::TypeChecker::new().check_program(&normal_program)?;

    // Build JIT engine from the typed program
    let engine = super::jit::JitEngine::compile(&normal_program).map_err(|e| {
        crate::errors::PipelineError::Runtime {
            message: e,
            line: 0,
            column: 0,
        }
    })?;
    let mut jit_vm = super::jit::JitVm {
        engine,
        output: None,
    };

    // Collect test functions
    let test_fns: Vec<&crate::ast::FnDef> = normal_program
        .items
        .iter()
        .filter_map(|item| {
            if let crate::ast::Item::Function(f) = item {
                if f.attributes.iter().any(|a| a.name == "test") {
                    Some(f)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let mut results = Vec::new();
    for test_fn in &test_fns {
        if jit_vm.engine.functions.contains_key(&test_fn.name) {
            let result = jit_vm.run_function(&test_fn.name);
            match result {
                VmResult::Value(_) => results.push(TestResult {
                    name: test_fn.name.clone(),
                    passed: true,
                    error: None,
                }),
                VmResult::Error(e) => results.push(TestResult {
                    name: test_fn.name.clone(),
                    passed: false,
                    error: Some(e),
                }),
            }
        } else {
            results.push(TestResult {
                name: test_fn.name.clone(),
                passed: false,
                error: Some("JIT: function not found".into()),
            });
        }
    }

    // Test compile_error functions (unchanged from VM path)
    for ce_fn in &compile_error_fns {
        let ce_item = crate::ast::Item::Function(ce_fn.clone());
        let mut ce_items = normal_program.items.clone();
        ce_items.push(ce_item);
        let ce_program = crate::ast::Program {
            items: ce_items,
            span: program.span,
        };

        let tc_result = crate::type_checker::TypeChecker::new().check_program(&ce_program);
        if tc_result.is_err() {
            results.push(TestResult {
                name: ce_fn.name.clone(),
                passed: true,
                error: None,
            });
            continue;
        }

        let compile_result = super::jit::JitEngine::compile(&ce_program);
        match compile_result {
            Err(_) => results.push(TestResult {
                name: ce_fn.name.clone(),
                passed: true,
                error: None,
            }),
            Ok(_) => results.push(TestResult {
                name: ce_fn.name.clone(),
                passed: false,
                error: Some(
                    "expected compilation error, but code compiled successfully".to_string(),
                ),
            }),
        }
    }

    Ok(results)
}

// ── IR interpreter entry points (all targets) ─────────────────────────
//
// The portable register-IR interpreter (`vm::interp`) executes the same IR the
// Cranelift backend compiles, delegating runtime semantics to the shared `oxy_*`
// FFI. It is the execution backend on `wasm32`, where Cranelift is unavailable.
// These mirror the `*_jit_*` functions above one-to-one so the public
// dispatchers can route by target with identical observable behavior.
//
// They are available on *all* targets (not just wasm): on native they are the
// reference engine the `jit_interp_parity` test diffs the JIT against. Only the
// Cranelift JIT is genuinely native-only.

/// Parse → resolve modules → expand derives → type-check, yielding a program
/// ready for IR lowering. Shared front-end for the interpreter entry points.
fn prepare_program(
    source: &str,
    source_path: Option<&str>,
    externs: &HashMap<String, PathBuf>,
) -> Result<crate::ast::Program, crate::errors::PipelineError> {
    let mut program = crate::parser::parse(source)?;
    let source_dir = source_path.and_then(|p| {
        std::path::Path::new(p)
            .parent()
            .and_then(|parent| parent.to_str())
    });
    super::jit::resolve_modules(&mut program.items, source_dir, externs).map_err(runtime_error)?;
    super::jit::expand_derives(&mut program);
    crate::type_checker::TypeChecker::new().check_program(&program)?;
    Ok(program)
}

/// Compile and run using the portable IR interpreter, with optional source path
/// and externs.
pub fn run_compiled_interp_with_options(
    source: &str,
    source_path: Option<&str>,
    externs: HashMap<String, PathBuf>,
) -> Result<Value, crate::errors::PipelineError> {
    let program = prepare_program(source, source_path, &externs)?;
    let engine = super::interp::InterpEngine::compile(&program).map_err(runtime_error)?;
    let interp = super::interp::Interpreter::new(&engine);
    match interp.run() {
        VmResult::Value(v) => Ok(v),
        VmResult::Error(e) => Err(runtime_error(e)),
    }
}

/// Compile and run using the IR interpreter, capturing printed output.
pub fn run_compiled_capturing_interp(
    source: &str,
) -> Result<(Value, Vec<String>), crate::errors::PipelineError> {
    let program = prepare_program(source, None, &HashMap::new())?;
    let engine = super::interp::InterpEngine::compile(&program).map_err(runtime_error)?;
    let mut interp = super::interp::Interpreter::new(&engine);
    interp.with_captured_output();
    match interp.run() {
        VmResult::Value(v) => Ok((v, interp.captured_output())),
        VmResult::Error(e) => Err(runtime_error(e)),
    }
}

/// Run all #[test] and #[compile_error] functions using the IR interpreter.
/// Mirrors [`run_tests_jit_with_options`] block-for-block, swapping the JIT
/// engine for the interpreter.
pub fn run_tests_interp_with_options(
    path: &str,
    source: &str,
    externs: HashMap<String, PathBuf>,
) -> Result<Vec<TestResult>, crate::errors::PipelineError> {
    let mut program = crate::parser::parse(source)?;
    let source_dir = std::path::Path::new(path).parent().and_then(|p| p.to_str());
    super::jit::resolve_modules(&mut program.items, source_dir, &externs).map_err(runtime_error)?;

    let mut normal_items: Vec<crate::ast::Item> = Vec::new();
    let mut compile_error_fns: Vec<crate::ast::FnDef> = Vec::new();

    for item in program.items {
        if let crate::ast::Item::Function(ref f) = item {
            if f.attributes.iter().any(|a| a.name == "compile_error") {
                compile_error_fns.push(f.clone());
                continue;
            }
        }
        normal_items.push(item);
    }

    let mut normal_program = crate::ast::Program {
        items: normal_items,
        span: program.span,
    };
    super::jit::expand_derives(&mut normal_program);

    crate::type_checker::TypeChecker::new().check_program(&normal_program)?;

    let engine = super::interp::InterpEngine::compile(&normal_program).map_err(runtime_error)?;
    let interp = super::interp::Interpreter::new(&engine);

    // Collect test functions.
    let test_fns: Vec<&crate::ast::FnDef> = normal_program
        .items
        .iter()
        .filter_map(|item| {
            if let crate::ast::Item::Function(f) = item {
                if f.attributes.iter().any(|a| a.name == "test") {
                    Some(f)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let mut results = Vec::new();
    for test_fn in &test_fns {
        match interp.run_function(&test_fn.name) {
            VmResult::Value(_) => results.push(TestResult {
                name: test_fn.name.clone(),
                passed: true,
                error: None,
            }),
            VmResult::Error(e) => results.push(TestResult {
                name: test_fn.name.clone(),
                passed: false,
                error: Some(e),
            }),
        }
    }

    // Test compile_error functions: each must fail to type-check or lower.
    for ce_fn in &compile_error_fns {
        let ce_item = crate::ast::Item::Function(ce_fn.clone());
        let mut ce_items = normal_program.items.clone();
        ce_items.push(ce_item);
        let ce_program = crate::ast::Program {
            items: ce_items,
            span: program.span,
        };

        let tc_result = crate::type_checker::TypeChecker::new().check_program(&ce_program);
        if tc_result.is_err() {
            results.push(TestResult {
                name: ce_fn.name.clone(),
                passed: true,
                error: None,
            });
            continue;
        }

        match super::interp::InterpEngine::compile(&ce_program) {
            Err(_) => results.push(TestResult {
                name: ce_fn.name.clone(),
                passed: true,
                error: None,
            }),
            Ok(_) => results.push(TestResult {
                name: ce_fn.name.clone(),
                passed: false,
                error: Some(
                    "expected compilation error, but code compiled successfully".to_string(),
                ),
            }),
        }
    }

    Ok(results)
}

/// Compile and run with captured output (for testing).
pub fn run_compiled(source: &str) -> Result<Value, crate::errors::PipelineError> {
    run_compiled_with_options(source, None, HashMap::new())
}

/// Compile and run with an optional source-file path (for resolving sibling
/// modules) and caller-supplied externs (`name → path`).
///
/// `externs` mirrors rustc's `--extern` flag: it lets a package manager like
/// `tug` inject dependency entry points without the compiler needing to know
/// about a global package directory.
pub fn run_compiled_with_options(
    source: &str,
    source_path: Option<&str>,
    externs: HashMap<String, PathBuf>,
) -> Result<Value, crate::errors::PipelineError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_compiled_jit_with_options(source, source_path, externs)
    }
    #[cfg(target_arch = "wasm32")]
    {
        run_compiled_interp_with_options(source, source_path, externs)
    }
}

/// Compile and run, capturing printed output (for testing).
pub fn run_compiled_capturing(
    source: &str,
) -> Result<(Value, Vec<String>), crate::errors::PipelineError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_compiled_capturing_jit(source)
    }
    #[cfg(target_arch = "wasm32")]
    {
        run_compiled_capturing_interp(source)
    }
}

/// Run a program and capture its output (compatibility alias).
pub fn run_capturing(source: &str) -> Result<(Value, Vec<String>), crate::errors::PipelineError> {
    run_compiled_capturing(source)
}

/// Run a program, return its value (compatibility alias).
pub fn run(source: &str) -> Result<Value, crate::errors::PipelineError> {
    run_compiled(source)
}

/// Parse, type-check, lower to register IR, and render the IR disassembly.
///
/// Also verifies the program compiles all the way to native code, so callers
/// that use this purely as a compile check (e.g. `tug build`) fail on codegen
/// errors and not just type errors.
pub fn disassemble_source(
    path: &str,
    source: &str,
) -> Result<String, crate::errors::PipelineError> {
    let mut program = crate::parser::parse(source)?;
    let source_dir = std::path::Path::new(path).parent().and_then(|p| p.to_str());
    super::jit::resolve_modules(&mut program.items, source_dir, &HashMap::new())
        .map_err(runtime_error)?;
    super::jit::expand_derives(&mut program);
    crate::type_checker::TypeChecker::new().check_program(&program)?;

    let disassembly = super::jit::dump_ir(&program);

    // Compile end-to-end so a failed lowering surfaces as an error. On native
    // this exercises the full Cranelift codegen; on wasm (no Cranelift) the IR
    // interpreter's lowering is the equivalent compile check.
    #[cfg(not(target_arch = "wasm32"))]
    super::jit::JitEngine::compile(&program).map_err(runtime_error)?;
    #[cfg(target_arch = "wasm32")]
    super::interp::InterpEngine::compile(&program).map_err(runtime_error)?;

    Ok(disassembly)
}

/// Result of running a test suite.
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

/// Run all #[test] functions in source via the VM, and verify that
/// #[compile_error] functions fail to compile.
pub fn run_tests(
    path: &str,
    source: &str,
) -> Result<Vec<TestResult>, crate::errors::PipelineError> {
    run_tests_with_options(path, source, HashMap::new())
}

/// Same as [`run_tests`], but with caller-supplied externs (see
/// [`run_compiled_with_options`]).
pub fn run_tests_with_options(
    path: &str,
    source: &str,
    externs: HashMap<String, PathBuf>,
) -> Result<Vec<TestResult>, crate::errors::PipelineError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_tests_jit_with_options(path, source, externs)
    }
    #[cfg(target_arch = "wasm32")]
    {
        run_tests_interp_with_options(path, source, externs)
    }
}

/// Compile `source` through parse → type-check → ir_gen and return the
/// canonical serialized IR snapshot string. Does NOT run codegen or the JIT.
///
/// Useful for golden/snapshot tests that verify IR shape before
/// any Cranelift lowering happens.
#[cfg(not(target_arch = "wasm32"))]
pub fn gen_ir_snapshot(source: &str) -> Result<String, String> {
    let mut program = crate::parser::parse(source).map_err(|e| e.to_string())?;
    super::jit::expand_derives(&mut program);
    crate::type_checker::TypeChecker::new()
        .check_program(&program)
        .map_err(|e| e.to_string())?;

    let mut ir = super::jit::ir_gen::IrGen::new();
    ir.gen_program(&program);
    let functions = std::mem::take(&mut ir.functions);

    Ok(super::jit::ir_snapshot::serialize_program(&functions))
}

/// Return all type names that have built-in method dispatch.
/// Used by symbol consistency tests to ensure `symbols.rs` stays in sync.
/// **Must** be updated when a new `Value` variant receives a dispatch arm in
/// `VMRuntime::builtin_method`.
pub fn dispatched_type_names() -> Vec<&'static str> {
    vec![
        "Vec",
        "String",
        "HashMap",
        "HashSet",
        "BTreeMap",
        "BTreeSet",
        "VecDeque",
        "BinaryHeap",
        "char",
        "numeric",
        "Option",
        "Result",
        "enum",
        "struct",
        "Iterator",
        "tuple",
    ]
}
