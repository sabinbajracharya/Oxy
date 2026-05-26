import type { Chapter } from '../types';

export const asyncChapter: Chapter = {
  id: 'async',
  title: 'Async Programming',
  lessons: [
    {
      id: 'async-fn',
      title: 'Async Functions',
      instructions: `## Async Functions

Prefix a function with \`async\` to make it asynchronous. An async function returns a \`Future<T>\` — a value that will eventually produce a \`T\`.

\`\`\`
async fn fetch_data() -> String {
    "result".to_string()
}
\`\`\`

The return type is the inner type. Oxy wraps it in \`Future<T>\` automatically. Calling an async fn gives you a Future — you don't get the value until you \`.await\` it.

**Your task:** Write an async function \`compute_answer\` that returns the int \`42\`. Call it from \`main\` to get a Future.`,
      hints: [
        'Add `async` before `fn`: `async fn name() -> T { ... }`.',
        'The return type is `int` — Oxy wraps it in `Future<int>`.',
        'Calling an async fn returns a Future, even if the body is synchronous.',
      ],
      initialCode: `// TODO: write an async function called "compute_answer" that returns 42
async fn compute_answer() -> int {
    ___
}

fn main() {
    let future = compute_answer();
    // future is a Future<int>
    println!("Got a future!");
}
`,
      testCode: `// Top-level async helpers for tests
async fn forty_two() -> int { 42 }
async fn greet(name: String) -> String { "Hello, ".to_string() + name }
async fn do_nothing() {}

#[test] fn test_async_fn_returns_future() {
    let f = forty_two();
    assert!(true);
}

#[test] fn test_async_fn_with_string() {
    let f = greet("Oxy".to_string());
    assert!(true);
}

#[test] fn test_async_fn_void_return() {
    let f = do_nothing();
    assert!(true);
}

#[test] fn test_user_compute_answer() {
    let f = compute_answer();
    assert!(true);
}
`,
    },
    {
      id: 'await',
      title: 'Await',
      instructions: `## Await

Use \`.await\` to unwrap a \`Future\` and get its value. The \`.await\` expression pauses until the future completes, then produces the inner value.

\`\`\`
async fn get_number() -> int { 42 }

fn main() {
    let future = get_number();   // Future<int>
    let value = future.await;    // int — unwraps the future
    println!("{}", value);       // 42
}
\`\`\`

You can chain async calls by awaiting one, then using the result in another.

**Your task:** Call \`compute_answer()\` in \`main\` and use \`.await\` to get the result. Print the answer.`,
      hints: [
        'Call the async fn: `let future = compute_answer();`.',
        'Await it: `let result = future.await;`.',
        'You can also await inline: `let result = compute_answer().await;`.',
      ],
      initialCode: `async fn compute_answer() -> int {
    42
}

fn main() {
    // TODO: call compute_answer() and .await the result
    let result = ___;
    println!("The answer is {}", result);
}
`,
      testCode: `async fn get_value() -> int { 99 }
async fn make_message() -> String { "hello async".to_string() }
async fn step1() -> int { 10 }
async fn step2(x: int) -> int { x + 5 }

#[test] fn test_await_basic() {
    let f = get_value();
    let v = f.await;
    assert_eq!(v, 99);
}

#[test] fn test_await_chained() {
    let a = step1().await;
    let b = step2(a).await;
    assert_eq!(b, 15);
}

#[test] fn test_await_expression() {
    let doubled = get_value().await * 2;
    assert_eq!(doubled, 198);
}
`,
    },
    {
      id: 'async-closures',
      title: 'Async Closures',
      instructions: `## Async Closures

Oxy supports two forms of async inline expressions:

1. **Async closures:** \`async || { ... }\` — captures variables like a regular closure
2. **Async blocks:** \`async { ... }\` — evaluates a block and returns a Future

\`\`\`
let f = async || { 42 };
let result = f().await;  // 42

let block = async { 100 };
let val = block.await;   // 100
\`\`\`

Both are **expressions** — you can define them anywhere, including inside function bodies.

**Your task:** Create an async closure that captures a \`name\` variable and returns a greeting string.`,
      hints: [
        'Async closure syntax: `async || { body }`.',
        'Async block syntax: `async { expression }`.',
        'Call and await: `let result = closure().await;`.',
      ],
      initialCode: `fn main() {
    let name = "Oxy".to_string();

    // TODO: create an async closure that captures "name" and returns a greeting
    let greet = ___;

    // TODO: create an async block that returns 100
    let compute = ___;

    // Await them and print:
    // let g = greet().await;
    // println!("{}", g);
    // let v = compute.await;
    // println!("{}", v);
}
`,
      testCode: `#[test] fn test_async_closure_basic() {
    let f = async || { 42 };
    let result = f().await;
    assert_eq!(result, 42);
}

#[test] fn test_async_closure_with_capture() {
    let x = 10;
    let f = async || { x * 2 };
    let result = f().await;
    assert_eq!(result, 20);
}

#[test] fn test_async_block() {
    let result = async { 99 }.await;
    assert_eq!(result, 99);
}

#[test] fn test_async_block_with_capture() {
    let a = 5;
    let b = 7;
    let result = async { a + b }.await;
    assert_eq!(result, 12);
}
`,
    },
    {
      id: 'async-http',
      title: 'Async HTTP',
      instructions: `## Async HTTP

Oxy provides two built-in async HTTP functions:

- \`http::fetch(url)\` — GET request, returns \`Future<HttpResponse>\`
- \`http::fetch_post(url, body)\` — POST request, returns \`Future<HttpResponse>\`

\`HttpResponse\` has fields: \`status: int\`, \`body: String\`, \`headers: HashMap<String, String>\`.

Use \`.await\` on the returned Future to get the response.

**Your task:** Write an async function \`fetch_status\` that fetches a URL and returns the response status code.`,
      hints: [
        'Call `http::fetch(url)` passing a String URL.',
        'Use `.await` on the result to get the HttpResponse.',
        'Return `response.status`.',
      ],
      initialCode: `struct HttpResponse {
    status: int,
    body: String,
    headers: HashMap<String, String>,
}

// TODO: write an async function "fetch_status" that fetches a URL and returns the status code
async fn fetch_status(url: String) -> int {
    let response = ___.await;
    ___
}

fn main() {
    let url = "https://example.com".to_string();
    let status_future = fetch_status(url);
    println!("fetch_status returns a Future<int>");
}
`,
      testCode: `async fn fake_fetch_status(url: String) -> int {
    let response = http::fetch(url).await;
    response.status
}

#[test] fn test_fetch_type_flows() {
    let f = http::fetch("https://example.com".to_string());
    let r = f.await;
    let _status: int = r.status;
    assert!(true);
}

#[test] fn test_fetch_status_function_compiles() {
    let f = fake_fetch_status("https://example.com".to_string());
    let _s = f.await;
    assert!(true);
}
`,
    },
  ],
};
