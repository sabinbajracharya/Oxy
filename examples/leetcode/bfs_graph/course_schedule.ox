// === Problem: Course Schedule (LeetCode #207) ===
// There are n courses labeled 0 to n-1. Some have prerequisites pairs
// [a, b] meaning you must take b before a. Return true if you can finish
// all courses (no cycles in the dependency graph).
//
// === Pattern: Graph Topological Sort (Cycle Detection) ===
// Build an adjacency list and track in-degrees. Use BFS (Kahn's algorithm):
// start with courses that have 0 prerequisites, remove them, decrement
// their neighbors' in-degrees. If all courses are processed, no cycle.
//
// === Intuition ===
// A cycle in prerequisites means impossible. Topological sort detects cycles:
// nodes with in-degree 0 have no unmet prerequisites. Remove them and
// their outgoing edges. If any nodes remain unprocessed → cycle.
//
// === Pattern Recognition ===
// - "Course prerequisites" / "dependency order" → topological sort
// - "Detect cycle in directed graph" → Kahn's (BFS) or DFS with colors
// - BFS uses in-degree tracking, DFS uses visited/processing states
//
// === Tips ===
// - Build graph: adjacency list from prerequisites
// - Track in_degree for each node
// - BFS: push all 0 in-degree nodes, process, decrement neighbors

fn main() {
    val prereqs = [[1, 0], [2, 1], [3, 2]];
    io::println("{}", can_finish(4, prereqs));
}

fn can_finish(num_courses: Int, prerequisites: List) -> bool {
    // Build adjacency list and in-degree array
    val n = num_courses;
    var graph = [];
    var indegree = [];
    for _i in 0..n {
        graph.push([]);
        indegree.push(0);
    }
    for pr in prerequisites {
        val course = pr[0];
        val prereq = pr[1];
        graph[prereq].push(course);
        indegree[course] = indegree[course] + 1;
    }
    // BFS: start with all 0 in-degree nodes
    var queue = VecDeque::new();
    for j in 0..n {
        if indegree[j] == 0 {
            queue.push_back(j);
        }
    }
    var processed = 0;
    while val Some(node) = queue.pop_front() {
        processed = processed + 1;
        for neighbor in graph[node] {
            indegree[neighbor] = indegree[neighbor] - 1;
            if indegree[neighbor] == 0 {
                queue.push_back(neighbor);
            }
        }
    }
    if processed == n { true } else { false }
}

#[test]
fn test_possible() {
    val prereqs = [[1, 0], [2, 1], [3, 2]];
    assert::true(can_finish(4, prereqs));
}

#[test]
fn test_cycle() {
    val prereqs = [[0, 1], [1, 0]];
    assert::true(!can_finish(2, prereqs));
}

#[test]
fn test_no_prereqs() {
    assert::true(can_finish(3, []));
}
