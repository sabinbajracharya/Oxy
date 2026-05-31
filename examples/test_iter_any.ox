fn main() {
    val v = [1, 2, 3, 4, 5];
    println("before any");
    val result = v.iter().any(|x| x % 2 == 0);
    println("{}", result);
}
