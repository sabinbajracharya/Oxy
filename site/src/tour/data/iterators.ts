import type { Chapter } from '../types';

export const iterators: Chapter = {
  id: 'iterators',
  title: 'Iterators',
  lessons: [
    {
      id: 'iter-basics',
      title: 'Iterator Basics',
      instructions: `## Iterator Basics

Call \`.iter()\` on a Vec to create an iterator. Iterators produce elements one at a time.

Use \`for x in sequence { ... }\` to loop over elements directly. You can also call \`.next()\` to get the next element as \`Option<&T>\`.

**Consumers** like \`.sum()\`, \`.count()\`, and \`.fold()\` process the entire iterator and return a single value.

**Your task:** Complete \`sum_squares\` that iterates over a Vec and returns the sum of squares of each element. Use a for-in loop. Fill in \`for x in v { ... }\`.`,
      hints: [
        'Use \`for x in v { total = total + x * x; }\`.',
        'Initialize \`total\` to 0 before the loop.',
        'Square an element with \`x * x\`.',
      ],
      initialCode: `fn sum_squares(v: Vec<int>) -> int {
    let mut total = 0;
    for x in v {
        ___
    }
    total
}

fn main() {
    let v = vec![1, 2, 3, 4];
    println!("sum of squares = {}", sum_squares(v));
}
`,
      testCode: `#[test] fn test_sum_squares_basic() {
    assert_eq!(sum_squares(vec![1, 2, 3]), 14);  // 1+4+9
}

#[test] fn test_sum_squares_more() {
    assert_eq!(sum_squares(vec![2, 2]), 8);      // 4+4
    assert_eq!(sum_squares(vec![]), 0);
    assert_eq!(sum_squares(vec![0, 0, 0]), 0);
}

#[test] fn test_iter_next() {
    let v = vec![10, 20, 30];
    let mut it = v.iter();
    assert_eq!(it.next(), Some(&10));
    assert_eq!(it.next(), Some(&20));
    assert_eq!(it.next(), Some(&30));
    assert_eq!(it.next(), None);
}

#[test] fn test_for_loop_sum() {
    let v = vec![1, 2, 3, 4, 5];
    let mut total = 0;
    for x in v {
        total = total + x;
    }
    assert_eq!(total, 15);
}
`,
    },
    {
      id: 'map-filter',
      title: 'Map & Filter',
      instructions: `## Map & Filter

**Map** transforms each element: \`v.iter().map(|x| x * 2)\` returns a Vec with each element doubled.

**Filter** keeps elements matching a condition: \`v.iter().filter(|x| x % 2 == 0)\` returns a Vec of even numbers.

Both \`.map()\` and \`.filter()\` are **eager** — they return a \`Vec\` directly, not a lazy iterator.

To chain them, call \`.iter()\` on the result before the next adapter:
\`\`\`
v.iter().filter(|x| x > 0).iter().map(|x| x * 2)
\`\`\`

**Your task:** Implement \`sum_doubled_evens\` that takes a Vec, filters to keep only even numbers, doubles each, and returns their sum.`,
      hints: [
        'First filter even numbers: \`.filter(|x| x % 2 == 0)\`.',
        'Then map to double: \`.iter().map(|x| x * 2)\`.',
        'Use \`.iter()\` between filter and map since filter returns a Vec.',
        'For sum, use \`.iter().sum()\` on the mapped result, or a for-in loop.',
      ],
      initialCode: `fn sum_doubled_evens(v: Vec<int>) -> int {
    let mut total = 0;
    for x in v {
        ___
    }
    total
}

fn main() {
    let v = vec![1, 2, 3, 4, 5, 6];
    println!("sum doubled evens = {}", sum_doubled_evens(v));
}
`,
      testCode: `#[test] fn test_filter_even() {
    let v = vec![1, 2, 3, 4, 5, 6];
    let evens = v.iter().filter(|x| x % 2 == 0);
    assert_eq!(evens, vec![2, 4, 6]);
}

#[test] fn test_map_double() {
    let v = vec![1, 2, 3];
    let doubled = v.iter().map(|x| x * 2);
    assert_eq!(doubled, vec![2, 4, 6]);
}

#[test] fn test_filter_then_map() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().filter(|x| x % 2 == 0).iter().map(|x| x * x);
    assert_eq!(result, vec![4, 16]);
}

#[test] fn test_map_then_filter() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().map(|x| x * 2).iter().filter(|x| x > 4);
    assert_eq!(result, vec![6, 8, 10]);
}

#[test] fn test_filter_empty() {
    let v: Vec<int> = vec![];
    let result = v.iter().filter(|x| x > 0);
    assert_eq!(result.len(), 0);
}

#[test] fn test_map_strings() {
    let v = vec!["hello", "world"];
    let result = v.iter().map(|s| s.to_uppercase());
    assert_eq!(result[0], "HELLO");
    assert_eq!(result[1], "WORLD");
}
`,
    },
    {
      id: 'take-skip',
      title: 'Take & Skip',
      instructions: `## Take & Skip

\`.take(n)\` takes the first \`n\` elements from an iterator and stops.

\`.skip(n)\` skips the first \`n\` elements and yields the rest.

Both are **lazy** — they return an iterator that needs to be consumed with \`.collect()\` or a for loop.

\`\`\`
v.iter().take(3).collect()   // first 3 elements as Vec
v.iter().skip(2).collect()   // all elements after the first 2
v.iter().skip(1).take(3).collect()  // skip 1, take next 3
\`\`\`

**Your task:** Implement \`first_n\` that returns the first \`n\` elements as a Vec, and \`skip_first\` that skips the first \`n\` and returns the rest.`,
      hints: [
        'Use \`v.iter().take(n).collect()\` for first_n.',
        'Use \`v.iter().skip(n).collect()\` for skip_first.',
        'Both handle \`n\` larger than the Vec length gracefully (no crash).',
      ],
      initialCode: `fn first_n(v: Vec<int>, n: int) -> Vec<int> {
    ___
}

fn skip_first(v: Vec<int>, n: int) -> Vec<int> {
    ___
}

fn main() {
    let v = vec![1, 2, 3, 4, 5];
    println!("first 3: {:?}", first_n(v, 3));
    let v2 = vec![1, 2, 3, 4, 5];
    println!("skip 2: {:?}", skip_first(v2, 2));
}
`,
      testCode: `#[test] fn test_take_basic() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().take(3).collect();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test] fn test_take_more_than_len() {
    let v = vec![1, 2, 3];
    let result = v.iter().take(10).collect();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test] fn test_take_zero() {
    let v = vec![1, 2, 3];
    let result = v.iter().take(0).collect();
    assert_eq!(result.len(), 0);
}

#[test] fn test_skip_basic() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().skip(2).collect();
    assert_eq!(result, vec![3, 4, 5]);
}

#[test] fn test_skip_all() {
    let v = vec![1, 2, 3];
    let result = v.iter().skip(10).collect();
    assert_eq!(result.len(), 0);
}

#[test] fn test_skip_then_take() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().skip(1).take(3).collect();
    assert_eq!(result, vec![2, 3, 4]);
}

#[test] fn test_first_n_using_take() {
    assert_eq!(first_n(vec![1, 2, 3, 4, 5], 3), vec![1, 2, 3]);
    assert_eq!(first_n(vec![1, 2, 3], 0).len(), 0);
}

#[test] fn test_skip_first_using_skip() {
    assert_eq!(skip_first(vec![1, 2, 3, 4, 5], 2), vec![3, 4, 5]);
    assert_eq!(skip_first(vec![1, 2, 3], 5).len(), 0);
}
`,
    },
    {
      id: 'chain-zip',
      title: 'Chain & Zip',
      instructions: `## Chain & Zip

\`.chain(other)\` concatenates two iterators end-to-end:

\`\`\`
let a = vec![1, 2, 3];
let b = vec![4, 5, 6];
let all = a.iter().chain(b.iter()).collect();  // [1, 2, 3, 4, 5, 6]
\`\`\`

\`.zip(other)\` pairs elements from two iterators into tuples, stopping at the shorter one:

\`\`\`
a.iter().zip(b.iter()).collect()  // [(1, 10), (2, 20), (3, 30)]
\`\`\`

\`.enumerate()\` pairs each element with its 0-based index.

**Your task:** Implement \`interleave\` that merges two Vecs by alternating elements. Zip them, then collect pairs into a flat sequence.`,
      hints: [
        'Zip the two iterators to get pairs \`(a0, b0), (a1, b1)\`.',
        'Iterate over zipped pairs with a for loop or use flat_map.',
        'After zip runs out, append remaining elements from the longer Vec.',
      ],
      initialCode: `fn interleave(a: Vec<int>, b: Vec<int>) -> Vec<int> {
    let mut result = vec![];
    ___
    result
}

fn main() {
    let a = vec![1, 3, 5];
    let b = vec![2, 4, 6];
    println!("interleaved: {:?}", interleave(a, b));
}
`,
      testCode: `#[test] fn test_chain_two_vecs() {
    let a = vec![1, 2, 3];
    let b = vec![4, 5, 6];
    let result = a.iter().chain(b.iter()).collect();
    assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
}

#[test] fn test_chain_empty_first() {
    let a: Vec<int> = vec![];
    let b = vec![1, 2, 3];
    let result = a.iter().chain(b.iter()).collect();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test] fn test_chain_then_map() {
    let a = vec![1, 2];
    let b = vec![3, 4];
    let result = a.iter().chain(b.iter()).collect().iter().map(|x| x * 10);
    assert_eq!(result, vec![10, 20, 30, 40]);
}

#[test] fn test_zip_basic() {
    let a = vec![1, 2, 3];
    let b = vec![10, 20, 30];
    let pairs = a.iter().zip(b.iter()).collect();
    assert_eq!(pairs.len(), 3);
    let (a0, b0) = pairs[0];
    assert_eq!(a0, 1);
    assert_eq!(b0, 10);
}

#[test] fn test_zip_stops_at_shorter() {
    let a = vec![1, 2, 3, 4, 5];
    let b = vec![10, 20];
    let pairs = a.iter().zip(b.iter()).collect();
    assert_eq!(pairs.len(), 2);
}

#[test] fn test_enumerate_basic() {
    let v = vec![10, 20, 30];
    let pairs = v.iter().enumerate().collect();
    let (i0, v0) = pairs[0];
    assert_eq!(i0, 0);
    assert_eq!(v0, 10);
}

#[test] fn test_interleave_example() {
    let result = interleave(vec![1, 3, 5], vec![2, 4, 6]);
    assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
}
`,
    },
    {
      id: 'fold-reduce',
      title: 'Fold & Reduce',
      instructions: `## Fold & Reduce

\`.fold(initial, |acc, x| expression)\` reduces an iterator to a single value by repeatedly applying a closure. The accumulator \`acc\` starts as \`initial\` and gets updated with each element.

\`\`\`
v.iter().fold(0, |acc, x| acc + x)  // sum
v.iter().fold(1, |acc, x| acc * x)  // product
v.iter().fold("", |acc, x| if x.len() > acc.len() { x } else { acc })  // longest string
\`\`\`

\`.reduce(|a, b| expression)\` is similar but uses the first element as the initial value.

**Your task:** Implement \`product\` using fold to multiply all numbers. Also implement \`longest_string\` using fold to find the longest String.`,
      hints: [
        'For product, start fold with \`1\` and multiply: \`fold(1, |acc, x| acc * x)\`.',
        'For longest, start with \`""\` and compare lengths: \`fold("", |acc, x| if x.len() > acc.len() { x } else { acc })\`.',
      ],
      initialCode: `fn product(v: Vec<int>) -> int {
    v.iter().fold(1, |acc, x| acc * x)
}

fn longest_string(v: Vec<String>) -> String {
    ___
}

fn main() {
    let nums = vec![2, 3, 4];
    println!("product = {}", product(nums));
    let words = vec!["hi".to_string(), "hello".to_string(), "a".to_string()];
    println!("longest = {}", longest_string(words));
}
`,
      testCode: `#[test] fn test_fold_sum() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().fold(0, |acc, x| acc + x);
    assert_eq!(result, 15);
}

#[test] fn test_fold_product_basic() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().fold(1, |acc, x| acc * x);
    assert_eq!(result, 120);
}

#[test] fn test_fold_product_with_zero() {
    let v = vec![1, 2, 0, 4, 5];
    let result = v.iter().fold(1, |acc, x| acc * x);
    assert_eq!(result, 0);
}

#[test] fn test_fold_max() {
    let v = vec![3, 1, 4, 1, 5, 9, 2, 6];
    let result = v.iter().fold(0, |acc, x| if x > acc { x } else { acc });
    assert_eq!(result, 9);
}

#[test] fn test_fold_string_concat() {
    let v = vec!["a", "b", "c"];
    let result = v.iter().fold("", |acc, x| {
        if acc == "" { x } else { acc + x }
    });
    assert_eq!(result, "abc");
}

#[test] fn test_fold_empty() {
    let v: Vec<int> = vec![];
    let result = v.iter().fold(42, |acc, x| acc + x);
    assert_eq!(result, 42);
}

#[test] fn test_product_fn() {
    assert_eq!(product(vec![2, 3, 4]), 24);
    assert_eq!(product(vec![5]), 5);
    assert_eq!(product(vec![1, 2, 3, 0]), 0);
}

#[test] fn test_longest_string_fn() {
    let w = vec!["hi".to_string(), "hello".to_string(), "a".to_string()];
    assert_eq!(longest_string(w), "hello");
    let one = vec!["xyz".to_string()];
    assert_eq!(longest_string(one), "xyz");
}
`,
    },
    {
      id: 'collect',
      title: 'Collect & Patterns',
      instructions: `## Collect & Patterns

\`.collect()\` gathers iterator items into a Vec. It's used with **lazy** iterators from \`.take()\`, \`.skip()\`, \`.rev()\`, \`.chain()\`, \`.zip()\`, \`.enumerate()\`, and \`.flat_map()\`.

Common patterns:

\`\`\`
v.iter().take(n).collect()              // take first n
v.iter().filter(|x| cond).iter().map(|x| f(x)).iter().take(3).collect()   // filter → map → take → collect
v.iter().rev().collect()                // reverse
\`\`\`

**Your task:** Implement \`first_n_positives_doubled\` that filters positives, doubles them, and returns only the first \`n\` results. Use \`collect()\` to build the final Vec.`,
      hints: [
        'Filter positives first: \`v.iter().filter(|x| x > 0)\`.',
        'Then map to double: \`.iter().map(|x| x * 2)\`.',
        'Then take n: \`.iter().take(n)\`.',
        'Finally collect: \`.collect()\`.',
      ],
      initialCode: `fn first_n_positives_doubled(v: Vec<int>, n: int) -> Vec<int> {
    ___
}

fn main() {
    let v = vec![-3, 1, 4, -1, 5, 9, -2];
    let result = first_n_positives_doubled(v, 3);
    println!("first 3 positives doubled: {:?}", result);
}
`,
      testCode: `#[test] fn test_collect_roundtrip() {
    let v = vec![1, 2, 3];
    let v2 = v.iter().collect();
    assert_eq!(v2, vec![1, 2, 3]);
}

#[test] fn test_collect_after_take() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().take(2).collect();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], 1);
    assert_eq!(result[1], 2);
}

#[test] fn test_collect_after_skip() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().skip(3).collect();
    assert_eq!(result, vec![4, 5]);
}

#[test] fn test_rev_collect() {
    let v = vec![1, 2, 3, 4, 5];
    let result = v.iter().rev();
    assert_eq!(result, vec![5, 4, 3, 2, 1]);
}

#[test] fn test_filter_map_chain() {
    let v = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let result = v.iter().filter(|x| x % 2 == 0).iter().map(|x| x * 2).iter().take(2).collect();
    assert_eq!(result, vec![4, 8]);
}

#[test] fn test_first_n_positives_doubled_basic() {
    let v = vec![-3, 1, 4, -1, 5, 9, -2];
    let result = first_n_positives_doubled(v, 3);
    assert_eq!(result, vec![2, 8, 10]);
}

#[test] fn test_first_n_positives_doubled_empty() {
    let v = vec![-1, -2, -3];
    let result = first_n_positives_doubled(v, 2);
    assert_eq!(result.len(), 0);
}

#[test] fn test_first_n_positives_doubled_less_than_n() {
    let v = vec![-1, 3, -2];
    let result = first_n_positives_doubled(v, 10);
    assert_eq!(result, vec![6]);
}
`,
    },
  ],
};
