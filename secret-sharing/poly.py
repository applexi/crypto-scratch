from crypto import CryptoEnv
from sympy import isprime


def eval_poly(coeffs: list, val: int, env: CryptoEnv) -> int:
    """
    Evaluates polynomial via Horner's Rule
    """
    val = val % env.q
    d = len(coeffs) - 1
    p = coeffs[d]
    for i in range(d - 1, -1, -1):
        p = (p * val + coeffs[i]) % env.q
    return p

def lagrange_mults(env: CryptoEnv, a: list[int] = None, val: int = 0) -> dict[tuple[int, int], int]:
    """
    Computes and returns langrage interp multipliers

    Parameters:
    - a: x indices for lagrange interp, default = [1...n]
    - val: x value for lagrange interp, default = 0
    """
    mults = {}
    if a is None: a = list(range(1, env.n + 1))
    for a_i in a:
        for a_j in a:
            if a_i != a_j:
                num, den = (val - a_j) % env.q, (a_i - a_j) % env.q
                mults[(a_i, a_j)] = (num * pow(den, -1, env.q)) % env.q
    return mults

def lagrange_interp(mults : dict[tuple[int, int], int], points: list[tuple[int, int]], env : CryptoEnv) -> int:
    """
    Pre-reqs:
    - q prime for langrange interp modulo inverse
    """
    assert isprime(env.q)
    g_0 = 0
    for (a_i, b_i) in points:
        mult_i = 1
        for (a_j, _) in points:
            if a_i != a_j and (a_i, a_j) in mults:
                mult_i = (mult_i * mults[(a_i, a_j)]) % env.q
        g_0 = (g_0 + b_i * mult_i) % env.q
    return g_0