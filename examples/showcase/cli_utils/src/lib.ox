// cli_utils — shared CLI output helpers for Oxy showcase projects.
//
// Other showcase projects depend on this via:
//   [dependencies]
//   cli_utils = { path = "../cli_utils" }

pub fn header(text: String) {
    val bar = "=".repeat(text.len() + 4);
    io::println(bar);
    io::println("  " + text);
    io::println(bar);
}

pub fn info(text: String) {
    io::println("  " + text);
}

pub fn success(text: String) {
    io::println("  ok  " + text);
}

pub fn warn(text: String) {
    io::println("  warn  " + text);
}

pub fn fail(text: String) {
    io::println("  FAIL  " + text);
}

pub fn die(text: String) {
    fail(text);
    std::process::exit(1);
}
