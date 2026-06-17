import pytest
from crypto import CryptoEnv
from schemes import additive, rss, shamir

Q = 13
secret = 11

@pytest.mark.parametrize(
    "scheme, split, recon",
    [
        (additive, additive.share, additive.reconstruct),
        (rss, rss.share, rss.reconstruct),
        (shamir, shamir.share, shamir.reconstruct),
    ],
    ids=["additive", "rss", "shamir"],
)
def test_2_of_3_secret_recovery(scheme, split, recon):
    env = CryptoEnv(n=3, k=2, q=Q)
    shares = split(secret, env)

    if scheme is additive:
        recovered = recon(shares, env)
    elif scheme is rss:
        recovered = recon({1: shares[1], 2: shares[2]}, env)
    else:
        recovered = recon(shares[:2], env)

    assert recovered == secret


@pytest.mark.parametrize(
    "scheme, split, recon",
    [
        (additive, additive.share, additive.reconstruct),
        (rss, rss.share, rss.reconstruct),
        (shamir, shamir.share, shamir.reconstruct),
    ],
    ids=["additive", "rss", "shamir"],
)
def test_3_of_3_secret_recovery(scheme, split, recon):
    env = CryptoEnv(n=3, k=3, q=Q)
    shares = split(secret, env)

    if scheme is rss:
        recovered = recon({1: shares[1], 2: shares[2], 3: shares[3]}, env)
    else:
        recovered = recon(shares, env)

    assert recovered == secret
