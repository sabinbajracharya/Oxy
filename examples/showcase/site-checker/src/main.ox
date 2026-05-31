// site-checker — find broken links in a directory of static HTML files.
//
//   tug run -- --dir=./public
//   tug run -- --dir=./public --check-external
//   tug run -- --dir=./public --verbose

use cli_utils;

fn find_html_files(dir: String) -> Vec<String> {
    let mut files = vec();
    let result = std::fs::read_dir(dir);
    match result {
        Ok(entries) => {
            for entry in entries {
                let sep = if dir.ends_with("/") { "" } else { "/" };
                let path = dir + sep + entry;
                if std::fs::is_dir(path) {
                    let nested = find_html_files(path);
                    for f in nested {
                        files.push(f);
                    }
                } else if entry.ends_with(".html") || entry.ends_with(".htm") {
                    files.push(path);
                }
            }
        }
        Err(e) => cli_utils::warn("cannot read dir " + dir + ": " + e),
    }
    files
}

fn extract_urls(html: String) -> Vec<String> {
    let mut urls = vec();

    let patterns = vec(
        r#"href="([^"]*)""#,
        r#"src="([^"]*)""#,
    );

    let mut i = 0;
    while i < patterns.len() {
        let rx_result = Regex::new(patterns.get(i).unwrap().to_string());
        match rx_result {
            Ok(rx) => {
                let matches = rx.find_all(html);
                for m in matches {
                    let s = m.to_string();
                    let url = s.replace("href=\"", "").replace("src=\"", "").replace("\"", "");
                    if url.len() > 0
                        && !url.starts_with("#")
                        && !url.starts_with("mailto:")
                        && !url.starts_with("javascript:")
                        && !url.starts_with("data:")
                    {
                        urls.push(url);
                    }
                }
            }
            Err(_) => {}
        }
        i = i + 1;
    }
    urls
}

fn check_http(url: String, verbose: bool) -> bool {
    let resp = http::get(url);
    match resp {
        Ok(response) => {
            let ok = response.status >= 200 && response.status < 400;
            if verbose {
                if ok {
                    cli_utils::success(url + "  HTTP " + response.status.to_string());
                } else {
                    cli_utils::fail(url + "  HTTP " + response.status.to_string());
                }
            }
            ok
        }
        Err(e) => {
            if verbose {
                cli_utils::fail(url + "  " + e);
            }
            false
        }
    }
}

fn main() {
    let args = std::args::parse();

    let dir_opt = args.flags.get("dir");
    if dir_opt.is_none() {
        cli_utils::die("--dir=<path> is required");
    }
    let base_dir = dir_opt.unwrap().to_string();
    let check_external = args.flags.contains_key("check-external");
    let verbose = args.flags.contains_key("verbose") || args.flags.contains_key("v");

    cli_utils::header("site-checker");
    cli_utils::info("scanning " + base_dir + "...");

    let files = find_html_files(base_dir);
    cli_utils::info("found " + files.len().to_string() + " HTML file(s)");

    let mut all_urls = vec();
    for file in files {
        let result = std::fs::read_to_string(file);
        match result {
            Ok(html) => {
                let urls = extract_urls(html);
                for url in urls {
                    all_urls.push(url);
                }
            }
            Err(e) => cli_utils::warn("cannot read " + file + ": " + e),
        }
    }

    cli_utils::info("found " + all_urls.len().to_string() + " link(s)");

    // separate local and external URLs
    let mut local_urls = vec();
    let mut external_urls = vec();

    for url in all_urls {
        if url.starts_with("http://") || url.starts_with("https://") {
            external_urls.push(url);
        } else {
            local_urls.push(url);
        }
    }

    let mut ok_count = 0;
    let mut broken = 0;

    // check local files
    if local_urls.len() > 0 {
        cli_utils::header("Local links (" + local_urls.len().to_string() + ")");
        for url in local_urls {
            let target = if url.starts_with("/") {
                base_dir + url
            } else {
                url
            };
            if std::fs::exists(target) {
                ok_count = ok_count + 1;
                if verbose {
                    cli_utils::success(url + "  ok");
                }
            } else {
                broken = broken + 1;
                cli_utils::fail(url + "  not found");
            }
        }
    }

    // check external URLs
    if check_external && external_urls.len() > 0 {
        cli_utils::header("External links (" + external_urls.len().to_string() + ")");
        for url in external_urls {
            if check_http(url, verbose) {
                ok_count = ok_count + 1;
            } else {
                broken = broken + 1;
            }
        }
    } else if external_urls.len() > 0 {
        cli_utils::info(external_urls.len().to_string() + " external link(s) skipped (use --check-external)");
    }

    // summary
    cli_utils::header("Results");
    if broken == 0 {
        cli_utils::success("all " + ok_count.to_string() + " checked link(s) passed");
    } else {
        cli_utils::fail(broken.to_string() + " broken, " + ok_count.to_string() + " ok");
        std::process::exit(1);
    }
}
