// cli_utils — shared CLI output helpers for Oxy showcase projects.
//
// Other showcase projects depend on this via:
//   [dependencies]
//   cli_utils = { path = "../cli_utils" }

pub fn header(text: String) {
    let bar = "=".repeat(text.len() + 4);
    println(bar);
    println("  " + text);
    println(bar);
}

pub fn info(text: String) {
    println("  " + text);
}

pub fn success(text: String) {
    println("  ok  " + text);
}

pub fn warn(text: String) {
    println("  warn  " + text);
}

pub fn fail(text: String) {
    println("  FAIL  " + text);
}

pub fn die(text: String) {
    fail(text);
    std::process::exit(1);
}
