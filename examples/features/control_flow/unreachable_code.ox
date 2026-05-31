// Unreachable-code detection: statements after an unconditional exit are
// rejected at compile time.

#[compile_error]
fn after_return() {
    return;
    let x = 1;  // unreachable
}

#[compile_error]
fn after_break_in_loop() {
    loop {
        break;
        let x = 1;  // unreachable
    }
}

#[compile_error]
fn after_continue_in_loop() {
    loop {
        continue;
        let x = 1;  // unreachable
    }
}

#[compile_error]
fn after_panic() {
    panic!("boom");
    let x = 1;  // unreachable
}

fn main() {}
