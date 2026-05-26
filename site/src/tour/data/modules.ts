import type { Chapter } from '../types';

export const modules: Chapter = {
  id: 'modules',
  title: 'Modules',
  lessons: [
    {
      id: 'mod-basics',
      title: 'Module Basics',
      instructions: `## Defining Modules

Modules organize code into namespaces. Define one with \`mod name { ... }\`:

\`\`\`oxy
mod math {
    pub fn add(a: int, b: int) -> int {
        a + b
    }
}
\`\`\`

Items inside a module are accessed with \`::\` path syntax: \`math::add(3, 4)\`. By default, everything inside a module is **private** — use \`pub\` to expose it.

**Your task:** Add a \`subtract\` function to the \`math\` module and call it from \`main\`.`,
      hints: [
        'Add your function inside the `mod math { ... }` block.',
        'Mark it `pub fn` so it\'s accessible from outside the module.',
        'Call it as `math::subtract(10, 3)` from main.',
      ],
      initialCode: `mod math {
    pub fn add(a: int, b: int) -> int {
        a + b
    }

    // TODO: add a pub fn subtract(a: int, b: int) -> int
}

fn main() {
    let sum = math::add(10, 5);
    println!("10 + 5 = {}", sum);

    // TODO: call math::subtract(10, 3)
    println!("10 - 3 = {}", math::subtract(10, 3));
}
`,
      testCode: `#[test]
fn test_math_add() {
    assert_eq!(math::add(3, 4), 7);
}

#[test]
fn test_math_add_negative() {
    assert_eq!(math::add(-5, 10), 5);
}

#[test]
fn test_math_add_zero() {
    assert_eq!(math::add(0, 0), 0);
}

#[test]
fn test_math_subtract() {
    assert_eq!(math::subtract(10, 3), 7);
}

#[test]
fn test_math_subtract_negative_result() {
    assert_eq!(math::subtract(3, 10), -7);
}

#[test]
fn test_math_subtract_zero() {
    assert_eq!(math::subtract(5, 0), 5);
}
`,
    },
    {
      id: 'use-statements',
      title: 'Use Statements',
      instructions: `## Bringing Items into Scope with \`use\`

The \`use\` keyword imports items so you can refer to them without the full path:

\`\`\`oxy
use math::add;          // single import
use math::{add, sub};   // group import
use math::*;            // glob import
use math::add as plus;  // alias
\`\`\`

After \`use math::add;\`, you can call \`add(3, 4)\` directly.

**Your task:** Add the missing \`use\` statements so the calls in \`main\` work without qualified paths.`,
      hints: [
        'Use `use math::{add, sub}` to import both at once.',
        'After `use math::mul;`, you can call `mul(4, 5)` directly.',
        'Group imports: `use math::{add, sub, mul};`.',
      ],
      initialCode: `mod math {
    pub fn add(a: int, b: int) -> int { a + b }
    pub fn sub(a: int, b: int) -> int { a - b }
    pub fn mul(a: int, b: int) -> int { a * b }
}

// TODO: add use statements for math::{add, sub, mul}
use math::add;
use math::sub;
use math::mul;

fn main() {
    println!("2 + 3 = {}", add(2, 3));
    println!("10 - 4 = {}", sub(10, 4));
    println!("4 * 5 = {}", mul(4, 5));
}
`,
      testCode: `#[test]
fn test_use_add() {
    use math::add;
    assert_eq!(add(3, 4), 7);
}

#[test]
fn test_use_sub() {
    use math::sub;
    assert_eq!(sub(10, 3), 7);
}

#[test]
fn test_use_mul() {
    use math::mul;
    assert_eq!(mul(4, 5), 20);
}

#[test]
fn test_use_group_import() {
    use math::{add, sub, mul};
    assert_eq!(add(1, 2), 3);
    assert_eq!(sub(5, 3), 2);
    assert_eq!(mul(3, 4), 12);
}

#[test]
fn test_qualified_path_still_works() {
    assert_eq!(math::add(10, 20), 30);
    assert_eq!(math::mul(6, 7), 42);
}
`,
    },
    {
      id: 'visibility',
      title: 'Visibility',
      instructions: `## Public and Private Items

Items inside a module are **private by default**. Use visibility modifiers:

- \`pub\` — visible everywhere
- \`pub(crate)\` — visible within the crate (not across crates)
- \`pub(super)\` — visible only to the parent module

Private items can only be accessed within their own module. This is the foundation of encapsulation.

**Your task:** Mark the \`greet\` function as \`pub\` so it's accessible from \`main\`. The private \`helper\` function should only be callable from within the module.`,
      hints: [
        'Add `pub` before `fn` to make a function public.',
        'Private functions are accessible only inside the same module.',
        'The `reveal` function acts as a public wrapper for `helper`.',
      ],
      initialCode: `mod greeter {
    // TODO: make this function pub so main can call it
    fn greet(name: String) -> String {
        "Hello, " + name
    }

    fn helper() -> String {
        "internal helper".to_string()
    }

    pub fn reveal() -> String {
        helper()  // accessible here: same module
    }
}

fn main() {
    println!("{}", greeter::greet("Alice".to_string()));
    println!("revealed: {}", greeter::reveal());
}
`,
      testCode: `#[test]
fn test_public_greet() {
    let r = greeter::greet("World".to_string());
    assert_eq!(r, "Hello, World");
}

#[test]
fn test_public_greet_empty() {
    let r = greeter::greet("".to_string());
    assert_eq!(r, "Hello, ");
}

#[test]
fn test_reveal_calls_private_helper() {
    let r = greeter::reveal();
    assert_eq!(r, "internal helper");
}

#[test]
fn test_reveal_multiple_calls() {
    assert_eq!(greeter::reveal(), "internal helper");
    assert_eq!(greeter::reveal(), "internal helper");
}
`,
    },
    {
      id: 'self-super',
      title: 'self / super / crate',
      instructions: `## Path Prefixes

Three special path prefixes help navigate the module hierarchy:

- \`self::\` — current module (usually optional)
- \`super::\` — parent module
- \`crate::\` — root of the crate (top-level)

The \`super::\` prefix is essential when a child module needs to access its parent:

\`\`\`oxy
mod parent {
    pub fn root_val() -> int { 42 }
    pub mod child {
        pub fn call_parent() -> int {
            super::root_val()  // accesses parent's function
        }
    }
}
\`\`\`

**Your task:** Complete \`call_super\` in the \`child\` module so it uses \`super::\` to call \`parent::parent_value()\`.`,
      hints: [
        'Use `super::parent_value()` to call the parent module\'s function.',
        '`super::` refers to the parent of the current module.',
        'The function is `pub` so it can be called from outside — but `super::` is about path resolution, not visibility.',
      ],
      initialCode: `mod parent {
    pub fn parent_value() -> String {
        "from parent".to_string()
    }

    pub mod child {
        pub fn call_super() -> String {
            // TODO: use super:: to call parent_value()
            ___
        }
    }
}

fn main() {
    println!("{}", parent::child::call_super());
}
`,
      testCode: `#[test]
fn test_child_calls_parent() {
    let r = parent::child::call_super();
    assert_eq!(r, "from parent");
}

#[test]
fn test_parent_direct_call() {
    let r = parent::parent_value();
    assert_eq!(r, "from parent");
}

#[test]
fn test_child_call_twice() {
    assert_eq!(parent::child::call_super(), "from parent");
    assert_eq!(parent::child::call_super(), "from parent");
}
`,
    },
    {
      id: 'reexport',
      title: 'Re-exporting',
      instructions: `## Re-exporting with \`pub use\`

When you want to expose an item from a different module, re-export it with \`pub use\`:

\`\`\`oxy
mod inner {
    pub fn compute(x: int) -> int { x * x }
}

pub use inner::compute;  // compute is now available at the current level
\`\`\`

This lets you flatten deep hierarchies, hide internal module structure, or create a clean public API. Re-exports can use group imports (\`pub use inner::{A, B}\`) and aliases (\`pub use inner::Thing as Renamed\`).

**Your task:** Add the \`pub use\` statement to re-export \`inner::compute\` so \`main\` can call it directly. Also add a re-export inside \`outer\` to expose \`super::inner::helper\`.`,
      hints: [
        '`pub use inner::compute;` makes `compute` available in the current scope.',
        'For the nested re-export, use `pub use super::inner::helper;` inside `mod outer`.',
        'Re-exports redirect without duplicating — the original item stays in `inner`.',
      ],
      initialCode: `mod inner {
    pub fn compute(x: int) -> int {
        x * x + 2 * x + 1
    }
    pub fn helper() -> String {
        "helper result".to_string()
    }
}

// TODO: pub use inner::compute
pub use inner::compute;

mod outer {
    // TODO: pub use super::inner::helper
}

fn main() {
    println!("compute(5) = {}", compute(5));
    println!("helper: {}", outer::helper());
}
`,
      testCode: `#[test]
fn test_reexported_compute() {
    assert_eq!(compute(5), 36);
}

#[test]
fn test_reexported_compute_zero() {
    assert_eq!(compute(0), 1);
}

#[test]
fn test_reexported_compute_negative() {
    assert_eq!(compute(-3), 4);
}

#[test]
fn test_reexported_helper() {
    assert_eq!(outer::helper(), "helper result");
}

#[test]
fn test_original_path_still_works() {
    assert_eq!(inner::compute(5), 36);
    assert_eq!(inner::helper(), "helper result");
}
`,
    },
  ],
};
