// vm/api.rs — Public crate entry points for compiling and running Oxy programs.
//
// Extracted from vm/mod.rs to keep that file focused on the Vm struct and its
// execution loop.

use super::{disassemble_chunk, Vm, VmResult};
use crate::types::Value;

/// Compile and run with captured output (for testing).
pub fn run_compiled(source: &str) -> Result<Value, crate::errors::FerriError> {
    let program = crate::parser::parse(source)?;
    crate::type_checker::TypeChecker::new().check_program(&program)?;
    let chunk = crate::compiler::Compiler::new().compile(&program)?;
    let mut vm = Vm::new(chunk);
    match vm.run() {
        VmResult::Value(v) => Ok(v),
        VmResult::Error(e) => Err(crate::errors::FerriError::Runtime {
            message: e,
            line: 0,
            column: 0,
        }),
    }
}

/// Compile and run, capturing printed output (for testing).
pub fn run_compiled_capturing(
    source: &str,
) -> Result<(Value, Vec<String>), crate::errors::FerriError> {
    let program = crate::parser::parse(source)?;
    crate::type_checker::TypeChecker::new().check_program(&program)?;
    let chunk = crate::compiler::Compiler::new().compile(&program)?;
    let mut vm = Vm::with_captured_output(chunk);
    match vm.run() {
        VmResult::Value(v) => Ok((v, vm.captured_output())),
        VmResult::Error(e) => Err(crate::errors::FerriError::Runtime {
            message: e,
            line: 0,
            column: 0,
        }),
    }
}

/// Run a program and capture its output (compatibility alias).
pub fn run_capturing(source: &str) -> Result<(Value, Vec<String>), crate::errors::FerriError> {
    run_compiled_capturing(source)
}

/// Run a program, return its value (compatibility alias).
pub fn run(source: &str) -> Result<Value, crate::errors::FerriError> {
    run_compiled(source)
}

/// Parse, type-check, compile, and disassemble a source file to a debug string.
pub fn disassemble_source(path: &str, source: &str) -> Result<String, crate::errors::FerriError> {
    let program = crate::parser::parse(source)?;
    crate::type_checker::TypeChecker::new().check_program(&program)?;
    let chunk = crate::compiler::Compiler::new_for_tests(Some(path)).compile(&program)?;
    Ok(disassemble_chunk(&chunk))
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
    let program = crate::parser::parse(source)?;

    // Split: normal items vs #[compile_error] functions
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

    // Type-check and compile normal items (must succeed)
    crate::type_checker::TypeChecker::new().check_program(&normal_program)?;
    let chunk = crate::compiler::Compiler::new_for_tests(Some(path)).compile(&normal_program)?;

    // Run #[test] functions
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
        let mut chunk = chunk.clone();
        if let Some(&ip) = chunk.functions.get(&test_fn.name) {
            chunk.entry_point = ip;
        }
        let mut vm = Vm::new(chunk);
        match vm.run() {
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

    // Test #[compile_error] functions — each must FAIL to compile
    for ce_fn in &compile_error_fns {
        let ce_item = crate::ast::Item::Function(ce_fn.clone());
        let mut ce_items = normal_program.items.clone();
        ce_items.push(ce_item);
        let ce_program = crate::ast::Program {
            items: ce_items,
            span: program.span,
        };

        // Try type-check first (catches visibility errors, type errors, etc.)
        let tc_result = crate::type_checker::TypeChecker::new().check_program(&ce_program);
        if tc_result.is_err() {
            results.push(TestResult {
                name: ce_fn.name.clone(),
                passed: true,
                error: None,
            });
            continue;
        }

        // Try compilation (catches compiler-level errors)
        let compile_result =
            crate::compiler::Compiler::new_for_tests(Some(path)).compile(&ce_program);
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
