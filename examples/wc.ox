// A small CLI tool — counts lines/words/chars of files or stdin.
// Usage:
//   oxy run examples/wc.ox file1 file2 ...
//   cat input.txt | oxy run examples/wc.ox --
//   oxy run examples/wc.ox --lines file1
//   oxy run examples/wc.ox --words --chars file1

fn count(text: String, want_lines: bool, want_words: bool, want_chars: bool) -> String {
    val lines = text.lines().collect().len();
    val words = text.split_whitespace().collect().len();
    val chars = text.len();
    var parts = [];
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
    val args = std::args::parse();
    val want_lines = args.flags.contains_key("lines") || args.flags.contains_key("l");
    val want_words = args.flags.contains_key("words") || args.flags.contains_key("w");
    val want_chars = args.flags.contains_key("chars") || args.flags.contains_key("c");
    // Default: show all three (like real wc).
    val none_selected = !want_lines && !want_words && !want_chars;
    val lines = want_lines || none_selected;
    val words = want_words || none_selected;
    val chars = want_chars || none_selected;

    if args.positionals.len() == 0 {
        val text = std::io::read_to_string().unwrap();
        println("{}", count(text, lines, words, chars));
        return;
    }

    var total_lines = 0;
    var total_words = 0;
    var total_chars = 0;
    for path in args.positionals {
        val result = std::fs::read_to_string(path);
        if val Ok(text) = result {
            total_lines = total_lines + text.lines().collect().len();
            total_words = total_words + text.split_whitespace().collect().len();
            total_chars = total_chars + text.len();
            println("{}\t{}", count(text, lines, words, chars), path);
        } else {
            println("error reading {}", path);
        }
    }
    if args.positionals.len() > 1 {
        var parts = [];
        if lines { parts.push(total_lines.to_string()); }
        if words { parts.push(total_words.to_string()); }
        if chars { parts.push(total_chars.to_string()); }
        println("{}\ttotal", parts.join("\t"));
    }
}
