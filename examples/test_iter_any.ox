fn main() {
    let v = vec(1, 2, 3, 4, 5);
    println("before any");
    let result = v.iter().any(|x| x % 2 == 0);
    println("{}", result);
}
