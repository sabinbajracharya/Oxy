// log-filter — filter and analyze log files by level or regex pattern.
//
//   tug run -- --file=app.log --level=ERROR
//   tug run -- --file=app.log --pattern="timeout|refused"
//   tug run -- --level=WARN --count < app.log
//   cat app.log | tug run -- --level=ERROR --with-filename

use cli_utils;

fn count_level(text: String, level: String) -> Int {
    var count = 0;
    for line in text.lines() {
        if line.contains(level) {
            count = count + 1;
        }
    }
    count
}

fn filter_level(text: String, level: String, show_filename: bool, filename: String) {
    var i = 1;
    for line in text.lines() {
        if line.contains(level) {
            if show_filename {
                io::println("{}:{}: {}", filename, i, line);
            } else {
                io::println("{}: {}", i, line);
            }
        }
        i = i + 1;
    }
}

fn filter_pattern(text: String, pattern: String, show_filename: bool, filename: String) {
    val rx_result = Regex::new(pattern);
    match rx_result {
        Ok(rx) => {
            var i = 1;
            for line in text.lines() {
                if rx.is_match(line) {
                    if show_filename {
                        io::println("{}:{}: {}", filename, i, line);
                    } else {
                        io::println("{}: {}", i, line);
                    }
                }
                i = i + 1;
            }
        }
        Err(e) => cli_utils::die("invalid regex: " + e),
    }
}

fn count_pattern(text: String, pattern: String) -> Int {
    val rx_result = Regex::new(pattern);
    match rx_result {
        Ok(rx) => {
            var count = 0;
            for line in text.lines() {
                if rx.is_match(line) {
                    count = count + 1;
                }
            }
            count
        }
        Err(e) => {
            cli_utils::die("invalid regex: " + e);
            0
        }
    }
}

fn read_input(file_path: String) -> String {
    val result = std::fs::read_to_string(file_path);
    match result {
        Ok(text) => text,
        Err(e) => cli_utils::die("cannot read file: " + e),
    }
}

fn main() {
    val args = std::args::parse();

    val has_level = args.flags.contains_key("level");
    val has_pattern = args.flags.contains_key("pattern");
    val show_count = args.flags.contains_key("count");
    val show_filename = args.flags.contains_key("with-filename");

    if !has_level && !has_pattern {
        cli_utils::die("need --level or --pattern");
    }

    val level = args.flags.get("level");
    val pattern = args.flags.get("pattern");
    val file_opt = args.flags.get("file");

    if val Some(path) = file_opt {
        val text = read_input(path.to_string());
        val fname = if show_filename { path.to_string() } else { "" };

        if show_count {
            val n = if has_pattern {
                count_pattern(text, pattern.unwrap().to_string())
            } else {
                count_level(text, level.unwrap().to_string())
            };
            io::println("{}", n);
            return;
        }

        if has_pattern {
            filter_pattern(text, pattern.unwrap().to_string(), show_filename, fname);
        } else {
            filter_level(text, level.unwrap().to_string(), show_filename, fname);
        }
        return;
    }

    // read from stdin
    val stdin_result = std::io::read_to_string();
    match stdin_result {
        Ok(text) => {
            if show_count {
                val n = if has_pattern {
                    count_pattern(text, pattern.unwrap().to_string())
                } else {
                    count_level(text, level.unwrap().to_string())
                };
                io::println("{}", n);
                return;
            }

            if has_pattern {
                filter_pattern(text, pattern.unwrap().to_string(), false, "");
            } else {
                filter_level(text, level.unwrap().to_string(), false, "");
            }
        }
        Err(e) => cli_utils::die("cannot read stdin: " + e),
    }
}
