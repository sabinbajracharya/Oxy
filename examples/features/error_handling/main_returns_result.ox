// === Feature: `fn main() -> Result<(), E>` propagates `?` to exit code ===
// With this, the user can write `val v = thing()?;` directly in main.
// If main returns `Err(_)`, the CLI prints the error and exits 1.
// (The CLI-level reporting is verified out-of-band; here we just verify
// that the type checker / compiler accept the pattern.)

fn fails() -> Result<Int, String> {
    Err("boom".to_string())
}

fn ok_value() -> Result<Int, String> {
    Ok(42)
}

fn imitate_main_ok() -> Result<(), String> {
    val _ = ok_value()?;
    Ok(())
}

fn imitate_main_err() -> Result<(), String> {
    val _ = fails()?;
    Ok(())
}

#[test]
fn test_main_returning_result_unit_parses_and_typechecks() {
    assert_eq(imitate_main_ok(), Ok(()));
}

#[test]
fn test_main_pattern_propagates_err() {
    match imitate_main_err() {
        Err(msg) => assert_eq(msg, "boom"),
        Ok(_) => panic("expected Err"),
    }
}
