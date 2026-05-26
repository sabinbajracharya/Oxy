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

An \`async fn\` looks like a regular \`fn\` but marked with \`async\`. The body executes asynchronously when the future is awaited.

**Your task:** Write an async function \`compute_answer\` that returns the int \`42\`. Then write a \`main\` that calls it (the result will be a Future).`,
      hints: [
        'Add \`async\` before \`fn\`: \`async fn name() -> T { ... }\`.',
        'The return type is the value type, not \`Future<T>\` — Oxy wraps it automatically.',
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
      testCode: `#[test] fn test_async_fn_returns_future() {
    async fn simple() -> int { 42 }
    let f = simple();
    // f is a Future<int> — compiles and type-checks
    assert!(true);
}

#[test] fn test_async_fn_with_string() {
    async fn greet(name: String) -> String {
        "Hello, ".to_string() + name
    }
    let f = greet("Oxy".to_string());
    assert!(true);
}

#[test] fn test_async_fn_void_return() {
    async fn do_something() {
        // no return type = returns Future<()>
    }
    let f = do_something();
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

**Your task:** Write \`async fn compute_answer()\` that returns 42, then in \`main\` call it and \`.await\` the result. Print the answer.`,
      hints: [
        'Call the async fn: \`let future = compute_answer();\`.',
        'Await it: \`let result = future.await;\`.',
        'You can also await inline: \`let result = compute_answer().await;\`.',
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
      testCode: `#[test] fn test_await_basic() {
    async fn get_value() -> int { 99 }
    let f = get_value();
    let v = f.await;
    assert_eq!(v, 99);
}

#[test] fn test_await_string() {
    async fn make_message() -> String {
        "hello async".to_string()
    }
    let msg = make_message().await;
    assert_eq!(msg, "hello async");
}

#[test] fn test_await_chained() {
    async fn step1() -> int { 10 }
    async fn step2(x: int) -> int { x + 5 }
    let a = step1().await;
    let b = step2(a).await;
    assert_eq!(b, 15);
}

#[test] fn test_await_expression() {
    async fn forty_two() -> int { 42 }
    let doubled = forty_two().await * 2;
    assert_eq!(doubled, 84);
}
`,
    },
    {
      id: 'async-closures',
      title: 'Async Closures',
      instructions: `## Async Closures

Oxy supports two forms of async closures:

1. \`async || { ... }\` — an async closure that captures variables
2. \`async { ... }\` — an async block that evaluates to a Future

\`\`\`
let f = async || {
    let data = fetch_data().await;
    data.len()
};

let block = async {
    42
};
\`\`\`

Both return a \`Future\` that can be \`.await\`ed.

**Your task:** Write an async closure that captures a \`name\` variable and returns a greeting. Also write an async block that computes a value. Call both and await the results.`,
      hints: [
        'Async closure syntax: \`async || { body }\`.',
        'Async block syntax: \`async { expression }\`.',
        'Call and await: \`let result = closure().await;\`.',
        'Inside the body, you can use other \`.await\` calls.',
      ],
      initialCode: `fn main() {
    let name = "Oxy".to_string();

    // TODO: create an async closure that captures "name" and returns a greeting
    let greet = ___;

    // TODO: create an async block that returns 100
    let compute = ___;

    // Await them:
    // let greeting = greet().await;
    // let value = compute.await;
    // println!("{} {}", greeting, value);
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

#[test] fn test_async_closure_multi_statement() {
    let f = async || -> int {
        let x = 10;
        let y = 20;
        x + y
    };
    let result = f().await;
    assert_eq!(result, 30);
}
`,
    },
    {
      id: 'async-http',
      title: 'Async HTTP',
      instructions: `## Async HTTP

Oxy provides two built-in async HTTP functions:

- \`http::fetch(url)\` — sends a GET request, returns \`Future<HttpResponse>\`
- \`http::fetch_post(url, body)\` — sends a POST request, returns \`Future<HttpResponse>\`

\`\`\`
struct HttpResponse {
    status: int,
    body: String,
    headers: HashMap<String, String>,
}
\`\`\`

Use \`.await\` to get the response after the request completes.

**Your task:** Write an async function that calls \`http::fetch\` and returns the response status code. Then call and await it.`,
      hints: [
        'Call \`http::fetch(url)\` passing a String URL.',
        'This returns \`Future<HttpResponse>\`, use \`.await\` to get the response.',
        'Access the status code: \`response.status\`.',
        'For POST, use \`http::fetch_post(url, body)\`.',
      ],
      initialCode: `struct HttpResponse {
    status: int,
    body: String,
    headers: HashMap<String, String>,
}

// TODO: write an async function "fetch_status" that takes a URL, fetches it, and returns the status code
async fn fetch_status(url: String) -> int {
    let response = ___.await;
    ___
}

fn main() {
    let url = "https://example.com".to_string();
    let status_future = fetch_status(url);
    // In production you'd .await this, but for now just verify it compiles
    println!("fetch_status returns a Future<int>");
}
`,
      testCode: `#[test] fn test_fetch_type_flows() {
    let f = http::fetch("https://example.com".to_string());
    let r = f.await;
    // r is HttpResponse — type checker verifies this
    let _status = r.status;
    let _body = r.body;
    assert!(true);
}

#[test] fn test_fetch_post_type() {
    let f = http::fetch_post("https://example.com".to_string(), "{\\"key\\": \\"value\\"}".to_string());
    let r = f.await;
    let _status = r.status;
    assert!(true);
}

#[test] fn test_fetch_status_function() {
    async fn fetch_status(url: String) -> int {
        let response = http::fetch(url).await;
        response.status
    }
    // Just verify the function compiles
    let f = fetch_status("https://example.com".to_string());
    let status = f.await;
    // status is int — type system verified
    assert_eq!(status, 200);
}
`,
    },
  ],
};
