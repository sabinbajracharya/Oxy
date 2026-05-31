// Read one integer per line from stdin and print the sum.
// Usage:  cat input.txt | oxy run examples/stdin_sum.ox
//
// Typical Advent-of-Code pattern: slurp stdin, iterate lines, parse, sum.

fn main() {
    val input = std::io::read_to_string().unwrap();
    var total = 0;
    for line in input.lines() {
        val trimmed = line.trim();
        if trimmed.len() == 0 {
            continue;
        }
        total = total + trimmed.parse_int().unwrap();
    }
    println("{}", total);
}
