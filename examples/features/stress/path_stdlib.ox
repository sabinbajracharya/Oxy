// === STRESS: std::path lexical path manipulation ===
//
// std::path operates purely lexically — it does not touch the filesystem.
// For "does this exist on disk?" use std::fs::exists.
//
// On Windows, std::path uses '\' as the separator. These tests use
// std::path::separator() to construct expected values portably where
// needed; the simple absolute-path cases on POSIX use '/' directly.

fn sep() -> String {
    std::path::separator()
}

#[test]
fn test_basename_simple() {
    assert_eq(std::path::basename("/a/b/c.txt"), "c.txt");
}

#[test]
fn test_basename_no_slash() {
    assert_eq(std::path::basename("file.txt"), "file.txt");
}

#[test]
fn test_basename_trailing_slash() {
    // file_name() returns "" for a path ending in "/" with no name.
    assert_eq(std::path::basename("/a/b/"), "b");
}

#[test]
fn test_basename_empty() {
    assert_eq(std::path::basename(""), "");
}

#[test]
fn test_dirname_simple() {
    assert_eq(std::path::dirname("/a/b/c.txt"), "/a/b");
}

#[test]
fn test_dirname_root_child() {
    assert_eq(std::path::dirname("/foo"), "/");
}

#[test]
fn test_dirname_bare_name() {
    assert_eq(std::path::dirname("foo"), "");
}

#[test]
fn test_stem_with_extension() {
    assert_eq(std::path::stem("/a/b/c.txt"), "c");
}

#[test]
fn test_stem_double_extension() {
    // "foo.tar.gz" — stem strips only the final extension.
    assert_eq(std::path::stem("foo.tar.gz"), "foo.tar");
}

#[test]
fn test_stem_no_extension() {
    assert_eq(std::path::stem("README"), "README");
}

#[test]
fn test_extension_present() {
    let e = std::path::extension("/a/b/c.txt");
    assert(e.is_some());
    assert_eq(e.unwrap(), "txt");
}

#[test]
fn test_extension_missing() {
    let e = std::path::extension("README");
    assert(e.is_none());
}

#[test]
fn test_extension_double() {
    // Only the trailing extension is returned.
    let e = std::path::extension("foo.tar.gz");
    assert_eq(e.unwrap(), "gz");
}

#[test]
fn test_extension_dotfile() {
    // Leading-dot files like ".gitignore" have no extension in Rust's model.
    let e = std::path::extension(".gitignore");
    assert(e.is_none());
}

#[test]
fn test_with_extension_replace() {
    let s = sep();
    let expected = "foo" + s + "bar.json";
    assert_eq(std::path::with_extension("foo/bar.txt", "json"), expected);
}

#[test]
fn test_with_extension_add() {
    let s = sep();
    let expected = "foo" + s + "bar.json";
    assert_eq(std::path::with_extension("foo/bar", "json"), expected);
}

#[test]
fn test_with_extension_remove() {
    let s = sep();
    let expected = "foo" + s + "bar";
    assert_eq(std::path::with_extension("foo/bar.txt", ""), expected);
}

#[test]
fn test_with_file_name_replace() {
    let s = sep();
    let expected = "foo" + s + "baz.rs";
    assert_eq(std::path::with_file_name("foo/bar.txt", "baz.rs"), expected);
}

#[test]
fn test_join_two_parts() {
    let s = sep();
    let expected = "a" + s + "b";
    let p = std::path::join(["a".to_string(), "b".to_string()]);
    assert_eq(p, expected);
}

#[test]
fn test_join_three_parts() {
    let s = sep();
    let expected = "a" + s + "b" + s + "c.txt";
    let p = std::path::join([
        "a".to_string(),
        "b".to_string(),
        "c.txt".to_string(),
    ]);
    assert_eq(p, expected);
}

#[test]
fn test_join_absolute_resets() {
    // PathBuf::push of an absolute resets the buffer — this matches Rust.
    let p = std::path::join(["a".to_string(), "/b".to_string()]);
    assert_eq(p, "/b");
}

#[test]
fn test_join_empty_vec_is_empty() {
    let p = std::path::join([]);
    assert_eq(p, "");
}

#[test]
fn test_is_absolute_unix() {
    assert(std::path::is_absolute("/a/b"));
    assert(!std::path::is_absolute("a/b"));
}

#[test]
fn test_is_relative_unix() {
    assert(std::path::is_relative("a/b"));
    assert(!std::path::is_relative("/a/b"));
}

#[test]
fn test_components_absolute() {
    let v = std::path::components("/a/b/c");
    // Rust gives ["/", "a", "b", "c"] on POSIX.
    assert(v.len() >= 3);
    assert_eq(v[v.len() - 1], "c");
    assert_eq(v[v.len() - 2], "b");
}

#[test]
fn test_components_relative() {
    let v = std::path::components("a/b/c");
    assert_eq(v.len(), 3);
    assert_eq(v[0], "a");
    assert_eq(v[2], "c");
}

#[test]
fn test_normalize_collapses_curdir() {
    let s = sep();
    let expected = "a" + s + "b";
    assert_eq(std::path::normalize("a/./b"), expected);
}

#[test]
fn test_normalize_collapses_parent() {
    let s = sep();
    let expected = "a" + s + "c";
    assert_eq(std::path::normalize("a/b/../c"), expected);
}

#[test]
fn test_normalize_preserves_leading_parent() {
    // Without a root, leading ".." cannot be collapsed.
    let s = sep();
    let expected = ".." + s + "a";
    assert_eq(std::path::normalize("../a"), expected);
}

#[test]
fn test_normalize_empty_becomes_dot() {
    assert_eq(std::path::normalize(""), ".");
}

#[test]
fn test_normalize_only_curdir() {
    assert_eq(std::path::normalize("./."), ".");
}

#[test]
fn test_separator_nonempty() {
    let s = std::path::separator();
    assert(s.len() >= 1);
}
