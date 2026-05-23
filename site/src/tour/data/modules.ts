import type { Chapter } from '../types';

export const modules: Chapter = {
  id: 'modules',
  title: 'Modules',
  lessons: [
    {
      id: 'mod-use',
      title: 'Mod & Use',
      instructions: `## Modules and Use

\`mod\` declares a module. Modules can be inline (\`mod name { ... }\`) or file-based (\`mod name;\`).

\`use\` imports items into scope: \`use module::path::Item;\`.

**Try it:** Add another function to the \`math\` module and use it in \`main\`.`,
      hints: [
        '`mod foo { ... }` defines an inline module.',
        '`use math::add;` brings `add` into scope without qualification.',
      ],
      initialCode: `mod math {
    pub fn add(a: int, b: int) -> int {
        a + b
    }

    pub fn square(x: int) -> int {
        x * x
    }
}

use math::add;

fn main() {
    let sum = add(3, 4);
    println!("3 + 4 = {}", sum);
    println!("5^2 = {}", math::square(5));
}
`,
    },
    {
      id: 'visibility',
      title: 'Visibility: pub, pub(crate), pub(super)',
      instructions: `## Visibility

By default, items in modules are **private**. Use visibility modifiers to expose them:

- \`pub\` — visible everywhere
- \`pub(crate)\` — visible within the current crate
- \`pub(super)\` — visible to the parent module

**Try it:** Try accessing \`secret()\` from \`main()\` — it should fail. Then make it visible.`,
      hints: [
        'Private items are only accessible within their own module.',
        'Top-level items are always visible (not inside a module).',
      ],
      initialCode: `mod outer {
    pub fn visible() -> String { "public".to_string() }
    pub(crate) fn crate_wide() -> String { "crate".to_string() }
    fn secret() -> String { "secret".to_string() }

    pub fn reveal() -> String {
        secret() // accessible here: same module
    }
}

fn main() {
    println!("{}", outer::visible());
    println!("{}", outer::crate_wide());
    println!("revealed: {}", outer::reveal());
    // println!("{}", outer::secret()); // ERROR: private
}
`,
    },
    {
      id: 'use-aliases',
      title: 'Use Aliases & Groups',
      instructions: `## Use — Aliases, Groups, Glob

Use supports:
- **Aliases**: \`use path::Item as Alias;\`
- **Groups**: \`use path::{A, B, C};\`
- **Glob**: \`use path::*;\`

**Try it:** Add an alias for \`math::multiply\` and use the shorter name.`,
      hints: [
        '`as` lets you rename imports to avoid conflicts.',
        '`use module::{a, b};` imports multiple items in one line.',
      ],
      initialCode: `mod math {
    pub fn add(a: int, b: int) -> int { a + b }
    pub fn mul(a: int, b: int) -> int { a * b }
    pub fn sub(a: int, b: int) -> int { a - b }
}

use math::{add, mul};
use math::sub as subtract;

fn main() {
    println!("2 + 3 = {}", add(2, 3));
    println!("4 * 5 = {}", mul(4, 5));
    println!("10 - 3 = {}", subtract(10, 3));
}
`,
    },
    {
      id: 'reexport',
      title: 'Re-exporting with pub use',
      instructions: `## Re-exporting

Use \`pub use\` to re-export items from a module, making them accessible through the re-exporting module's public API.

This lets you flatten deep module hierarchies or expose internal items under a different path.

**Try it:** Re-export \`math::multiply\` as well.`,
      hints: [
        '`pub use inner::Thing;` makes `Thing` available at the outer module level.',
        'Re-exports can glob: `pub use inner::*;`.',
      ],
      initialCode: `mod inner {
    pub fn compute(x: int) -> int {
        x * x + 2 * x + 1
    }
    pub fn helper() -> String { "helper".to_string() }
}

pub use inner::compute;

mod outer {
    pub use super::inner::helper;
}

fn main() {
    println!("compute(5) = {}", compute(5)); // direct access via re-export
    println!("helper: {}", outer::helper());
}
`,
    },
  ],
};
