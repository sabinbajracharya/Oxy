// === Problem: Group Anagrams (LeetCode #49) ===
// Given an array of strings, group the anagrams together.
// An anagram is a word formed by rearranging letters (e.g., "eat" and "tea").
//
// === Pattern: Hash Map (Encoding) ===
// Anagrams share a canonical "signature" — the sorted string. Sort each
// word's characters and use that as the HashMap key.
//
// === Intuition ===
// Two words are anagrams iff they have the same sorted form.
// "eat" → "aet", "tea" → "aet", "tan" → "ant".
// Build a map from sorted form → list of originals.
//
// === Pattern Recognition ===
// - "Group by some shared property" → compute a key, use HashMap<String, Vec>
// - Anagrams → sorted string key
// - Isomorphic strings → character mapping pattern
//
// === Tips ===
// - Sort characters to get a canonical anagram key
// - Use sort_by with .code() for char ordering
// - Vec::join("") to reconstruct the key string

fn main() {
    let strs = vec!["eat", "tea", "tan", "ate", "nat", "bat"];
    let groups = group_anagrams(strs);
    for g in groups {
        println!("{:?}", g);
    }
}

fn sort_string(s: String) -> String {
    let mut chars = vec![];
    for ch in s {
        chars.push(ch);
    }
    chars.sort_by(|a, b| {
        let ao = a.code();
        let bo = b.code();
        if ao < bo { -1 } else if ao > bo { 1 } else { 0 }
    });
    // Convert sorted chars back to string using join
    chars.join("")
}

fn group_anagrams(strs: Vec) -> Vec {
    let mut map = HashMap::new();
    for s in strs {
        let key = sort_string(s);
        let mut group = map.get(key.clone()).unwrap_or(vec![]);
        group.push(s);
        map.insert(key, group);
    }
    map.values()
}

#[test]
fn test_basic() {
    let strs = vec!["eat", "tea", "tan", "ate", "nat", "bat"];
    let result = group_anagrams(strs);
    assert_eq!(result.len(), 3);
}

#[test]
fn test_single_word() {
    let strs = vec!["abc"];
    let result = group_anagrams(strs);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 1);
}

#[test]
fn test_no_anagrams() {
    let strs = vec!["abc", "def", "ghi"];
    let result = group_anagrams(strs);
    assert_eq!(result.len(), 3);
}
