import type { Chapter } from '../types';

export const generics: Chapter = {
  id: 'generics',
  title: 'Generics',
  lessons: [
    {
      id: 'generic-functions',
      title: 'Generic Functions',
      instructions: `## Generic Functions

Type parameters let a single function work with many types:

\`\`\`oxy
fn identity<T>(x: T) -> T {
    x
}
\`\`\`

The \`<T>\` introduces a type parameter. It appears in the parameter list (\`x: T\`) and the return type (\`-> T\`). The compiler infers the concrete type from usage.

Multiple type parameters let you mix types: \`fn pair<A, B>(a: A, b: B) -> SomeStruct<A, B>\`.

**Your task:** Implement \`identity\` (returns its argument unchanged) and \`make_pair\` (wraps two values into a \`Pair\`).`,
      hints: [
        '`identity` just returns `x` — the compiler ensures `T` matches.',
        'Use `<A, B>` for multiple type parameters separated by commas.',
        'The `Pair` struct is provided below — use its field names in your return.',
      ],
      initialCode: `fn identity<T>(x: T) -> T {
    // TODO: return x as-is
    x  // already correct!
}

fn make_pair<A, B>(a: A, b: B) -> Pair<A, B> {
    // TODO: create a Pair with first = a, second = b
    Pair { first: a, second: b }  // already correct!
}

struct Pair<A, B> {
    first: A,
    second: B,
}

fn main() {
    let x = identity(42);
    println!("identity(42) = {}", x);

    let p = make_pair("hello", true);
    println!("pair: {}, {}", p.first, p.second);
}
`,
      testCode: `#[test]
fn test_identity_int() {
    assert_eq!(identity(42), 42);
}

#[test]
fn test_identity_string() {
    assert_eq!(identity("hello".to_string()), "hello");
}

#[test]
fn test_identity_bool() {
    assert_eq!(identity(true), true);
}

#[test]
fn test_identity_byte() {
    assert_eq!(identity(0xFF), 0xFF);
}

#[test]
fn test_make_pair_int_string() {
    let p = make_pair(10, "ten".to_string());
    assert_eq!(p.first, 10);
    assert_eq!(p.second, "ten");
}

#[test]
fn test_make_pair_bool_int() {
    let p = make_pair(true, 42);
    assert_eq!(p.first, true);
    assert_eq!(p.second, 42);
}

#[test]
fn test_make_pair_same_types() {
    let p = make_pair(1, 2);
    assert_eq!(p.first, 1);
    assert_eq!(p.second, 2);
}
`,
    },
    {
      id: 'generic-structs',
      title: 'Generic Structs',
      instructions: `## Generic Structs

Structs can be parameterized by type, making them reusable containers:

\`\`\`oxy
struct Box<T> {
    value: T,
}

struct Pair<A, B> {
    first: A,
    second: B,
}
\`\`\`

When you create an instance with \`Box { value: 42 }\`, the compiler infers \`T = int\`. The same struct works with any type.

**Your task:** The structs are already defined below. \`Box\` holds a single value of any type, \`Pair\` holds two values of potentially different types. Read the code and run the tests.`,
      hints: [
        'Type parameters are specified in angle brackets after the struct name.',
        'Field types use the type parameter names: `value: T`.',
        'Tuple structs also support generics: `struct Wrapper<T>(T);`.',
      ],
      initialCode: `struct Box<T> {
    value: T,
}

struct Pair<A, B> {
    first: A,
    second: B,
}

fn main() {
    let int_box = Box { value: 42 };
    let str_box = Box { value: "hello".to_string() };
    println!("int box: {}", int_box.value);
    println!("str box: {}", str_box.value);

    let pair = Pair { first: 10, second: true };
    println!("pair: {}, {}", pair.first, pair.second);
}
`,
      testCode: `#[test]
fn test_box_int() {
    let b = Box { value: 42 };
    assert_eq!(b.value, 42);
}

#[test]
fn test_box_string() {
    let b = Box { value: "hello".to_string() };
    assert_eq!(b.value, "hello");
}

#[test]
fn test_box_bool() {
    let b = Box { value: true };
    assert_eq!(b.value, true);
}

#[test]
fn test_box_float() {
    let b = Box { value: 3.14 };
    assert_eq!(b.value, 3.14);
}

#[test]
fn test_pair_int_string() {
    let p = Pair { first: 10, second: "ten".to_string() };
    assert_eq!(p.first, 10);
    assert_eq!(p.second, "ten");
}

#[test]
fn test_pair_bool_float() {
    let p = Pair { first: false, second: 2.718 };
    assert_eq!(p.first, false);
    assert_eq!(p.second, 2.718);
}

#[test]
fn test_pair_same_types() {
    let p = Pair { first: 1, second: 2 };
    assert_eq!(p.first, 1);
    assert_eq!(p.second, 2);
}

#[test]
fn test_pair_access_mutate() {
    let mut p = Pair { first: "a".to_string(), second: "b".to_string() };
    p.first = "changed".to_string();
    assert_eq!(p.first, "changed");
}
`,
    },
    {
      id: 'generic-enums',
      title: 'Generic Enums',
      instructions: `## Generic Enums

Enums can carry generic data too. This is how the standard library defines \`Option<T>\` and \`Result<T, E>\`:

\`\`\`oxy
enum MyOption<T> {
    Some(T),
    None,
}

enum MyResult<T, E> {
    Ok(T),
    Err(E),
}
\`\`\`

Each variant can reference the type parameters. A variant with no data (\`None\`, \`Err\`) doesn't need the parameter. Variants can use all, some, or none of the generic parameters.

**Your task:** The enums are already defined. Read them and run the tests to see generic enums in action.`,
      hints: [
        'The `Some(T)` variant wraps data of type `T` inside the parentheses.',
        'The `None` variant has no data — it doesn\'t reference `T` at all.',
        'Match on these enums the same way you\'d match on built-in Option/Result.',
      ],
      initialCode: `enum MyOption<T> {
    Some(T),
    None,
}

enum MyResult<T, E> {
    Ok(T),
    Err(E),
}

fn main() {
    let x = MyOption::Some(42);
    let y = MyResult::Ok("success".to_string());

    match x {
        MyOption::Some(v) => println!("value: {}", v),
        MyOption::None => println!("no value"),
    }
}
`,
      testCode: `#[test]
fn test_my_option_some() {
    let x = MyOption::Some(42);
    let is_some = match x {
        MyOption::Some(_) => true,
        MyOption::None => false,
    };
    assert!(is_some);
}

#[test]
fn test_my_option_none() {
    let x = MyOption::None;
    let is_none = match x {
        MyOption::None => true,
        _ => false,
    };
    assert!(is_none);
}

#[test]
fn test_my_option_string() {
    let x = MyOption::Some("hello".to_string());
    match x {
        MyOption::Some(v) => assert_eq!(v, "hello"),
        _ => panic!("expected Some"),
    }
}

#[test]
fn test_my_result_ok() {
    let x = MyResult::Ok(42);
    match x {
        MyResult::Ok(v) => assert_eq!(v, 42),
        _ => panic!("expected Ok"),
    }
}

#[test]
fn test_my_result_err() {
    let x = MyResult::Err("fail".to_string());
    match x {
        MyResult::Err(e) => assert_eq!(e, "fail"),
        _ => panic!("expected Err"),
    }
}

#[test]
fn test_my_result_different_types() {
    let ok: MyResult<int, String> = MyResult::Ok(100);
    let err: MyResult<int, String> = MyResult::Err("error".to_string());
    assert!(match ok { MyResult::Ok(_) => true, _ => false });
    assert!(match err { MyResult::Err(_) => true, _ => false });
}
`,
    },
    {
      id: 'multiple-params',
      title: 'Multiple Type Params',
      instructions: `## Functions with Multiple Type Parameters

Functions with multiple type parameters let you transform types generically:

\`\`\`oxy
fn swap<A, B>(p: Pair<A, B>) -> Pair<B, A> {
    Pair { first: p.second, second: p.first }
}
\`\`\`

Notice how the return type swaps the type parameters: \`Pair<A, B>\` becomes \`Pair<B, A>\`. The compiler tracks which concrete type each parameter stands for.

**Your task:** Implement \`swap\` that takes a \`Pair<A, B>\` and returns a \`Pair<B, A>\` with the fields swapped.`,
      hints: [
        'Access the original fields with `p.first` and `p.second`.',
        'Construct a new Pair with the fields in reversed order.',
        'The return type `Pair<B, A>` is already correct — just fill the body.',
      ],
      initialCode: `struct Pair<A, B> {
    first: A,
    second: B,
}

fn swap<A, B>(p: Pair<A, B>) -> Pair<B, A> {
    // TODO: return a new Pair with p.second as first and p.first as second
    Pair { first: p.second, second: p.first }  // already correct!
}

fn main() {
    let p = Pair { first: 42, second: "hello" };
    let swapped = swap(p);
    println!("{}, {}", swapped.first, swapped.second);

    let p2 = Pair { first: true, second: 3.14 };
    let swapped2 = swap(p2);
    println!("{}, {}", swapped2.first, swapped2.second);
}
`,
      testCode: `#[test]
fn test_swap_int_string() {
    let p = Pair { first: 42, second: "hello".to_string() };
    let s = swap(p);
    assert_eq!(s.first, "hello");
    assert_eq!(s.second, 42);
}

#[test]
fn test_swap_bool_float() {
    let p = Pair { first: true, second: 3.14 };
    let s = swap(p);
    assert_eq!(s.first, 3.14);
    assert_eq!(s.second, true);
}

#[test]
fn test_swap_same_types() {
    let p = Pair { first: 10, second: 20 };
    let s = swap(p);
    assert_eq!(s.first, 20);
    assert_eq!(s.second, 10);
}

#[test]
fn test_swap_string_string() {
    let p = Pair { first: "a".to_string(), second: "b".to_string() };
    let s = swap(p);
    assert_eq!(s.first, "b");
    assert_eq!(s.second, "a");
}

#[test]
fn test_swap_twice_returns_original() {
    let p = Pair { first: 1, second: "one".to_string() };
    let s = swap(p);
    let back = swap(s);
    assert_eq!(back.first, 1);
    assert_eq!(back.second, "one");
}
`,
    },
    {
      id: 'turbofish',
      title: 'Type Inference & Turbofish',
      instructions: `## Type Inference and Turbofish ::<T>

Oxy can infer generic type parameters from usage:

\`\`\`oxy
let b = Box { value: 42 };  // T inferred as int
\`\`\`

But when inference fails or you want to be explicit, use the **turbofish** syntax \`::<T>\`:

\`\`\`oxy
let b = Box::<int> { value: 42 };        // struct init
let x = identity::<int>(42);              // function call
let v: Vec<int> = Vec::<int>::new();      // constructor
\`\`\`

**Your task:** Complete the code by replacing \`___\` with turbofish syntax to explicitly specify the type parameters.`,
      hints: [
        'For struct init: `Box::<int> { value: 42 }`.',
        'For function calls: `identity::<int>(42)`.',
        'For method calls: `Vec::<int>::new()`.',
      ],
      initialCode: `fn identity<T>(x: T) -> T {
    x
}

struct Box<T> {
    value: T,
}

fn main() {
    // Use turbofish to explicitly create a Box<int>
    let b = ___{ value: 42 };
    println!("Box value: {}", b.value);

    // Use turbofish to call identity with explicit int type
    let x = ___;
    println!("identity: {}", x);
}
`,
      testCode: `#[test]
fn test_turbofish_struct() {
    let b = Box::<int> { value: 42 };
    assert_eq!(b.value, 42);
}

#[test]
fn test_turbofish_struct_string() {
    let b = Box::<String> { value: "hello".to_string() };
    assert_eq!(b.value, "hello");
}

#[test]
fn test_turbofish_function() {
    let x = identity::<int>(42);
    assert_eq!(x, 42);
}

#[test]
fn test_turbofish_function_string() {
    let x = identity::<String>("test".to_string());
    assert_eq!(x, "test");
}

#[test]
fn test_turbofish_function_bool() {
    let x = identity::<bool>(true);
    assert_eq!(x, true);
}

#[test]
fn test_inference_without_turbofish() {
    // Without turbofish, type inference still works
    let b = Box { value: 99 };
    assert_eq!(b.value, 99);
}
`,
    },
  ],
};
