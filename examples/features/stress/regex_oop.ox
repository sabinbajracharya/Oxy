// === STRESS: OOP-style Regex::new(pattern) ===

#[test]
fn test_regex_new_and_is_match() {
    val re = std::regex::Regex::new("\\d+");
    val text = "hello 42 world 99";
    assert::eq(re.is_match(text), true);
}

#[test]
fn test_regex_no_match() {
    val re = std::regex::Regex::new("\\d+");
    assert::eq(re.is_match("no digits here"), false);
}

#[test]
fn test_regex_find() {
    val re = std::regex::Regex::new("[a-z]+");
    val text = "hi 42 world";
    val m = re.find(text);
    assert::eq(m.is_some(), true);
}

#[test]
fn test_regex_find_none() {
    val re = std::regex::Regex::new("\\d+");
    assert::eq(re.find("no digits").is_none(), true);
}

#[test]
fn test_regex_find_all_count() {
    val re = std::regex::Regex::new("\\d+");
    val v: List<String> = re.find_all("1 22 333");
    assert::eq(v.len(), 3);
    assert::eq(v[0], "1");
    assert::eq(v[1], "22");
    assert::eq(v[2], "333");
}

#[test]
fn test_regex_replace() {
    val re = std::regex::Regex::new("\\d+");
    val r: String = re.replace("a1 b22 c333", "N");
    assert::eq(r, "aN bN cN");
}

#[test]
fn test_regex_short_path() {
    // Unqualified `Regex::new` (without std::regex::) should also resolve.
    val re = Regex::new("foo");
    assert::eq(re.is_match("foobar"), true);
}

#[test]
fn test_regex_reuse() {
    // A single Regex value reused across many calls works.
    val re = std::regex::Regex::new("\\d");
    var count = 0;
    for s in ["abc", "1", "a2", "33"] {
        if re.is_match(s) { count = count + 1; }
    }
    assert::eq(count, 3);
}
