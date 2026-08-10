use std::{collections::HashMap, fmt::Debug, marker::PhantomData};

use itertools::Itertools;
use rand::TryCryptoRng;

mod arithmetic;
mod binary;
mod bitadder;
pub use arithmetic::{ArithShare, Arithmetic};
pub use binary::{Binary, BitShare};

use crate::error::Error;

#[cfg(test)]
mod test;

pub type PartyID = usize;
pub type ReplicatedShares<T: Sharing> = HashMap<Vec<PartyID>, T::Share>;

pub type ArithmeticSharing = ReplicatedSharing<Arithmetic>;
/// `HashMap<Vec<PartyID>, ArithShare>`
pub type ArithmeticShares = ReplicatedShares<Arithmetic>;

pub type BinarySharing = ReplicatedSharing<Binary>;
/// `HashMap<Vec<PartyID>, BitShare>`
pub type BinaryShares = ReplicatedShares<Binary>;

pub trait Sharing {
    type Share: Copy + Debug;

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
    pub k: usize,
    pub n: usize,
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
    ) -> Result<HashMap<PartyID, ReplicatedShares<T>>, Error> {
        let subsets: Vec<Vec<PartyID>> = (1..self.n + 1).combinations(self.k - 1).collect();
        let Some((last, rest)) = subsets.split_last() else {
            return Err(Error::String("No k - 1 subsets".to_string()));
        };
        let mut a_sum = T::zero();
        let mut a_map = HashMap::new();
        for subset in rest {
            let a = T::random_share(rng).map_err(|_| Error::Rng)?;
            a_sum = T::add(a_sum, a);
            a_map.insert(subset, a);
        }
        a_map.insert(last, T::sub(secret, a_sum));

        let mut shares: HashMap<PartyID, ReplicatedShares<T>> =
            (1..self.n + 1).map(|i| (i, HashMap::new())).collect();
        for (subset, a) in a_map {
            let not_in = (1..self.n + 1).filter(|x| !subset.contains(x));
            for i in not_in {
                shares.entry(i).or_default().insert(subset.clone(), a);
            }
        }
        Ok(shares)
    }

    /// Given [`k`][`Self::k`] shares, returns a reconstructed secret
    pub fn reconstruct(&self, shares: &HashMap<PartyID, ReplicatedShares<T>>) -> T::Share {
        assert!(shares.len() == self.k);
        let subsets: Vec<Vec<PartyID>> = (1..self.n + 1).combinations(self.k - 1).collect();
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
