// vm/api.rs — Public crate entry points for compiling and running Oxy programs.
//
// Extracted from vm/mod.rs to keep that file focused on the Vm struct and its
// execution loop.
//
// Backend selection lives in *one* place: the `ExecutionBackend` seam near the
// bottom. The two backends (Cranelift JIT on native, IR interpreter elsewhere)
// each implement that trait, and `ActiveBackend` is the single `#[cfg]`-selected
// alias the public dispatchers route through — so target selection is one
// polymorphic call rather than a `#[cfg]` branch repeated at every entry point.

use std::collections::HashMap;
use std::path::PathBuf;

use super::VmResult;
use crate::ast::Program;
use crate::errors::PipelineError;
use crate::types::Value;

/// Wrap a backend message string as a runtime `PipelineError` (line/column unknown).
fn runtime_error(message: String) -> PipelineError {
    PipelineError::Runtime {
        message,
        line: 0,
        column: 0,
    }
}

// ── JIT entry points (native only) ────────────────────────────────────
//
// Cranelift emits host machine code and is unavailable on `wasm32`, so the
// whole JIT surface is gated to non-wasm. On wasm, execution runs through the
// portable IR interpreter (see the `*_interp_*` entry points below). The
// `ExecutionBackend` seam picks between them per target.

/// Compile and run using the Cranelift JIT backend.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_compiled_jit(source: &str) -> Result<Value, PipelineError> {
    run_compiled_jit_with_options(source, None, HashMap::new())
}

/// Compile and run with JIT, with optional source path and externs.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_compiled_jit_with_options(
    source: &str,
    source_path: Option<&str>,
    externs: HashMap<String, PathBuf>,
) -> Result<Value, PipelineError> {
    let mut jit_vm = super::jit::JitVm::compile_with_options(source, source_path, externs)
        .map_err(runtime_error)?;
    match jit_vm.run() {
        VmResult::Value(v) => Ok(v),
        VmResult::Error(e) => Err(runtime_error(e)),
    }
}

/// Compile and run with JIT, capturing printed output.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_compiled_capturing_jit(source: &str) -> Result<(Value, Vec<String>), PipelineError> {
    let mut jit_vm = super::jit::JitVm::compile(source).map_err(runtime_error)?;
    jit_vm.with_captured_output();
    match jit_vm.run() {
        VmResult::Value(v) => Ok((v, jit_vm.captured_output())),
        VmResult::Error(e) => Err(runtime_error(e)),
    }
}

/// Run all #[test] and #[compile_error] functions using the JIT backend.
/// Imported directly by the `jit_interp_parity` test to drive the JIT side.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_tests_jit_with_options(
    path: &str,
    source: &str,
    externs: HashMap<String, PathBuf>,
) -> Result<Vec<TestResult>, PipelineError> {
    run_tests_with_backend::<JitBackend>(path, source, externs)
}

// ── IR interpreter entry points (all targets) ─────────────────────────
//
// The portable register-IR interpreter (`vm::interp`) executes the same IR the
// Cranelift backend compiles, delegating runtime semantics to the shared `oxy_*`
// FFI. It is the execution backend on `wasm32`, where Cranelift is unavailable.
// These mirror the `*_jit_*` functions above one-to-one.
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
) -> Result<Program, PipelineError> {
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
) -> Result<Value, PipelineError> {
    let program = prepare_program(source, source_path, &externs)?;
    let engine = super::interp::InterpEngine::compile(&program).map_err(runtime_error)?;
    let interp = super::interp::Interpreter::new(&engine);
    match interp.run() {
        VmResult::Value(v) => Ok(v),
        VmResult::Error(e) => Err(runtime_error(e)),
    }
}

/// Compile and run using the IR interpreter, capturing printed output.
pub fn run_compiled_capturing_interp(source: &str) -> Result<(Value, Vec<String>), PipelineError> {
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
pub fn run_tests_interp_with_options(
    path: &str,
    source: &str,
    externs: HashMap<String, PathBuf>,
) -> Result<Vec<TestResult>, PipelineError> {
    run_tests_with_backend::<InterpBackend>(path, source, externs)
}

// ── Backend seam ──────────────────────────────────────────────────────
//
// `ExecutionBackend` is the one place where the two engines differ from the
// entry points' point of view. Each method delegates to the concrete
// `*_jit_*` / `*_interp_*` machinery above; the trait adds the polymorphic
// seam, not new behavior. `ActiveBackend` selects the backend per target with
// a single `#[cfg]`, and the shared orchestration (`run_tests_with_backend`)
// stays backend-agnostic.

/// One pluggable execution backend. Implemented by [`JitBackend`] (native
/// Cranelift) and [`InterpBackend`] (portable IR interpreter). The
/// type-checking front-end (`prepare_program`) is shared and runs before any
/// backend-specific work; these methods cover only what genuinely differs.
trait ExecutionBackend {
    /// Compile `source` and run `main`, returning its final value.
    fn run_with_options(
        source: &str,
        source_path: Option<&str>,
        externs: HashMap<String, PathBuf>,
    ) -> Result<Value, PipelineError>;

    /// Compile `source` and run `main`, capturing printed output.
    fn run_capturing(source: &str) -> Result<(Value, Vec<String>), PipelineError>;

    /// Compile a type-checked `program` once, then run each named `#[test]`
    /// function against it. Returns one [`TestResult`] per name, in order.
    fn run_test_functions(
        program: &Program,
        test_names: &[String],
    ) -> Result<Vec<TestResult>, PipelineError>;

    /// Compile-check a type-checked `program` without running it. Used to
    /// confirm `#[compile_error]` functions are rejected at lowering/codegen
    /// time and to give `disassemble_source` an end-to-end compile check.
    fn compile_check(program: &Program) -> Result<(), PipelineError>;
}

/// Native Cranelift JIT backend.
#[cfg(not(target_arch = "wasm32"))]
enum JitBackend {}

#[cfg(not(target_arch = "wasm32"))]
impl ExecutionBackend for JitBackend {
    fn run_with_options(
        source: &str,
        source_path: Option<&str>,
        externs: HashMap<String, PathBuf>,
    ) -> Result<Value, PipelineError> {
        run_compiled_jit_with_options(source, source_path, externs)
    }

    fn run_capturing(source: &str) -> Result<(Value, Vec<String>), PipelineError> {
        run_compiled_capturing_jit(source)
    }

    fn run_test_functions(
        program: &Program,
        test_names: &[String],
    ) -> Result<Vec<TestResult>, PipelineError> {
        let engine = super::jit::JitEngine::compile(program).map_err(runtime_error)?;
        let mut jit_vm = super::jit::JitVm {
            engine,
            output: None,
        };
        let mut results = Vec::new();
        for name in test_names {
            if jit_vm.engine.functions.contains_key(name) {
                results.push(test_result(name, jit_vm.run_function(name)));
            } else {
                results.push(TestResult {
                    name: name.clone(),
                    passed: false,
                    error: Some("JIT: function not found".into()),
                });
            }
        }
        Ok(results)
    }

    fn compile_check(program: &Program) -> Result<(), PipelineError> {
        super::jit::JitEngine::compile(program)
            .map(|_| ())
            .map_err(runtime_error)
    }
}

/// Portable register-IR interpreter backend.
enum InterpBackend {}

impl ExecutionBackend for InterpBackend {
    fn run_with_options(
        source: &str,
        source_path: Option<&str>,
        externs: HashMap<String, PathBuf>,
    ) -> Result<Value, PipelineError> {
        run_compiled_interp_with_options(source, source_path, externs)
    }

    fn run_capturing(source: &str) -> Result<(Value, Vec<String>), PipelineError> {
        run_compiled_capturing_interp(source)
    }

    fn run_test_functions(
        program: &Program,
        test_names: &[String],
    ) -> Result<Vec<TestResult>, PipelineError> {
        let engine = super::interp::InterpEngine::compile(program).map_err(runtime_error)?;
        let interp = super::interp::Interpreter::new(&engine);
        Ok(test_names
            .iter()
            .map(|name| test_result(name, interp.run_function(name)))
            .collect())
    }

    fn compile_check(program: &Program) -> Result<(), PipelineError> {
        super::interp::InterpEngine::compile(program)
            .map(|_| ())
            .map_err(runtime_error)
    }
}

/// The backend selected for this build: Cranelift JIT on native, IR
/// interpreter on `wasm32`. This is the **only** `#[cfg(target_arch)]` switch
/// for execution — every public dispatcher routes through it.
#[cfg(not(target_arch = "wasm32"))]
type ActiveBackend = JitBackend;
#[cfg(target_arch = "wasm32")]
type ActiveBackend = InterpBackend;

/// Build a [`TestResult`] from a function's [`VmResult`].
fn test_result(name: &str, outcome: VmResult) -> TestResult {
    match outcome {
        VmResult::Value(_) => TestResult {
            name: name.to_string(),
            passed: true,
            error: None,
        },
        VmResult::Error(e) => TestResult {
            name: name.to_string(),
            passed: false,
            error: Some(e),
        },
    }
}

/// Backend-agnostic test-suite orchestration: the front-end (parse → resolve →
/// split `#[compile_error]` fns → expand derives → type-check) plus the
/// `#[compile_error]` rejection loop are identical across backends, so they
/// live here once. Only `B::run_test_functions` and `B::compile_check` differ.
fn run_tests_with_backend<B: ExecutionBackend>(
    path: &str,
    source: &str,
    externs: HashMap<String, PathBuf>,
) -> Result<Vec<TestResult>, PipelineError> {
    let mut program = crate::parser::parse(source)?;
    let source_dir = std::path::Path::new(path).parent().and_then(|p| p.to_str());
    super::jit::resolve_modules(&mut program.items, source_dir, &externs).map_err(runtime_error)?;

    // Split off the `#[compile_error]` functions — they are expected to fail,
    // so they must not participate in the type-check/compile of the suite.
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

    let mut normal_program = Program {
        items: normal_items,
        span: program.span,
    };
    super::jit::expand_derives(&mut normal_program);
    crate::type_checker::TypeChecker::new().check_program(&normal_program)?;

    // Run the `#[test]` functions on the chosen backend.
    let test_names: Vec<String> = normal_program
        .items
        .iter()
        .filter_map(|item| match item {
            crate::ast::Item::Function(f) if f.attributes.iter().any(|a| a.name == "test") => {
                Some(f.name.clone())
            }
            _ => None,
        })
        .collect();
    let mut results = B::run_test_functions(&normal_program, &test_names)?;

    // Each `#[compile_error]` function must fail to type-check or to lower.
    for ce_fn in &compile_error_fns {
        let mut ce_items = normal_program.items.clone();
        ce_items.push(crate::ast::Item::Function(ce_fn.clone()));
        let ce_program = Program {
            items: ce_items,
            span: program.span,
        };

        let rejected = crate::type_checker::TypeChecker::new()
            .check_program(&ce_program)
            .is_err()
            || B::compile_check(&ce_program).is_err();

        results.push(TestResult {
            name: ce_fn.name.clone(),
            passed: rejected,
            error: if rejected {
                None
            } else {
                Some("expected compilation error, but code compiled successfully".to_string())
            },
        });
    }

    Ok(results)
}

// ── Public dispatchers ────────────────────────────────────────────────

/// Compile and run with captured output (for testing).
pub fn run_compiled(source: &str) -> Result<Value, PipelineError> {
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
) -> Result<Value, PipelineError> {
    ActiveBackend::run_with_options(source, source_path, externs)
}

/// Compile and run, capturing printed output (for testing).
pub fn run_compiled_capturing(source: &str) -> Result<(Value, Vec<String>), PipelineError> {
    ActiveBackend::run_capturing(source)
}

/// Run a program and capture its output (compatibility alias).
pub fn run_capturing(source: &str) -> Result<(Value, Vec<String>), PipelineError> {
    run_compiled_capturing(source)
}

/// Run a program, return its value (compatibility alias).
pub fn run(source: &str) -> Result<Value, PipelineError> {
    run_compiled(source)
}

/// Parse, type-check, lower to register IR, and render the IR disassembly.
///
/// Also verifies the program compiles all the way to native code, so callers
/// that use this purely as a compile check (e.g. `tug build`) fail on codegen
/// errors and not just type errors.
pub fn disassemble_source(path: &str, source: &str) -> Result<String, PipelineError> {
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
    ActiveBackend::compile_check(&program)?;

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
pub fn run_tests(path: &str, source: &str) -> Result<Vec<TestResult>, PipelineError> {
    run_tests_with_options(path, source, HashMap::new())
}

/// Same as [`run_tests`], but with caller-supplied externs (see
/// [`run_compiled_with_options`]).
pub fn run_tests_with_options(
    path: &str,
    source: &str,
    externs: HashMap<String, PathBuf>,
) -> Result<Vec<TestResult>, PipelineError> {
    run_tests_with_backend::<ActiveBackend>(path, source, externs)
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
