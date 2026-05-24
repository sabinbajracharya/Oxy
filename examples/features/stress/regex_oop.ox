// === STRESS: OOP-style Regex::new(pattern) ===

#[test]
fn test_regex_new_and_is_match() {
    let re = std::regex::Regex::new("\\d+");
    let text = "hello 42 world 99";
    assert_eq!(re.is_match(text), true);
}

#[test]
fn test_regex_no_match() {
    let re = std::regex::Regex::new("\\d+");
    assert_eq!(re.is_match("no digits here"), false);
}

#[test]
fn test_regex_find() {
    let re = std::regex::Regex::new("[a-z]+");
    let text = "hi 42 world";
    let m = re.find(text);
    assert_eq!(m.is_some(), true);
}

#[test]
fn test_regex_find_none() {
    let re = std::regex::Regex::new("\\d+");
    assert_eq!(re.find("no digits").is_none(), true);
}

#[test]
fn test_regex_find_all_count() {
    let re = std::regex::Regex::new("\\d+");
    let v: Vec<String> = re.find_all("1 22 333");
    assert_eq!(v.len(), 3);
    assert_eq!(v[0], "1");
    assert_eq!(v[1], "22");
    assert_eq!(v[2], "333");
}

#[test]
fn test_regex_replace() {
    let re = std::regex::Regex::new("\\d+");
    let r: String = re.replace("a1 b22 c333", "N");
    assert_eq!(r, "aN bN cN");
}

#[test]
fn test_regex_short_path() {
    // Unqualified `Regex::new` (without std::regex::) should also resolve.
    let re = Regex::new("foo");
    assert_eq!(re.is_match("foobar"), true);
}

#[test]
fn test_regex_reuse() {
    // A single Regex value reused across many calls works.
    let re = std::regex::Regex::new("\\d");
    let mut count = 0;
    for s in vec!["abc", "1", "a2", "33"] {
        if re.is_match(s) { count = count + 1; }
    }
    assert_eq!(count, 3);
}
