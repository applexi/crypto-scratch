//! This module contains a mock PRNG and assumes seeds are known to respective parties beforehand

use std::{collections::HashMap};
use rand::{TryCryptoRng, rngs::ChaCha20Rng, rand_core::SeedableRng};

use crate::{ArithRepSharing, PartyID, error::Error, sharing::Sharing};

pub struct PRNG {
    seeds: HashMap<Vec<PartyID>, [u8; 32]>,
    rngs: HashMap<Vec<PartyID>, ChaCha20Rng>,
}

impl PRNG {
    /// Given parameters `k` and `n` for a RSS scheme, returns a new PRNG with all internal seeds and rngs zeroed
    pub fn new(k: usize, n: usize) -> Self {
        let seeds = ArithRepSharing::all_subsets(k, n, None)
            .into_iter()
            .map(|subset| (subset, [0u8; 32]))
            .collect();
        let rngs = ArithRepSharing::all_subsets(k, n, None)
            .into_iter()
            .map(|subset| (subset, ChaCha20Rng::from_seed([0u8; 32])))
            .collect();
        PRNG { seeds, rngs  }
    }

    /// Refreshes all internal randomness
    pub fn new_seeds<R: TryCryptoRng>(&mut self, rng: &mut R) -> Result<(), Error> {
        assert!(self.seeds.len() != 0);
        for bytes in self.seeds.values_mut() {
            rng.try_fill_bytes(bytes).map_err(|_| Error::Rng)?;
        }
        for (subset, rng) in self.rngs.iter_mut() {
            *rng = ChaCha20Rng::from_seed(self.seeds[subset]);
        }
        Ok(())
    }

    /// Given a seeded subset, returns the respective random 
    pub fn from_seed<T: Sharing>(&mut self, subset: &Vec<PartyID>) -> Result<T::Share, Error> {
        let rng = self.rngs.get_mut(subset).ok_or(Error::String("Subset not found".to_string()))?;
        T::random_share(rng).map_err(|_| Error::Rng)
    }
}
