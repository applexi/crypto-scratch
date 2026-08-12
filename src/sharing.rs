use std::{collections::HashMap, fmt::Debug, marker::PhantomData};

use itertools::Itertools;
use rand::TryCryptoRng;

mod arithmetic;
mod binary;
mod bitadder;
mod helper;
pub use arithmetic::{ArithShare, Arithmetic};
pub use binary::{Binary, BitShare};
pub use helper::n_choose_k;

use crate::error::Error;

#[cfg(test)]
mod test;

pub type PartyID = usize;

pub type ArithRepSharing = ReplicatedSharing<Arithmetic>;
pub type BitRepSharing = ReplicatedSharing<Binary>;

pub type ArithAddSharing = AdditiveSharing<Arithmetic>;
pub type BitAddSharing = AdditiveSharing<Binary>;

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
    _sharing: PhantomData<T>,
}

impl<T: Sharing> ReplicatedSharing<T> {
    /// Returns `n` shares from a given secret
    pub fn share<R: TryCryptoRng>(
        rng: &mut R,
        secret: T::Share,
        k: usize,
        n: usize,
    ) -> Result<HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>, Error> {
        let subsets: Vec<Vec<PartyID>> = ArithRepSharing::all_subsets(k, n, None);
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

        let mut shares: HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>> =
            (1..n + 1).map(|i| (i, HashMap::new())).collect();
        for (subset, a) in a_map {
            let not_in = (1..n + 1).filter(|x| !subset.contains(x));
            for i in not_in {
                shares.entry(i).or_default().insert(subset.clone(), a);
            }
        }
        Ok(shares)
    }

    /// Given at least `k` shares, returns a reconstructed secret
    pub fn reconstruct(
        shares: &HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>,
        k: usize,
        n: usize,
    ) -> T::Share {
        assert!(shares.len() >= k);
        let subsets: Vec<Vec<PartyID>> = ArithRepSharing::all_subsets(k, n, None);
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

    /// Returns all replicated secret sharing subsets of a k-of-n scheme
    pub fn all_subsets(k: usize, n: usize, id: Option<PartyID>) -> Vec<Vec<PartyID>> {
        let all_subsets = (1..n + 1).combinations(k - 1).collect();
        let Some(id) = id else { return all_subsets };
        all_subsets
            .into_iter()
            .filter(|x| !x.contains(&id))
            .collect()
    }
}

pub struct AdditiveSharing<T: Sharing> {
    _sharing: PhantomData<T>,
}

impl<T: Sharing> AdditiveSharing<T> {
    /// Returns `n` shares from a given secret
    pub fn share<R: TryCryptoRng>(
        rng: &mut R,
        secret: T::Share,
        n: usize,
    ) -> Result<Vec<T::Share>, R::Error> {
        let mut a: Vec<T::Share> = (0..n - 1)
            .map(|_| T::random_share(rng))
            .collect::<Result<Vec<T::Share>, R::Error>>()?;
        let sum = T::sum(&a);
        let a_n = T::sub(secret, sum);
        a.push(a_n);
        Ok(a)
    }

    /// Given at least `k` shares, returns a reconstructed secret
    pub fn reconstruct(
        shares: &[T::Share],
        n: usize,
    ) -> T::Share {
        assert!(shares.len() == n);
        T::sum(shares)
    }
}
