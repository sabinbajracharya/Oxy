// http-scraper — fetch a URL and extract links or regex-matched text.
//
//   tug run -- --url=https://example.com --links
//   tug run -- --url=https://example.com --pattern="<title>([^<]+)</title>"
//   tug run -- --url=https://example.com --links --json

use cli_utils;

fn extract_hrefs(html: String) -> List<String> {
    var urls = [];
    val rx_result = Regex::new(r#"href="([^"]*)""#);
    match rx_result {
        Ok(rx) => {
            val matches = rx.find_all(html);
            for m in matches {
                val s = m.to_string();
                val url = s.replace("href=\"", "").replace("\"", "");
                if url.len() > 0 && !url.starts_with("#") && !url.starts_with("mailto:") && !url.starts_with("javascript:") {
                    urls.push(url);
                }
            }
        }
        Err(_) => {}
    }
    urls
}

fn extract_matches(html: String, pattern: String) -> List<String> {
    var results = [];
    val rx_result = Regex::new(pattern);
    match rx_result {
        Ok(rx) => {
            val matches = rx.find_all(html);
            for m in matches {
                results.push(m.to_string());
            }
        }
        Err(e) => cli_utils::die("invalid regex: " + e),
    }
    results
}

fn main() {
    val args = std::args::parse();

    val url_opt = args.flags.get("url");
    if url_opt.is_none() {
        cli_utils::die("--url=<url> is required");
    }
    val target = url_opt.unwrap().to_string();

    val want_links = args.flags.contains_key("links");
    val has_pattern = args.flags.contains_key("pattern");
    val as_json = args.flags.contains_key("json");

    if !want_links && !has_pattern {
        cli_utils::die("need --links or --pattern=<regex>");
    }

    cli_utils::info("fetching " + target + "...");

    val resp = http::get(target);
    match resp {
        Ok(response) => {
            if !response.status_ok() {
                cli_utils::die("HTTP " + response.status.to_string());
            }

            val body = response.body;

            if want_links {
                val links = extract_hrefs(body);

                if as_json {
                    io::println("{}", links.to_json());
                } else {
                    cli_utils::header("Links (" + links.len().to_string() + ")");
                    for link in links {
                        io::println("  " + link);
                    }
                }
                return;
            }

            if has_pattern {
                val pat = args.flags.get("pattern").unwrap().to_string();
                val items = extract_matches(body, pat);

                if as_json {
                    io::println("{}", items.to_json());
                } else {
                    cli_utils::header("Matches (" + items.len().to_string() + ")");
                    for item in items {
                        io::println("  " + item);
                    }
                }
                return;
            }
        }
        Err(e) => cli_utils::die("request failed: " + e),
    }
}
