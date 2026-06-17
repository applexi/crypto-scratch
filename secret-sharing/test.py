import pytest
from crypto import CryptoEnv
from schemes import additive, rss, shamir
from itertools import combinations

Q = 13
secret = 11
n = 5
k = 3

@pytest.mark.parametrize(
    "scheme, split, recon",
    [
        (additive, additive.share, additive.reconstruct),
        (rss, rss.share, rss.reconstruct),
        (shamir, shamir.share, shamir.reconstruct),
    ],
    ids=["additive", "rss", "shamir"],
)
def test_k_of_n_secret_recovery(scheme, split, recon):
    env = CryptoEnv(n=n, k=k, q=Q)
    shares = split(secret, env)
    subsets = combinations(range(1, env.n + 1), env.k)

    if scheme is additive:
        # always assumes n-of-n regardless of k 
        recovered = recon(shares, env)
        assert recovered == secret
    elif scheme is rss:
        for subset in subsets:
            recovered = recon({i: shares[i] for i in subset}, env)
            assert recovered == secret
    elif scheme is shamir:
        recovered = recon(shares[:k], env)
        assert recovered == secret
