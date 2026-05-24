// A small CLI tool — counts lines/words/chars of files or stdin.
// Usage:
//   oxy run examples/wc.ox file1 file2 ...
//   cat input.txt | oxy run examples/wc.ox --
//   oxy run examples/wc.ox --lines file1
//   oxy run examples/wc.ox --words --chars file1

fn count(text: String, want_lines: bool, want_words: bool, want_chars: bool) -> String {
    let lines = text.lines().collect().len();
    let words = text.split_whitespace().collect().len();
    let chars = text.len();
    let mut parts = vec![];
    if want_lines {
        parts.push(lines.to_string());
    }
    if want_words {
        parts.push(words.to_string());
    }
    if want_chars {
        parts.push(chars.to_string());
    }
    parts.join("\t")
}

fn main() {
    let args = std::args::parse();
    let want_lines = args.flags.contains_key("lines") || args.flags.contains_key("l");
    let want_words = args.flags.contains_key("words") || args.flags.contains_key("w");
    let want_chars = args.flags.contains_key("chars") || args.flags.contains_key("c");
    // Default: show all three (like real wc).
    let none_selected = !want_lines && !want_words && !want_chars;
    let lines = want_lines || none_selected;
    let words = want_words || none_selected;
    let chars = want_chars || none_selected;

    if args.positionals.len() == 0 {
        let text = std::io::read_to_string().unwrap();
        println!("{}", count(text, lines, words, chars));
        return;
    }

    let mut total_lines = 0;
    let mut total_words = 0;
    let mut total_chars = 0;
    for path in args.positionals {
        let result = std::fs::read_to_string(path);
        if let Ok(text) = result {
            total_lines = total_lines + text.lines().collect().len();
            total_words = total_words + text.split_whitespace().collect().len();
            total_chars = total_chars + text.len();
            println!("{}\t{}", count(text, lines, words, chars), path);
        } else {
            println!("error reading {}", path);
        }
    }
    if args.positionals.len() > 1 {
        let mut parts = vec![];
        if lines { parts.push(total_lines.to_string()); }
        if words { parts.push(total_words.to_string()); }
        if chars { parts.push(total_chars.to_string()); }
        println!("{}\ttotal", parts.join("\t"));
    }
}
