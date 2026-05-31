// Unreachable-code detection: statements after an unconditional exit are
// rejected at compile time.

#[compile_error]
fn after_return() {
    return;
    val x = 1;  // unreachable
}

#[compile_error]
fn after_break_in_loop() {
    loop {
        break;
        val x = 1;  // unreachable
    }
}

#[compile_error]
fn after_continue_in_loop() {
    loop {
        continue;
        val x = 1;  // unreachable
    }
}

#[compile_error]
fn after_panic() {
    panic("boom");
    val x = 1;  // unreachable
}

fn main() {}
