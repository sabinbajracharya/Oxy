// === Problem: Number of Islands (LeetCode #200) ===
// Given a 2D grid of '1's (land) and '0's (water), count the number of
// islands. An island is a group of adjacent land cells (4-directionally).
//
// === Pattern: Graph DFS (Flood Fill) ===
// Scan the grid. When you find unvisited land ('1'), increment count and
// flood-fill with DFS to mark all connected land as '0' (sink the island).
// Modify the grid in place to avoid visited tracking.
//
// === Intuition ===
// Each '1' is a node. Adjacent '1's are connected. DFS: when you find a
// '1', increment count and recursively sink all connected land cells
// to '0' by modifying the grid.
//
// === Pattern Recognition ===
// - "Number of connected components in grid" → flood fill
// - "Islands" → DFS + grid modification

fn main() {
    let grid = vec![
        vec!['1', '1', '1', '1', '0'],
        vec!['1', '1', '0', '1', '0'],
        vec!['1', '1', '0', '0', '0'],
        vec!['0', '0', '0', '0', '0']
    ];
    println!("{}", num_islands(grid));
}

fn dfs(grid: Vec, i: i64, j: i64) -> Vec {
    let rows = grid.len();
    let cols = grid[0].len();
    if i < 0 || i >= rows || j < 0 || j >= cols {
        return grid;
    }
    if grid[i][j] != '1' {
        return grid;
    }
    // Mark as visited by setting to '0'
    let mut row = grid[i];
    row[j] = '0';
    grid[i] = row;
    // Recurse in 4 directions
    let mut g = dfs(grid, i + 1, j);
    g = dfs(g, i - 1, j);
    g = dfs(g, i, j + 1);
    g = dfs(g, i, j - 1);
    g
}

fn num_islands(grid: Vec) -> i64 {
    if grid.len() == 0 {
        return 0;
    }
    let rows = grid.len();
    let cols = grid[0].len();
    let mut count = 0i64;
    let mut g = grid;
    let mut i = 0i64;
    while i < rows {
        let mut j = 0i64;
        while j < cols {
            if g[i][j] == '1' {
                count = count + 1;
                g = dfs(g, i, j);
            }
            j = j + 1;
        }
        i = i + 1;
    }
    count
}

#[test]
fn test_example() {
    let grid = vec![
        vec!['1', '1', '1', '1', '0'],
        vec!['1', '1', '0', '1', '0'],
        vec!['1', '1', '0', '0', '0'],
        vec!['0', '0', '0', '0', '0']
    ];
    assert_eq!(num_islands(grid), 1);
}

#[test]
fn test_multiple() {
    let grid = vec![
        vec!['1', '1', '0', '0', '0'],
        vec!['1', '1', '0', '0', '0'],
        vec!['0', '0', '1', '0', '0'],
        vec!['0', '0', '0', '1', '1']
    ];
    assert_eq!(num_islands(grid), 3);
}
