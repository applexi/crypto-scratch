fn factorial(n: usize) -> usize {
    (1..n + 1).product()
}

pub fn n_choose_k(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    factorial(n) / (factorial(k) * factorial(n - k))
}