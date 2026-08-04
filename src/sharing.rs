use std::{collections::HashMap, marker::PhantomData};

use rand::TryCryptoRng;
use itertools::{Itertools};

mod binary;
mod arithmetic;
pub use binary::{Binary, BitShare};
pub use arithmetic::{Arithmetic, ArithShare};

use crate::error::Error;

pub type ArithmeticSharing = ReplicatedSharing<Arithmetic>;
pub type BinarySharing = ReplicatedSharing<Binary>;

pub trait Sharing {
    type Share: Copy + std::fmt::Debug;

    fn zero() -> Self::Share;
    fn random_share<T: TryCryptoRng>(rng: &mut T) -> Result<Self::Share, T::Error>;
    fn add(a: Self::Share, b: Self::Share) -> Self::Share;
    /// a - b
    fn sub(a: Self::Share, b: Self::Share) -> Self::Share;

    fn sum(shares: &[Self::Share]) -> Self::Share {
        shares
            .iter()
            .fold(Self::zero(), |acc, share| Self::add(acc, *share))
    }
}

pub struct ReplicatedSharing<T: Sharing> {
    k: usize,
    n: usize,
    _sharing: PhantomData<T>,
}

impl<T: Sharing> ReplicatedSharing<T> {
    /// Panics if either `k` or `n` are negative, and if `k > n`
    pub fn new(k: usize, n: usize) -> Self {
        assert!(1 <= k && k <= n);
        ReplicatedSharing {
            k,
            n,
            _sharing: PhantomData,
        }
    }
    /// Returns [`n`][`Self::n`] shares from a given secret
    pub fn share<R: TryCryptoRng>(
        &self,
        rng: &mut R,
        secret: T::Share,
    ) -> Result<HashMap<usize, HashMap<Vec<usize>, T::Share>>, Error> {
        let subsets: Vec<Vec<usize>> = (1..self.n + 1)
            .combinations(self.k - 1)
            .collect();
        let Some((last, rest)) = subsets.split_last() else {
            return Err(Error::String("No k - 1 subsets".to_string()))
        };
        let mut a_sum = T::zero();
        let mut a_map = HashMap::new();
        for subset in rest {
            let a = T::random_share(rng).map_err(|_| Error::Rng)?;
            a_sum = T::add(a_sum, a);
            a_map.insert(subset, a);
        }
        a_map.insert(last, T::sub(secret, a_sum));
        
        let mut shares: HashMap<usize, HashMap<Vec<usize>, <T as Sharing>::Share>> = (1..self.n + 1)
            .map(|i| (i, HashMap::new()))
            .collect();
        for (subset, a) in a_map {
            let not_in = (1..self.n + 1).filter(|x| !subset.contains(x));
            for i in not_in {
                shares.entry(i).or_default().insert(subset.clone(), a);
            }
        }
        Ok(shares)
    }

    /// Given [`k`][`Self::k`] shares, returns a reconstructed secret
    pub fn reconstruct(&self, shares: &HashMap<usize, HashMap<Vec<usize>, T::Share>>) -> T::Share {
        assert!(shares.len() == self.k);
        let subsets: Vec<Vec<usize>> = (1..self.n + 1)
            .combinations(self.k - 1)
            .collect();
        let mut reconstructed_s = T::zero();
        for subset in subsets {
            for (j, share_j) in shares {
                if !subset.contains(&j) {
                    let a = share_j[&subset];
                    reconstructed_s = T::add(reconstructed_s, a);
                    break;
                }
            } 
        }
        reconstructed_s
    }
}

#[cfg(test)]
mod tests {
use super::*;
    use rand::rngs::SysRng;

    #[test]
    fn sharing_correctness_test() -> Result<(), Error>{
        let k = 2;
        let n = 3;
        let arithmetic = ArithmeticSharing::new(k, n);
        let binary = BinarySharing::new(k, n);
        let mut rng = SysRng;

        let secret_arith = Arithmetic::random_share(&mut rng)
            .expect("Could not generate random arithmetic share");
        let arith_shares = arithmetic.share(&mut rng, secret_arith)?;
        let secret_bit = Binary::random_share(&mut rng)
            .expect("Could not generate random binary share");
        let binary_shares = binary.share(&mut rng, secret_bit)?;

        let k_subsets: Vec<Vec<usize>>  = (1..n + 1).combinations(k).collect();
        for k_subset in k_subsets {
            let k_arith_shares: HashMap<usize, HashMap<Vec<usize>, ArithShare>> = arith_shares
                .iter()
                .filter_map(|(i, x)| k_subset.contains(i).then(|| (*i, x.clone())))
                .collect();
            let k_binary_shares: HashMap<usize, HashMap<Vec<usize>, BitShare>> = binary_shares
                .iter()
                .filter_map(|(i, x)| k_subset.contains(i).then(|| (*i, x.clone())))
                .collect();
            let r_secret_arith = arithmetic.reconstruct(&k_arith_shares);
            let r_secret_binary = binary.reconstruct(&k_binary_shares);
            assert!(secret_arith == r_secret_arith);
            assert!(secret_bit == r_secret_binary);
        }
        Ok(())
    }
}

