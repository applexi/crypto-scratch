from crypto import CryptoEnv
from poly import eval_poly, lagrange_interp, lagrange_mults
from itertools import combinations
from sympy import isprime
import secrets

class additive:
    @staticmethod
    def share(s : int, env: CryptoEnv) -> list[int]:
        assert 0 <= s < env.q
        a = [secrets.randbelow(env.q) for _ in range(env.n - 1)]
        a_n = (s - sum(a)) % env.q
        shares = a + [a_n]
        return shares
    
    @staticmethod
    def reconstruct(shares: list[int], env: CryptoEnv) -> int:
        assert len(shares) == env.n
        s_prime = sum(shares) % env.q
        return s_prime

class rss:
    @staticmethod
    def share(s : int, env: CryptoEnv) -> dict[int, dict[tuple[int, ...], int]]:
        assert 0 <= s < env.q
        assert 1 <= env.k <= env.n
        # collection of all k-1 subsets of {1...n}
        Alpha = combinations(range(1, env.n + 1), env.k - 1)
        a_A, a_sum = {}, 0
        prev_A = next(Alpha)
        # for each k - 1 subset A, generate a random a and update a_sum
        for A in Alpha:
            a_A[prev_A] = secrets.randbelow(env.q)
            a_sum = (a_sum + a_A[prev_A]) % env.q
            prev_A = A
        # set last a to s - a_sum
        a_A[prev_A] = (s - a_sum) % env.q
        shares = {i: {} for i in range(1, env.n + 1)}
        # for each party i, set shares[i] = {{A: a} for all A that does not include i}
        for A, a in a_A.items():
            not_in = set(range(1, env.n + 1)) - set(A)
            for i in not_in:
                shares[i][A] = a
        return shares
    
    @staticmethod
    def reconstruct(shares: dict[int, dict[tuple[int, ...], int]], env: CryptoEnv) -> int:
        assert len(shares) == env.k
        Alpha = combinations(range(1, env.n + 1), env.k - 1)
        s_prime = 0
        for A in Alpha:
            for j, share_j in shares.items():
                if j not in A:
                    a = share_j[A]
                    s_prime = (s_prime + a) % env.q
                    break
            else: 
                raise ValueError(f"Missing share for subset {A}")
        return s_prime

class shamir:
    def __init__(self, env: CryptoEnv):
        self.mults = lagrange_mults(env)
        
    @staticmethod
    def share(s : int, env: CryptoEnv) -> dict[int, int]:
        """
        - Pre-reqs:
        - q > n for n distinct points for langrange interp
        """
        assert 0 <= s < env.q
        assert 1 <= env.k <= env.n
        assert env.q > env.n
        a = [secrets.randbelow(env.q) for _ in range(env.k)]
        a[-1] = secrets.randbelow(env.q - 1) + 1
        a[0] = s
        shares = {i: eval_poly(a, i, env) for i in range(1, env.n + 1)}
        return shares
    
    def reconstruct(self, shares: dict[int, int], env: CryptoEnv) -> int:
        """
        Pre-reqs: 
        - q prime for langrange interp modulo inverse
        - at least k shares (unique guaranteed by type dict of shares)
        """
        assert isprime(env.q)
        assert len(shares) >= env.k
        s_prime = lagrange_interp(self.mults, list(shares.items()), env)
        return s_prime