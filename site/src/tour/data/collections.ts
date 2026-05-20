import type { Chapter } from '../types';

export const collections: Chapter = {
  id: 'collections',
  title: 'Collections',
  lessons: [
    {
      id: 'vec',
      title: 'Vec',
      instructions: `## Vec — Dynamic Array

\`Vec<T>\` is a growable array. Create one with \`vec![1, 2, 3]\` or a comma-separated list in brackets.

Methods: \`push()\`, \`pop()\`, \`len()\`, \`get()\`, \`first()\`, \`last()\`, \`sort()\`, \`contains()\`, \`iter()\`, \`reverse()\`, and many more.

**Try it:** Add \`sort()\` before printing, or use \`reverse()\`.`,
      hints: [
        '`vec![1, 2, 3]` creates a `Vec<i64>`.',
        'Index access: `v[0]` returns the first element.',
      ],
      initialCode: `fn main() {
    let mut v = vec![10, 20, 30];
    v.push(40);
    v.push(50);
    println!("v = {}", v);
    println!("len = {}", v.len());
    println!("first = {}", v.first().unwrap());
    println!("v[2] = {}", v[2]);

    v.pop();
    println!("after pop: {}", v);
}
`,
    },
    {
      id: 'hashmap',
      title: 'HashMap',
      instructions: `## HashMap — Key-Value Store

\`HashMap<K, V>\` maps keys to values. Create with \`HashMap::new()\` and \`insert()\` entries.

Create from pairs: \`[("a", 1), ("b", 2)].iter().collect::<HashMap<_, _>>()\`.

Methods: \`insert()\`, \`get()\`, \`remove()\`, \`contains_key()\`, \`keys()\`, \`values()\`, \`len()\`.

**Try it:** Add another entry and use \`contains_key()\` to check for it.`,
      hints: [
        '`map.get(key)` returns an `Option<V>`.',
        '`map["key"]` syntax also works for index access.',
      ],
      initialCode: `fn main() {
    let mut scores = HashMap::new();
    scores.insert("alice".to_string(), 95);
    scores.insert("bob".to_string(), 82);

    println!("alice: {}", scores.get("alice".to_string()).unwrap());
    println!("len = {}", scores.len());

    for key in scores.keys() {
        let val = scores[key.clone()];
        println!("  {} -> {}", key, val);
    }
}
`,
    },
    {
      id: 'hashset',
      title: 'HashSet',
      instructions: `## HashSet — Unique Values

\`HashSet<T>\` stores unique values with fast lookup.

Methods: \`insert()\`, \`remove()\`, \`contains()\`, \`len()\`, \`union()\`, \`intersection()\`, \`difference()\`.

**Try it:** Create a third set and find the intersection with one of the existing sets.`,
      hints: [
        '`set.contains(value)` returns `true` if the value is present.',
        'Duplicates are silently ignored on insert.',
      ],
      initialCode: `fn main() {
    let mut set = HashSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(2); // duplicate — ignored
    set.insert(3);

    println!("set = {}", set.to_vec());
    println!("len = {}", set.len());
    println!("contains 2: {}", set.contains(2));
    println!("contains 5: {}", set.contains(5));
}
`,
    },
    {
      id: 'arrays',
      title: 'Fixed-Size Arrays',
      instructions: `## Fixed-Size Arrays [T; N]

Arrays have a fixed size known at compile time. Type annotation: \`[i64; 3]\`.

Create with a comma-separated list \`[1, 2, 3]\` or a repeat expression \`[0; 5]\` (five zeros).

Arrays are **value types** — they're copied on assignment, not shared.

**Try it:** Create a \`[bool; 4]\` with alternating true/false values.`,
      hints: [
        '`[0; 5]` creates an array of 5 zeros.',
        'Arrays work with `for` loops and iterators just like Vec.',
      ],
      initialCode: `fn main() {
    let arr: [i64; 3] = [10, 20, 30];
    println!("arr = {}", arr);
    println!("len = {}", arr.len());
    println!("arr[1] = {}", arr[1]);

    // Repeat expression
    let zeros = [0; 5];
    println!("zeros = {}", zeros);

    // Equality
    let a = [1, 2, 3];
    let b = [1, 2, 3];
    println!("a == b: {}", a == b);

    for v in arr {
        println!("  {}", v);
    }
}
`,
    },
    {
      id: 'iterators',
      title: 'Iterators & Chaining',
      instructions: `## Iterators

Call \`.iter()\` on a collection to get an iterator. Chain operations like \`map\`, \`filter\`, \`take\`, \`skip\`, and \`collect\`.

Use \`collect::<Vec<_>>()\` to gather results into a Vec.

**Try it:** Add \`skip(1)\` and \`take(2)\` to get only the middle elements.`,
      hints: [
        '`filter(|x| condition)` keeps only elements matching the predicate.',
        '`collect::<Vec<_>>()` uses turbofish syntax to specify the collection type.',
      ],
      initialCode: `fn main() {
    let nums = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    let evens = nums.iter()
        .filter(|x| x % 2 == 0)
        .collect::<Vec<_>>();
    println!("evens: {}", evens);

    let squares = nums.iter()
        .take(5)
        .map(|x| x * x)
        .collect::<Vec<_>>();
    println!("first 5 squared: {}", squares);
}
`,
    },
  ],
};
