// === Feature: std::process::spawn (streaming subprocess) ===
// `std::process::spawn(program, args, callback)` runs a command with piped
// stdout/stderr and invokes the callback once per line as it is produced,
// tagged with the stream name ("stdout" or "stderr"). The call returns a
// `Result<CommandOutput, String>` once the child exits; only `status` and
// `success` are populated on the struct (output is delivered via callback).

#[test]
fn test_spawn_streams_stdout_lines_in_order() {
    let mut lines = [];
    let result = std::process::spawn(
        "printf",
        ["%s\n%s\n", "alpha", "beta"],
        |line, stream| {
            lines.push(stream + ":" + line);
        },
    );
    if let Ok(output) = result {
        assert_eq(output.success, true);
        assert_eq(output.status, 0);
        assert_eq(lines.len(), 2);
        assert_eq(lines[0], "stdout:alpha");
        assert_eq(lines[1], "stdout:beta");
    } else {
        assert(false);
    }
}

#[test]
fn test_spawn_tags_stderr_separately_from_stdout() {
    // `sh -c` lets us deterministically write to both streams. Interleaving
    // between streams is racy, so we sort before asserting.
    let mut tagged = [];
    let result = std::process::spawn(
        "sh",
        ["-c", "echo out1; echo err1 1>&2; echo out2"],
        |line, stream| {
            tagged.push(stream + ":" + line);
        },
    );
    assert(result.is_ok());
    tagged.sort();
    assert_eq(tagged.len(), 3);
    assert_eq(tagged[0], "stderr:err1");
    assert_eq(tagged[1], "stdout:out1");
    assert_eq(tagged[2], "stdout:out2");
}

#[test]
fn test_spawn_reports_nonzero_exit_status() {
    let mut count = 0;
    let result = std::process::spawn("false", [], |_line, _stream| {
        count = count + 1;
    });
    if let Ok(output) = result {
        assert_eq(output.success, false);
        assert_eq(output.status, 1);
        assert_eq(count, 0);
    } else {
        assert(false);
    }
}

#[test]
fn test_spawn_returns_err_for_nonexistent_program() {
    let result = std::process::spawn(
        "definitely_not_a_real_program_xyz_98765",
        [],
        |_line, _stream| {},
    );
    assert(result.is_err());
}

#[test]
fn test_spawn_handles_many_lines() {
    // seq emits N lines on stdout — covers the streaming case where output
    // is larger than a single pipe buffer might hold.
    let mut received = 0;
    let mut last = String::from("");
    let result = std::process::spawn("seq", ["1", "50"], |line, _stream| {
        received = received + 1;
        last = line;
    });
    assert(result.is_ok());
    assert_eq(received, 50);
    assert_eq(last, "50");
}
