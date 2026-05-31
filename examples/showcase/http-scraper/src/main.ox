// http-scraper — fetch a URL and extract links or regex-matched text.
//
//   tug run -- --url=https://example.com --links
//   tug run -- --url=https://example.com --pattern="<title>([^<]+)</title>"
//   tug run -- --url=https://example.com --links --json

use cli_utils;

fn extract_hrefs(html: String) -> List<String> {
    let mut urls = [];
    let rx_result = Regex::new(r#"href="([^"]*)""#);
    match rx_result {
        Ok(rx) => {
            let matches = rx.find_all(html);
            for m in matches {
                let s = m.to_string();
                let url = s.replace("href=\"", "").replace("\"", "");
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
    let mut results = [];
    let rx_result = Regex::new(pattern);
    match rx_result {
        Ok(rx) => {
            let matches = rx.find_all(html);
            for m in matches {
                results.push(m.to_string());
            }
        }
        Err(e) => cli_utils::die("invalid regex: " + e),
    }
    results
}

fn main() {
    let args = std::args::parse();

    let url_opt = args.flags.get("url");
    if url_opt.is_none() {
        cli_utils::die("--url=<url> is required");
    }
    let target = url_opt.unwrap().to_string();

    let want_links = args.flags.contains_key("links");
    let has_pattern = args.flags.contains_key("pattern");
    let as_json = args.flags.contains_key("json");

    if !want_links && !has_pattern {
        cli_utils::die("need --links or --pattern=<regex>");
    }

    cli_utils::info("fetching " + target + "...");

    let resp = http::get(target);
    match resp {
        Ok(response) => {
            if !response.status_ok() {
                cli_utils::die("HTTP " + response.status.to_string());
            }

            let body = response.body;

            if want_links {
                let links = extract_hrefs(body);

                if as_json {
                    println("{}", links.to_json());
                } else {
                    cli_utils::header("Links (" + links.len().to_string() + ")");
                    for link in links {
                        println("  " + link);
                    }
                }
                return;
            }

            if has_pattern {
                let pat = args.flags.get("pattern").unwrap().to_string();
                let items = extract_matches(body, pat);

                if as_json {
                    println("{}", items.to_json());
                } else {
                    cli_utils::header("Matches (" + items.len().to_string() + ")");
                    for item in items {
                        println("  " + item);
                    }
                }
                return;
            }
        }
        Err(e) => cli_utils::die("request failed: " + e),
    }
}
