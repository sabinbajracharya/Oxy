// === STRESS: std::args CLI argument parsing ===
//
// Tests use parse_from(List<String>) so they're independent of the
// actual process argv. argv[0] is the program/script path; argv[1..]
// is parsed using the rules:
//   --key=val     flags["key"] = "val"
//   --key         flags["key"] = ""        (presence)
//   -k=val        flags["k"]   = "val"
//   -k            flags["k"]   = ""
//   --            terminator
//   -             positional
//   else          positional

#[test]
fn test_empty_argv() {
    val a = std::args::parse_from([]);
    assert::eq(a.program, "");
    assert::eq(a.flags.len(), 0);
    assert::eq(a.positionals.len(), 0);
}

#[test]
fn test_program_extracted() {
    val a = std::args::parse_from(["script.ox".to_string()]);
    assert::eq(a.program, "script.ox");
    assert::eq(a.flags.len(), 0);
    assert::eq(a.positionals.len(), 0);
}

#[test]
fn test_long_flag_presence_only() {
    val a = std::args::parse_from(["p".to_string(), "--verbose".to_string()]);
    assert::eq(a.flags.get("verbose").unwrap(), "");
    assert::true(a.flags.contains_key("verbose"));
}

#[test]
fn test_long_flag_with_value() {
    val a = std::args::parse_from(["p".to_string(), "--name=alice".to_string()]);
    assert::eq(a.flags.get("name").unwrap(), "alice");
}

#[test]
fn test_short_flag_with_value() {
    val a = std::args::parse_from(["p".to_string(), "-k=v".to_string()]);
    assert::eq(a.flags.get("k").unwrap(), "v");
}

#[test]
fn test_short_flag_presence() {
    val a = std::args::parse_from(["p".to_string(), "-v".to_string()]);
    assert::eq(a.flags.get("v").unwrap(), "");
}

#[test]
fn test_positionals_only() {
    val a = std::args::parse_from([
        "p".to_string(),
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
    ]);
    assert::eq(a.positionals.len(), 3);
    assert::eq(a.positionals[0], "a");
    assert::eq(a.positionals[1], "b");
    assert::eq(a.positionals[2], "c");
    assert::eq(a.flags.len(), 0);
}

#[test]
fn test_mixed_flags_and_positionals() {
    val a = std::args::parse_from([
        "p".to_string(),
        "--verbose".to_string(),
        "file1".to_string(),
        "--name=bob".to_string(),
        "file2".to_string(),
    ]);
    assert::eq(a.flags.len(), 2);
    assert::eq(a.flags.get("verbose").unwrap(), "");
    assert::eq(a.flags.get("name").unwrap(), "bob");
    assert::eq(a.positionals.len(), 2);
    assert::eq(a.positionals[0], "file1");
    assert::eq(a.positionals[1], "file2");
}

#[test]
fn test_double_dash_terminator() {
    val a = std::args::parse_from([
        "p".to_string(),
        "--verbose".to_string(),
        "--".to_string(),
        "--not-a-flag".to_string(),
        "x".to_string(),
    ]);
    assert::eq(a.flags.len(), 1);
    assert::eq(a.flags.get("verbose").unwrap(), "");
    assert::eq(a.positionals[0], "--not-a-flag");
    assert::eq(a.positionals[1], "x");
}

#[test]
fn test_bare_dash_is_positional() {
    val a = std::args::parse_from(["p".to_string(), "-".to_string()]);
    assert::eq(a.positionals[0], "-");
    assert::eq(a.flags.len(), 0);
}

#[test]
fn test_value_with_embedded_equals() {
    val a = std::args::parse_from(["p".to_string(), "--query=a=b=c".to_string()]);
    assert::eq(a.flags.get("query").unwrap(), "a=b=c");
}

#[test]
fn test_later_flag_overrides_earlier() {
    val a = std::args::parse_from([
        "p".to_string(),
        "--name=first".to_string(),
        "--name=second".to_string(),
    ]);
    assert::eq(a.flags.get("name").unwrap(), "second");
}
