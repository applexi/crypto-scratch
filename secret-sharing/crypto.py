from dataclasses import dataclass

@dataclass
class CryptoEnv:
    n : int # num of parties
    k : int # threshold
    q : int # ring/field (Z_q/F_q)