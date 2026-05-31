// === Problem: Group Anagrams (LeetCode #49) ===
// Given an array of strings, group the anagrams together.
// An anagram is a word formed by rearranging letters (e.g., "eat" and "tea").
//
// === Pattern: Hash Map (Encoding) ===
// Anagrams share a canonical "signature" — the sorted string. Sort each
// word's characters and use that as the Map key.
//
// === Intuition ===
// Two words are anagrams iff they have the same sorted form.
// "eat" → "aet", "tea" → "aet", "tan" → "ant".
// Build a map from sorted form → list of originals.
//
// === Pattern Recognition ===
// - "Group by some shared property" → compute a key, use Map<String, List>
// - Anagrams → sorted string key
// - Isomorphic strings → character mapping pattern
//
// === Tips ===
// - Sort characters to get a canonical anagram key
// - Use sort_by with .code() for char ordering
// - List::join("") to reconstruct the key string

fn main() {
    val strs = ["eat", "tea", "tan", "ate", "nat", "bat"];
    val groups = group_anagrams(strs);
    for g in groups {
        println("{:?}", g);
    }
}

fn sort_string(s: String) -> String {
    var chars = [];
    for ch in s {
        chars.push(ch);
    }
    chars.sort_by(|a, b| {
        val ao = a.code();
        val bo = b.code();
        if ao < bo { -1 } else if ao > bo { 1 } else { 0 }
    });
    // Convert sorted chars back to string using join
    chars.join("")
}

fn group_anagrams(strs: List) -> List {
    var map = Map::new();
    for s in strs {
        val key = sort_string(s);
        var group = map.get(key.clone()).unwrap_or([]);
        group.push(s);
        map.insert(key, group);
    }
    map.values()
}

#[test]
fn test_basic() {
    val strs = ["eat", "tea", "tan", "ate", "nat", "bat"];
    val result = group_anagrams(strs);
    assert_eq(result.len(), 3);
}

#[test]
fn test_single_word() {
    val strs = ["abc"];
    val result = group_anagrams(strs);
    assert_eq(result.len(), 1);
    assert_eq(result[0].len(), 1);
}

#[test]
fn test_no_anagrams() {
    val strs = ["abc", "def", "ghi"];
    val result = group_anagrams(strs);
    assert_eq(result.len(), 3);
}
