// vm/api.rs — Public crate entry points for compiling and running Oxy programs.
//
// Extracted from vm/mod.rs to keep that file focused on the Vm struct and its
// execution loop.

use std::collections::HashMap;
use std::path::PathBuf;

use super::VmResult;
use crate::types::Value;

// ── JIT entry points ──────────────────────────────────────────────────

/// Compile and run using the Cranelift JIT backend.
pub fn run_compiled_jit(source: &str) -> Result<Value, crate::errors::FerriError> {
    run_compiled_jit_with_options(source, None, HashMap::new())
}

/// Compile and run with JIT, with optional source path and externs.
pub fn run_compiled_jit_with_options(
    source: &str,
    source_path: Option<&str>,
    externs: HashMap<String, PathBuf>,
) -> Result<Value, crate::errors::FerriError> {
    let program = crate::parser::parse(source)?;
    crate::type_checker::TypeChecker::new().check_program(&program)?;
    let chunk =
        crate::compiler::Compiler::new_with_options(source_path, externs).compile(&program)?;
    let mut jit_vm =
        super::jit::JitVm::new(chunk).map_err(|e| crate::errors::FerriError::Runtime {
            message: e,
            line: 0,
            column: 0,
        })?;
    match jit_vm.run() {
        VmResult::Value(v) => Ok(v),
        VmResult::Error(e) => Err(crate::errors::FerriError::Runtime {
            message: e,
            line: 0,
            column: 0,
        }),
    }
}

/// Compile and run with JIT, capturing printed output.
pub fn run_compiled_capturing_jit(
    source: &str,
) -> Result<(Value, Vec<String>), crate::errors::FerriError> {
    let program = crate::parser::parse(source)?;
    crate::type_checker::TypeChecker::new().check_program(&program)?;
    let chunk = crate::compiler::Compiler::new().compile(&program)?;
    let mut jit_vm = super::jit::JitVm::with_captured_output(chunk).map_err(|e| {
        crate::errors::FerriError::Runtime {
            message: e,
            line: 0,
            column: 0,
        }
    })?;
    match jit_vm.run() {
        VmResult::Value(v) => Ok((v, jit_vm.captured_output())),
        VmResult::Error(e) => Err(crate::errors::FerriError::Runtime {
            message: e,
            line: 0,
            column: 0,
        }),
    }
}

/// JIT-based conformance alias for run.
pub fn run_jit(source: &str) -> Result<Value, crate::errors::FerriError> {
    run_compiled_jit(source)
}

/// Run all #[test] and #[compile_error] functions using the JIT backend.
pub fn run_tests_jit(
    path: &str,
    source: &str,
) -> Result<Vec<TestResult>, crate::errors::FerriError> {
    run_tests_jit_with_options(path, source, HashMap::new())
}

/// Same as run_tests_jit with externs.
pub fn run_tests_jit_with_options(
    path: &str,
    source: &str,
    externs: HashMap<String, PathBuf>,
) -> Result<Vec<TestResult>, crate::errors::FerriError> {
    let program = crate::parser::parse(source)?;

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

    let normal_program = crate::ast::Program {
        items: normal_items,
        span: program.span,
    };

    crate::type_checker::TypeChecker::new().check_program(&normal_program)?;
    let chunk = crate::compiler::Compiler::new_for_tests(Some(path))
        .with_externs(externs.clone())
        .compile(&normal_program)?;

    // Build JIT engine once for all tests
    let mut jit_vm =
        super::jit::JitVm::new(chunk).map_err(|e| crate::errors::FerriError::Runtime {
            message: e,
            line: 0,
            column: 0,
        })?;

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

        let compile_result = crate::compiler::Compiler::new_for_tests(Some(path))
            .with_externs(externs.clone())
            .compile(&ce_program);
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

/// Compile and run with captured output (for testing).
pub fn run_compiled(source: &str) -> Result<Value, crate::errors::FerriError> {
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
) -> Result<Value, crate::errors::FerriError> {
    run_compiled_jit_with_options(source, source_path, externs)
}

/// Compile and run, capturing printed output (for testing).
pub fn run_compiled_capturing(
    source: &str,
) -> Result<(Value, Vec<String>), crate::errors::FerriError> {
    run_compiled_capturing_jit(source)
}

/// Run a program and capture its output (compatibility alias).
pub fn run_capturing(source: &str) -> Result<(Value, Vec<String>), crate::errors::FerriError> {
    run_compiled_capturing(source)
}

/// Run a program, return its value (compatibility alias).
pub fn run(source: &str) -> Result<Value, crate::errors::FerriError> {
    run_compiled(source)
}

/// Parse, type-check, and disassemble a source file.
pub fn disassemble_source(_path: &str, _source: &str) -> Result<String, crate::errors::FerriError> {
    Err(crate::errors::FerriError::Runtime {
        message: "disassemble not yet wired for IR path".to_string(),
        line: 0,
        column: 0,
    })
}

/// Result of running a test suite.
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

/// Run all #[test] functions in source via the VM, and verify that
/// #[compile_error] functions fail to compile.
pub fn run_tests(path: &str, source: &str) -> Result<Vec<TestResult>, crate::errors::FerriError> {
    run_tests_with_options(path, source, HashMap::new())
}

/// Same as [`run_tests`], but with caller-supplied externs (see
/// [`run_compiled_with_options`]).
pub fn run_tests_with_options(
    path: &str,
    source: &str,
    externs: HashMap<String, PathBuf>,
) -> Result<Vec<TestResult>, crate::errors::FerriError> {
    run_tests_jit_with_options(path, source, externs)
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
