import type { Chapter } from '../types';

export const collections: Chapter = {
  id: 'collections',
  title: 'Collections',
  lessons: [
    {
      id: 'vec-basics',
      title: 'Vec Basics',
      instructions: `## Vec Basics

\`Vec<T>\` is a dynamic (growable) array. It stores elements of the same type in contiguous memory.

**Creating a Vec:**
- Use the \`vec!\` macro: \`vec![1, 2, 3]\` creates a \`Vec<int>\`
- Start an empty one: \`let v: Vec<int> = vec![]\`

**Key methods:**
- \`v.push(x)\` — appends \`x\` to the end
- \`v.pop()\` — removes and returns the last element (\`Option<T>\`)
- \`v.len()\` — returns the number of elements

**Your task:**

Complete these two functions:

1. \`make_vec() -> Vec<int>\` — return \`vec![1, 2, 3]\`
2. \`push_and_len(mut v: Vec<int>, x: int) -> int\` — push \`x\` onto \`v\` and return the new length

The \`mut\` keyword on the parameter lets you modify the Vec inside the function.`,
      hints: [
        'Use `vec![1, 2, 3]` to create a Vec literal.',
        '`v.push(x)` appends `x` to the end of the Vec.',
        '`v.len()` returns the current number of elements.',
      ],
      initialCode: `fn make_vec() -> Vec<int> {
    // TODO: return vec![1, 2, 3]
    vec![]
}

fn push_and_len(mut v: Vec<int>, x: int) -> int {
    // TODO: push x onto v and return the new length
    0
}

fn main() {
    let v = make_vec();
    println!("v = {}", v);

    let len = push_and_len(v, 99);
    println!("len after push = {}", len);
}
`,
      testCode: `#[test] fn test_make_vec_length() {
    let v = make_vec();
    assert!(v.len() == 3);
}

#[test] fn test_make_vec_values() {
    let v = make_vec();
    assert!(v[0] == 1);
    assert!(v[1] == 2);
    assert!(v[2] == 3);
}

#[test] fn test_push_and_len_nonempty() {
    assert!(push_and_len(vec![10, 20], 30) == 3);
}

#[test] fn test_push_and_len_empty() {
    assert!(push_and_len(vec![], 5) == 1);
}
`,
    },
    {
      id: 'vec-methods',
      title: 'Vec Methods',
      instructions: `## Safe Vec Methods

Vec provides access methods that return \`Option<T>\` instead of panicking on invalid access:

- \`v.get(i)\` — returns \`Some(element)\` or \`None\` if the index is out of bounds
- \`v.first()\` — returns \`Some(first)\` or \`None\` if empty
- \`v.last()\` — returns \`Some(last)\` or \`None\` if empty
- \`v.is_empty()\` — returns \`true\` if there are no elements

Unlike direct indexing (\`v[i]\`), these methods never panic. Invalid access produces \`None\` instead.

**Your task:**

Implement \`fn get_element(v: Vec<int>, i: int) -> Option<int>\` that:
- Returns the element at index \`i\` wrapped in \`Some()\`
- Returns \`None\` if the index is out of bounds

Use \`v.get(i)\` to implement this.`,
      hints: [
        '`v.get(i)` returns `Option<int>` — no need to wrap the result.',
        '`v.get(0)` returns the first element as `Some(value)`.',
        '`v.get(v.len())` and beyond return `None`.',
      ],
      initialCode: `fn get_element(v: Vec<int>, i: int) -> Option<int> {
    // TODO: use v.get(i) to safely access the element
    None()
}

fn main() {
    let v = vec![10, 20, 30];

    let first = get_element(v, 0);
    println!("first = {}", first.unwrap());

    let v2 = vec![10, 20, 30];
    let missing = get_element(v2, 100);
    println!("missing.is_none() = {}", missing.is_none());
}
`,
      testCode: `#[test] fn test_get_in_bounds() {
    let v = vec![5, 10, 15];
    let r = get_element(v, 1);
    assert!(r.is_some());
    assert!(r.unwrap() == 10);
}

#[test] fn test_get_out_of_bounds() {
    let v = vec![5, 10, 15];
    let r = get_element(v, 10);
    assert!(r.is_none());
}

#[test] fn test_get_first() {
    let v = vec![100, 200, 300];
    let r = get_element(v, 0);
    assert!(r.is_some());
    assert!(r.unwrap() == 100);
}

#[test] fn test_get_negative_index() {
    let v = vec![1, 2, 3];
    assert!(get_element(v, -1).is_none());
}
`,
    },
    {
      id: 'vec-indexing',
      title: 'Indexing Vec',
      instructions: `## Indexing Vec with Bracket Syntax

You can access elements directly with bracket syntax: \`v[index]\`.

**Important:** If the index is out of bounds, the program **panics** (crashes). Always check the index first for safe access.

To check bounds, compare against \`v.len()\`: valid indices are \`0 <= index < v.len()\`.

**Your task:**

Implement \`fn safe_get(v: Vec<int>, i: int) -> String\`:
- If \`i\` is a valid index (\`0 <= i < v.len()\`), return the element as a string using \`format!\`
- Otherwise, return \`"out of bounds"\`

Use an \`if\` expression to check the bounds before indexing.`,
      hints: [
        'Check `i >= 0 && i < v.len()` before indexing.',
        'Use `format!("{}", v[i])` to convert the integer to a `String`.',
        'Return `"out of bounds".to_string()` for invalid indices.',
      ],
      initialCode: `fn safe_get(v: Vec<int>, i: int) -> String {
    // TODO: check bounds, return element string or "out of bounds"
    "".to_string()
}

fn main() {
    let v = vec![5, 10, 15];
    println!("v[1] = {}", safe_get(v, 1));

    let v2 = vec![5, 10, 15];
    println!("v[10] = {}", safe_get(v2, 10));
}
`,
      testCode: `#[test] fn test_in_bounds() {
    let r = safe_get(vec![1, 2, 3], 1);
    assert!(r == "2".to_string());
}

#[test] fn test_out_of_bounds() {
    let r = safe_get(vec![1, 2, 3], 100);
    assert!(r == "out of bounds".to_string());
}

#[test] fn test_negative_index() {
    let r = safe_get(vec![1, 2, 3], -1);
    assert!(r == "out of bounds".to_string());
}

#[test] fn test_first_and_last() {
    assert!(safe_get(vec![42, 99, 7], 0) == "42".to_string());
    assert!(safe_get(vec![42, 99, 7], 2) == "7".to_string());
}
`,
    },
    {
      id: 'hashmap',
      title: 'HashMap',
      instructions: `## HashMap — Key-Value Store

\`HashMap<K, V>\` maps keys to values. Each key can only appear once — inserting a duplicate key overwrites the old value.

**Common methods:**
- \`map.insert(key, value)\` — insert or update a key
- \`map.get(key)\` — returns \`Option<V>\`
- \`map.contains_key(key)\` — returns \`bool\`
- \`map.len()\` — number of entries

**Your task:**

Implement \`fn build_map() -> HashMap<String, int>\` that creates a HashMap with three entries:
- \`"a"\` → \`10\`
- \`"b"\` → \`20\`
- \`"c"\` → \`30\`

Use \`"key".to_string()\` to convert string literals to \`String\` keys.`,
      hints: [
        'Create the map with `HashMap::new()`.',
        'Insert entries with `map.insert("a".to_string(), 10)`.',
        'Keys must be `String` — use `.to_string()` on string literals.',
      ],
      initialCode: `fn build_map() -> HashMap<String, int> {
    // TODO: create and return a HashMap with entries a=10, b=20, c=30
    HashMap::new()
}

fn main() {
    let map = build_map();
    println!("map has {} entries", map.len());
    println!("a = {}", map.get("a".to_string()).unwrap());
}
`,
      testCode: `#[test] fn test_build_map_length() {
    let map = build_map();
    assert!(map.len() == 3);
}

#[test] fn test_build_map_values() {
    let map = build_map();
    assert!(map.get("a".to_string()).unwrap() == 10);
    assert!(map.get("b".to_string()).unwrap() == 20);
    assert!(map.get("c".to_string()).unwrap() == 30);
}

#[test] fn test_contains_key() {
    let map = build_map();
    assert!(map.contains_key("a".to_string()));
    assert!(map.contains_key("b".to_string()));
    assert!(map.contains_key("c".to_string()));
    assert!(!map.contains_key("z".to_string()));
}
`,
    },
    {
      id: 'hashset',
      title: 'HashSet',
      instructions: `## HashSet — Unique Values

\`HashSet<T>\` stores unique values. Inserting a duplicate is silently ignored — the set keeps only one copy.

**Common methods:**
- \`set.insert(value)\` — add a value
- \`set.contains(value)\` — check for membership
- \`set.len()\` — number of unique values
- \`set.to_vec()\` — convert back to a \`Vec<T>\`

**Your task:**

Implement \`fn unique_items(items: Vec<int>) -> Vec<int>\` that removes duplicate values:

1. Create an empty \`HashSet<int>\`
2. Loop over \`items\` and insert each element into the set (duplicates are ignored automatically)
3. Convert the set to a Vec using \`set.to_vec()\`
4. Return the Vec`,
      hints: [
        'Create the set with `HashSet::new()`.',
        'Use a `for` loop with `item in items` to iterate.',
        'Call `set.to_vec()` at the end to get a `Vec<int>`.',
      ],
      initialCode: `fn unique_items(items: Vec<int>) -> Vec<int> {
    // TODO: use a HashSet to remove duplicates, then return unique items
    items
}

fn main() {
    let items = vec![1, 2, 2, 3, 3, 3];
    let unique = unique_items(items);
    println!("unique: {}", unique);
}
`,
      testCode: `#[test] fn test_removes_duplicates() {
    let result = unique_items(vec![1, 2, 2, 3, 3, 3]);
    assert!(result.len() == 3);
    assert!(result.contains(1));
    assert!(result.contains(2));
    assert!(result.contains(3));
}

#[test] fn test_all_unique() {
    let result = unique_items(vec![5, 1, 9]);
    assert!(result.len() == 3);
}

#[test] fn test_all_same() {
    let result = unique_items(vec![7, 7, 7, 7]);
    assert!(result.len() == 1);
    assert!(result.contains(7));
}

#[test] fn test_empty() {
    let result = unique_items(vec![]);
    assert!(result.len() == 0);
}
`,
    },
    {
      id: 'arrays',
      title: 'Arrays & Tuples',
      instructions: `## Arrays and Tuples

**Arrays** have a fixed length known at compile time. Type annotation uses \`[T; N]\`:

\`\`\`
let arr: [int; 3] = [10, 20, 30];
let zeros = [0; 5];     // five zeros
\`\`\`

Arrays are value types — they are copied on assignment.

**Tuples** group values of possibly different types:

\`\`\`
let t: (int, String, bool) = (42, "hello".to_string(), true);
\`\`\`

Access tuple fields with dot syntax: \`t.0\`, \`t.1\`, \`t.2\`.

**Your task:**

Implement \`fn first_three(items: Vec<int>) -> (int, int, int)\` that:
1. Assumes \`items\` has at least 3 elements
2. Returns the first three as a tuple: \`(items[0], items[1], items[2])\``,
      hints: [
        'Access individual elements with `items[0]`, `items[1]`, `items[2]`.',
        'Return a tuple with parentheses: `(a, b, c)`.',
        'Tuple types are written as `(int, int, int)`.',
      ],
      initialCode: `fn first_three(items: Vec<int>) -> (int, int, int) {
    // TODO: return the first three elements as a tuple
    (0, 0, 0)
}

fn main() {
    let items = vec![10, 20, 30, 40, 50];
    let (a, b, c) = first_three(items);
    println!("{}, {}, {}", a, b, c);
}
`,
      testCode: `#[test] fn test_first_three_many() {
    let result = first_three(vec![1, 2, 3, 4, 5]);
    assert!(result.0 == 1);
    assert!(result.1 == 2);
    assert!(result.2 == 3);
}

#[test] fn test_first_three_exact() {
    let result = first_three(vec![100, 200, 300]);
    assert!(result.0 == 100);
    assert!(result.1 == 200);
    assert!(result.2 == 300);
}

#[test] fn test_first_three_negative() {
    let result = first_three(vec![-5, 0, 5]);
    assert!(result.0 == -5);
    assert!(result.1 == 0);
    assert!(result.2 == 5);
}
`,
    },
    {
      id: 'iterating',
      title: 'Iterating Collections',
      instructions: `## Iterating Collections with For

Use \`for\` to loop over elements in a collection:

\`\`\`
for item in vec {
    // use item
}

for key in map.keys() {
    // use key
}
\`\`\`

The \`.keys()\` method on a HashMap returns an iterator over all keys.

**Your task:**

Implement \`fn sum_values(map: HashMap<String, int>) -> int\` that:
1. Iterates over all keys in the map using \`map.keys()\`
2. For each key, looks up the value and adds it to a running total
3. Returns the total sum

Start with \`let mut total = 0;\` and update it inside the loop.`,
      hints: [
        'Use `for key in map.keys() { ... }` to iterate over keys.',
        'Access the value with `map[key]` inside the loop.',
        'Accumulate: `total = total + map[key]`.',
      ],
      initialCode: `fn sum_values(map: HashMap<String, int>) -> int {
    // TODO: iterate over keys and sum all values
    0
}

fn main() {
    let mut map = HashMap::new();
    map.insert("a".to_string(), 10);
    map.insert("b".to_string(), 20);
    let total = sum_values(map);
    println!("total = {}", total);
}
`,
      testCode: `#[test] fn test_sum_multiple() {
    let mut map = HashMap::new();
    map.insert("x".to_string(), 5);
    map.insert("y".to_string(), 15);
    map.insert("z".to_string(), 25);
    assert!(sum_values(map) == 45);
}

#[test] fn test_sum_single() {
    let mut map = HashMap::new();
    map.insert("only".to_string(), 42);
    assert!(sum_values(map) == 42);
}

#[test] fn test_sum_zero_values() {
    let mut map = HashMap::new();
    map.insert("a".to_string(), 0);
    map.insert("b".to_string(), 0);
    assert!(sum_values(map) == 0);
}

#[test] fn test_sum_empty_map() {
    let map = HashMap::new();
    assert!(sum_values(map) == 0);
}
`,
    },
  ],
};
