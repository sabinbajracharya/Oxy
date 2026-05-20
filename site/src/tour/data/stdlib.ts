import type { Chapter } from '../types';

export const stdlib: Chapter = {
  id: 'stdlib',
  title: 'Standard Library',
  lessons: [
    {
      id: 'json',
      title: 'JSON',
      instructions: `## JSON Parsing & Serialization

Oxy includes a built-in \`json::\` module. Parse JSON strings into values and serialize values to JSON.

\`json::parse(text)\` returns \`Result<Value, String>\`. \`json::to_string(value)\` serializes.

Access JSON fields with \`value["key"]\` and \`value[index]\`.

**Try it:** Parse a larger JSON object with nested objects and arrays.`,
      hints: [
        '`json::parse()` returns `Result<Value, String>`.',
        '`Value` from JSON is dynamic — index with strings and ints.',
      ],
      initialCode: `fn main() {
    let data = json::parse(
        "{\\"name\\": \\"Oxy\\", \\"year\\": 2024, \\"stable\\": true}"
    );
    match data {
        Ok(v) => {
            println!("name: {}", v["name"]);
            println!("year: {}", v["year"]);
            println!("{}", f"{} is stable: {}");
        },
        Err(e) => println!("parse error: {}", e),
    }

    let out = json::to_string([1, 2, 3]);
    println!("serialized: {}", out);
}
`,
    },
    {
      id: 'http',
      title: 'HTTP Client',
      instructions: `## HTTP Client

Use \`http::\` to make HTTP requests. \`http::get(url)\` fetches a URL and returns the response body as a String.

\`http::post(url, body)\` sends a POST request with a string body.

**Try it:** Change the URL to a different API endpoint (if you have internet access).`,
      hints: [
        '`http::get()` returns `Result<String, String>`.',
        'The HTTP module supports GET, POST, PUT, DELETE.',
      ],
      initialCode: `fn main() {
    let response = http::get("https://httpbin.org/json");
    match response {
        Ok(body) => {
            println!("got response: {} bytes", body.len());
            let json = json::parse(body);
            match json {
                Ok(v) => println!("parsed JSON"),
                Err(_) => println!("not JSON"),
            }
        },
        Err(e) => println!("HTTP error: {}", e),
    }
}
`,
    },
    {
      id: 'fs',
      title: 'Filesystem',
      instructions: `## Filesystem Operations

\`std::fs::\` provides file I/O:
- \`read_to_string(path)\` — read entire file
- \`write(path, content)\` — write to file
- \`exists(path)\` — check if file exists

All return \`Result\` to handle I/O errors.

**Try it:** Write a JSON config file and read it back.`,
      hints: [
        'File paths can be absolute or relative to the current working directory.',
        'Always check for errors — files might not exist.',
      ],
      initialCode: `fn main() {
    let path = "/tmp/oxy-tour-demo.txt";

    std::fs::write(path, "Hello from Oxy!\\nSecond line\\n");
    println!("file written");

    if std::fs::exists(path) {
        match std::fs::read_to_string(path) {
            Ok(content) => println!("read:\\n{}", content),
            Err(e) => println!("read error: {}", e),
        }
    }
}
`,
    },
    {
      id: 'math',
      title: 'Math',
      instructions: `## Math Module

\`math::\` provides mathematical functions and constants:
- \`math::abs()\`, \`math::sqrt()\`, \`math::pow()\`
- \`math::sin()\`, \`math::cos()\`, \`math::tan()\`
- \`math::floor()\`, \`math::ceil()\`, \`math::round()\`
- \`math::min()\`, \`math::max()\`, \`math::clamp()\`
- \`math::gcd()\`, \`math::lcm()\`
- \`math::PI\`, \`math::E\`

**Try it:** Compute the hypotenuse of a right triangle using \`sqrt\` and \`pow\`.`,
      hints: [
        '`math::PI` is a constant — use it directly.',
        'Numeric built-in methods also cover many of these: `x.abs()`, `x.sqrt()`.',
      ],
      initialCode: `fn main() {
    println!("PI = {}", math::PI);
    println!("sqrt(16) = {}", math::sqrt(16.0));
    println!("sin(PI/2) = {}", math::sin(math::PI / 2.0));

    println!("gcd(48, 18) = {}", math::gcd(48, 18));
    println!("lcm(12, 18) = {}", math::lcm(12, 18));

    println!("ceil(3.1) = {}", math::ceil(3.1));
    println!("floor(3.9) = {}", math::floor(3.9));
    println!("clamp(15, 0, 10) = {}", math::clamp(15, 0, 10));
}
`,
    },
    {
      id: 'other-stdlib',
      title: 'Regex, Time, Random',
      instructions: `## More Standard Library

- \`std::regex::\` — regular expressions: \`Regex::new(pattern)\`, \`is_match()\`, \`find()\`, \`captures()\`
- \`std::time::\` — timing: \`now()\`, \`elapsed()\`, \`sleep()\`
- \`std::rand::\` — random numbers: \`rand_int(min, max)\`, \`rand_float()\`
- \`std::env::\` — environment variables and paths

**Try it:** Generate 5 random dice rolls (1-6).`,
      hints: [
        '`std::rand::rand_int(min, max)` is inclusive on both ends.',
        '`std::time::now()` returns a timestamp.',
      ],
      initialCode: `fn main() {
    // Random numbers
    for i in 0..5 {
        let roll = std::rand::rand_int(1, 6);
        println!("dice roll: {}", roll);
    }

    // Regex
    let re = std::regex::Regex::new("\\\\d+");
    let text = "hello 42 world 99";
    println!("has digits: {}", re.is_match(text));
}
`,
    },
  ],
};
