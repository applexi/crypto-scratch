from dataclasses import dataclass

@dataclass
class CryptoEnv:
    """
    Attributes:
    - n: num of parties
    - k: threshold
    - q: ring/field (Z_q/F_q)
    """
    n: int
    k: int
    q: int