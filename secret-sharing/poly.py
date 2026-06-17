from crypto import CryptoEnv
from sympy import isprime


def eval_poly(coeffs: list, val: int, env: CryptoEnv) -> int:
    val = val % env.q
    d = len(coeffs) - 1
    p = coeffs[d]
    for i in range(d - 1, -1, -1):
        p = (p * val + coeffs[i]) % env.q
    return p

def lagrange_interp(val: int, points: list[tuple[int, int]], env : CryptoEnv) -> int:
    assert isprime(env.q)
    R = [0] * len(points)
    for i, (a_i, b_i) in enumerate(points):
        num, den = 1, 1
        for j, (a_j, _) in enumerate(points):
            if i != j:
                num = (num * (val - a_j)) % env.q
                den = (den * (a_i - a_j)) % env.q
        R[i] = (b_i * num * pow(den, -1, env.q)) % env.q
    return sum(R) % env.q