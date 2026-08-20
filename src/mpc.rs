//! This module contains a MPC struct that simulates k-of-n RSS MPC, with the following:
//! - Creation + deletion of "variables" (automatically RSS shared with a given secret value)
//! - Secure MPC addition and (asymmetric) multiplication operations between variables
//! - MPC MSB extraction of a variable, with PRNG and bit adder call optimization

use rand::rngs::SysRng;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::Range,
};

use crate::{
    ArithRepSharing, ArithShare, BitAdder, BitOps, BitRepSharing, BitShare, PartyID, error::Error, prng::PRNG, sharing::{Arithmetic, Binary, ReplicatedSharing, Sharing},
};

pub struct MPC {
    prng: PRNG,
    rng: SysRng,
    shared_vars: HashMap<String, HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>>,
    pub k: usize,
    pub n: usize,
}

impl MPC {
    pub fn new(k: usize, n: usize) -> Self {
        let shared_vars = HashMap::new();
        let rng = SysRng;
        let prng = PRNG::new(k, n);
        MPC {
            prng,
            rng,
            shared_vars,
            k,
            n,
        }
    }

    pub fn set_prng(&mut self) -> Result<(), Error> {
        self.prng.new_seeds(&mut self.rng)
    }

    /// Create a new arithmetic variable with value `secret` that is automatically shared amongst the `n` parties
    pub fn new_secret(&mut self, name: &String, secret: ArithShare) -> Result<(), Error> {
        let arith_shares = ArithRepSharing::share(&mut self.rng, secret, self.k, self.n)?;
        self.add_var(name, arith_shares)?;
        Ok(())
    }

    /// Opens a variable via replicated secret sharing reconstruction
    pub fn open_var(&self, name: &String) -> Result<ArithShare, Error> {
        let shares = self.get_shares(name)?;
        let secret = ArithRepSharing::reconstruct(&shares, self.k, self.n);
        Ok(secret)
    }

    fn add_var(
        &mut self,
        name: &String,
        value: HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>,
    ) -> Result<(), Error> {
        self.shared_vars.insert(name.to_string(), value);
        Ok(())
    }

    /// Deletes a variable
    pub fn delete_var(&mut self, name: &String) -> Result<(), Error> {
        self.shared_vars.remove(name);
        Ok(())
    }

    /// Returns all shares of a variable given its name
    pub fn get_shares(
        &self,
        name: &String,
    ) -> Result<HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>, Error> {
        let Some(shares) = self.shared_vars.get(name) else {
            return Err(Error::String(format!("Name: {name} is not existent")));
        };
        Ok(shares.clone())
    }

    /// Returns the range of all parties
    pub fn parties(&self) -> Range<usize> {
        1..self.n + 1
    }

    /// Linearly adds
    fn add_linear<T: Sharing>(
        &self,
        shared_a: &HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>,
        shared_b: &HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>,
    ) -> Result<HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>, Error> {
        if shared_a.len() != shared_b.len() {
            return Err(Error::String(format!(
                "Lengths of shared_a: {:?} and shared_b: {:?} do not match",
                shared_a.len(),
                shared_b.len()
            )));
        }
        let mut shared_c = HashMap::new();
        // Every party locally adds their shares of a and b
        for id in self.parties() {
            let mut c = HashMap::new();
            let (Some(all_a), Some(all_b)) = (shared_a.get(&id), shared_b.get(&id)) else {
                return Err(Error::String(format!(
                    "Either name_a or name_b does not have the required party: {id}"
                )));
            };
            for (subset, a) in all_a {
                let Some(b) = all_b.get(subset) else {
                    return Err(Error::String(
                        "Either name_a or name_b does not have a required subset share".to_string(),
                    ));
                };
                c.insert(subset.clone(), T::add(*a, *b));
            }
            shared_c.insert(id, c);
        }
        Ok(shared_c)
    }

    /// Computes the addition of variable `name_a` and variable `name_b`, and stores the result as variable `out_name`
    pub fn add(
        &mut self,
        name_a: &String,
        name_b: &String,
        out_name: &String,
    ) -> Result<(), Error> {
        let (Some(shared_a), Some(shared_b)) =
            (self.shared_vars.get(name_a), self.shared_vars.get(name_b))
        else {
            return Err(Error::String(format!(
                "Either name_a: {name_a} or name_b: {name_b} is not existent"
            )));
        };
        let shared_c = self.add_linear::<Arithmetic>(shared_a, shared_b)?;
        self.add_var(out_name, shared_c)?;
        Ok(())
    }

    /// Returns a set of all replicated secret sharing mult crossterms, where each crossterm is of the form:
    ///
    /// `(Vec<`[`PartyID`]`>, Vec<`[`PartyID`]`>)`
    fn all_crossterms(&self) -> HashSet<(Vec<PartyID>, Vec<PartyID>)> {
        let all_subsets_a = ArithRepSharing::all_subsets(self.k, self.n, None);
        let all_subsets_b = ArithRepSharing::all_subsets(self.k, self.n, None);
        let crossterms: Vec<(Vec<usize>, Vec<usize>)> = all_subsets_a
            .into_iter()
            .map(|subset_a| {
                all_subsets_b
                    .iter()
                    .map(move |subset_b| (subset_a.clone(), subset_b.clone()))
            })
            .flatten()
            .collect();
        crossterms.into_iter().collect()
    }

    /// Greedily assigns items based on party id and a given function `can_take`
    fn greedy_assign<T: Eq + Hash + Clone, F: Fn(&PartyID, &T) -> bool>(
        &self,
        mut items: HashSet<T>,
        can_take: F,
    ) -> Result<HashMap<PartyID, Vec<T>>, Error> {
        let mut assignment: HashMap<PartyID, Vec<T>> = HashMap::new();

        for id in self.parties() {
            let mut to_del = HashSet::new();
            for item in &items {
                if can_take(&id, item) {
                    assignment.entry(id).or_default().push(item.clone());
                    to_del.insert(item.clone());
                }
            }
            for item in &to_del {
                items.remove(item);
            }
        }
        Ok(assignment)
    }

    /// Given an assignment and a function `compute`, return computed results
    fn compute_assignment<T: Sharing, A, F: Fn(&PartyID, &A) -> Result<T::Share, Error>>(
        &self,
        assignment: &HashMap<PartyID, Vec<A>>,
        compute: F,
    ) -> Result<HashMap<PartyID, T::Share>, Error> {
        let mut output = HashMap::new();
        // Each party locally computes all assigned items and sums them to a single value
        for (id, items) in assignment {
            let mut c = T::zero();
            for item in items {
                c = T::add(compute(id, item)?, c);
            }
            output.insert(*id, c);
        }
        Ok(output)
    }

    /// Given party `sender_id`'s locally computed value `secret`, simulates resharing preparation using a PRNG
    /// 
    /// Fills `out_sharing` with all PRNG values, and fills `communication_set` with the correction subsets that still require communication
    /// 
    /// Ex: Party 1 has secret v1 -> {(2, 3): PRNG((2, 3)), (2, 4): PRNG((2, 4)), ..., (4, 5): v1 - sum of all previous PRNGS}
    /// - In this example, subset (4, 5) is the correction subset, and requires communication btw parties 1, 2, and 3
    fn prepare_party_reshare<T: Sharing>(&mut self, sender_id: PartyID, secret: T::Share, out_sharing: &mut HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>, communication_set: &mut HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>) -> Result<(), Error> {
        let mut rest = ReplicatedSharing::<T>::all_subsets(self.k, self.n, Some(sender_id));
        let correction_subset = rest.pop().ok_or(Error::String("Subsets failed".to_string()))?;
        let mut sum_randoms = T::zero();
        for random_subset in rest {
            // Every party who should have `random_subset` "locally" draws from the seed to get sender's share
            let random = self.prng.from_seed::<T>(&random_subset)?;
            for receiver_id in self.parties() {
                if !random_subset.contains(&receiver_id) {
                    out_sharing.entry(receiver_id)
                        .or_default()
                        .entry(random_subset.clone())
                        .and_modify(|s| *s = T::add(*s, random))
                        .or_insert(random);
                }
            }
            sum_randoms = T::add(sum_randoms, random);
        }
        // Only need to communicate the correction subsets (the last shares containing the secret correction)
        communication_set.entry(sender_id).or_default().insert(correction_subset, T::sub(secret ,sum_randoms));
        Ok(())
    }

    /// Given (assumed) PRNG-filled sharing `out_sharing` and a set of correction subsets that require communication `communication_set`, completes filling of `out_sharing` with simulated communication
    fn apply_reshare_corrections<T: Sharing>(
        &self,
        out_sharing: &mut HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>,
        communication_set: HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>,
    ) -> Result<(), Error> {
        for (_, shares) in communication_set {
            for receiver_id in self.parties() {
                for (subset, sender_share) in shares.iter() {
                    if !subset.contains(&receiver_id) {
                        out_sharing
                            .entry(receiver_id)
                            .or_default()
                            .entry(subset.clone())
                            .and_modify(|s| *s = T::add(*s, *sender_share))
                            .or_insert(*sender_share);
                    }
                }
            }
        }
        Ok(())
    }

    /// Computes mpc multiplication in a greedy, asymmetric way
    fn mult_greedy<T: Sharing>(
        &mut self,
        shared_a: &HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>,
        shared_b: &HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>,
    ) -> Result<HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>, Error> {
        if shared_a.len() != shared_b.len() {
            return Err(Error::String(format!(
                "Lengths of shared_a: {:?} and shared_b: {:?} do not match",
                shared_a.len(),
                shared_b.len()
            )));
        }
        // Greedily assigns all crossterms
        let crossterms = self.all_crossterms();
        let assignment = self.greedy_assign(crossterms, |id, (subset_a, subset_b)| {
            !subset_a.contains(id) && !subset_b.contains(id)
        })?;
        // Each party locally computes all crossterms and sums them to a single value each
        let compute_crossterm = |id: &PartyID, (cross_a, cross_b): &(Vec<usize>, Vec<usize>)| -> Result<T::Share, Error> {
            let error = Error::String("Either shared_a or shared_b does not have a required item".to_string());
            let share_a = shared_a.get(id).and_then(|x| x.get(cross_a)).ok_or(error.clone())?;
            let share_b = shared_b.get(id).and_then(|x| x.get(cross_b)).ok_or(error)?;
            Ok(T::mul(*share_a, *share_b))
        };
        let computed_assignment = self.compute_assignment::<T, _, _>(&assignment, compute_crossterm)?;

        // Using PRNG, parties "reshare" their single values without communication
        // Correction subsets (subsets that require communication & aren't PRNG) are stored in `communication_set`
        let mut shared_c: HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>> = HashMap::new();
        let mut communication_set: HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>> = HashMap::new();
        for (sender_id, secret) in computed_assignment {
            self.prepare_party_reshare::<T>(sender_id, secret, &mut shared_c, &mut communication_set)?;
        }

        // Parties then communicate to share the final correction subsets to the parties who should have them
        self.apply_reshare_corrections::<T>(&mut shared_c, communication_set)?;
        Ok(shared_c)
    }

    /// Computes the mpc multiplication of variable `name_a` and variable `name_b`, and stores the result as variable `out_name`
    pub fn mult(
        &mut self,
        name_a: &String,
        name_b: &String,
        out_name: &String,
    ) -> Result<(), Error> {
        if 2 * (self.k - 1) >= self.n {
            return Err(Error::String("2(k - 1) >= n, which means there exists a crossterm such that no party can locally compute it".to_string()));
        }
        if self.n - 2 * (self.k - 1) > 1 {
            return Err(Error::String("n - 2(k - 1) > 1, which means there doesn't exist any crossterm that only one party can compute".to_string()));
        }
        assert!(self.n - 2 * (self.k - 1) == 1);
        // Since n - 2(k - 1) = 1, all crossterms can be computed locally by some party, and each party has a crossterm only they can compute
        // Additionally, this implies n should be odd
        let shared_a = self.get_shares(name_a)?;
        let shared_b = self.get_shares(name_b)?;
        let shared_c = self.mult_greedy::<Arithmetic>(&shared_a, &shared_b)?;
        self.add_var(out_name, shared_c)?;
        Ok(())
    }

    /// Opens the MSB of an variable `name_a` using a given bit adder function
    /// - Optimized number of bit adder calls by having parties sum up their local shares 
    pub fn get_msb(&mut self, name: &String, bit_adder: BitAdder<Self>) -> Result<bool, Error> {
        if self.n - 2 * (self.k - 1) != 1 {
            return Err(Error::String(
                "The following must hold true as this function uses mpc mult: n - 2(k - 1) == 1"
                    .to_string(),
            ));
        }
        let sharing = self.get_shares(name)?;

        // Optimize MSB extraction by having parties sum up their local shares
        let subsets = ArithRepSharing::all_subsets(self.k, self.n, None).into_iter().collect();
        let assignment = self.greedy_assign(subsets, |id, subset| !subset.contains(id))?;
        let compute = |id: &PartyID, subset: &Vec<PartyID>| -> Result<ArithShare, Error> {
            let share = sharing.get(id).and_then(|x| x.get(subset)).ok_or(Error::String("Share does not have required items".to_string()))?;
            Ok(*share)
        };
        let computed_assignment = self.compute_assignment::<Arithmetic, _, _>(&assignment, compute)?;

        // Each party locally turns their arithmetic sum (secret) into a vector of RSS shared bits
        // We need sum of all arithmetic shares, stored in `bit_sharings`
        let mut bit_sharings = Vec::new();
        for (sender_id, secret) in computed_assignment {
            let mut bit_shares = Vec::new();
            for bit in Arithmetic::to_binary(secret) {
                let mut shared_bit = <Self as BitOps>::zero(self)?;
                let mut communication_set = HashMap::new();
                self.prepare_party_reshare::<Binary>(sender_id, bit, &mut shared_bit, &mut communication_set)?;
                self.apply_reshare_corrections::<Binary>(&mut shared_bit, communication_set)?;
                bit_shares.push(shared_bit);
            }
            bit_sharings.push(bit_shares);
        }

        // Reduce all bit sharings to a final sum bit sharing using a bit adder
        let mut iter = bit_sharings.into_iter();
        let first = iter
            .next()
            .ok_or(Error::String("No bit sharings".to_string()))?;
        let sum = iter.try_fold(first, |a, b| bit_adder(self, &a, &b))?;
        let last = sum
            .last()
            .ok_or(Error::String("Summed shares has no MSB".to_string()))?;

        // Extract the MSB of the final sum bit sharing
        let msb = BitRepSharing::reconstruct(last, self.k, self.n);
        Ok(msb)
    }
}

impl BitOps for MPC {
    type Bit = HashMap<PartyID, HashMap<Vec<PartyID>, BitShare>>;

    fn and(&mut self, a: &Self::Bit, b: &Self::Bit) -> Result<Self::Bit, Error> {
        self.mult_greedy::<Binary>(a, b)
    }
    fn xor(&mut self, a: &Self::Bit, b: &Self::Bit) -> Result<Self::Bit, Error> {
        self.add_linear::<Binary>(a, b)
    }
    fn zero(&mut self) -> Result<Self::Bit, Error> {
        BitRepSharing::share(&mut self.rng, Binary::zero(), self.k, self.n)
    }
}

#[cfg(test)]
mod test {
    const TIMES: usize = 3;

    use super::*;
    use crate::{full_adder, parallel_prefix};

    // Test basic MPC operations (add, multiply)
    fn mpc_ops_test(k: usize, n: usize) -> Result<(), Error> {
        let mut rng = SysRng;
        let mut mpc = MPC::new(k, n);
        mpc.set_prng()?;

        let (name_a, name_b, name_c) = ("a".to_string(), "b".to_string(), "c".to_string());
        let real_a = Arithmetic::random_share(&mut rng).map_err(|_| Error::Rng)?;
        let real_b = Arithmetic::random_share(&mut rng).map_err(|_| Error::Rng)?;
        mpc.new_secret(&name_a, real_a)?;
        mpc.new_secret(&name_b, real_b)?;

        // Test basic sharing and reconstruction works as intended
        let recon_a = mpc.open_var(&name_a)?;
        let recon_b = mpc.open_var(&name_b)?;
        assert!(recon_a == real_a);
        assert!(recon_b == real_b);

        // Test addition
        mpc.add(&name_a, &name_b, &name_c)?;
        mpc.add(&name_a, &name_b, &name_c)?;
        let c = mpc.open_var(&name_c)?;
        assert!(c == real_a.wrapping_add(real_b));

        // Test all parties have been assigned at least one crossterm
        let crossterms = mpc.all_crossterms();
        let assignment = mpc.greedy_assign(crossterms, |id, (subset_a, subset_b)| {
            !subset_a.contains(id) && !subset_b.contains(id)
        })?;
        if assignment.len() != n {
            if !(n - (2 * k - 1) > 1) {
                return Err(Error::String(
                    "There exists a party that was not assigned any shares".to_string(),
                ));
            } else {
                println!("As expected, there exists a party that was not assigned any shares");
            }
        }

        // Test that all crossterms have been assigned
        let mut assigned_subsets = HashSet::new();
        for (id, subsets) in &assignment {
            for (subset_a, subset_b) in subsets {
                assigned_subsets.insert((subset_a, subset_b));
                assert!(!subset_a.contains(id) && !subset_b.contains(id));
            }
        }
        let crossterms = mpc.all_crossterms();
        if crossterms.len() != assigned_subsets.len() {
            if !(2 * (k - 1) >= n) {
                return Err(Error::String(
                    "There are shares that were not assigned to any party".to_string(),
                ));
            } else {
                println!("As expected, there are shares that were not assigned to any party:");
            }
        }

        // Test multiplication
        mpc.mult(&name_a, &name_b, &name_c)?;
        let recon_c = mpc.open_var(&name_c)?;
        let prod = real_a.wrapping_mul(real_b);
        assert!(recon_c == prod);
        Ok(())
    }

    // Test MPC MSB extraction
    fn msb_test(k: usize, n: usize) -> Result<(), Error> {
        let mut rng = SysRng;
        let mut mpc = MPC::new(k, n);
        mpc.set_prng()?;

        let name_a = "a".to_string();
        let real_a = Arithmetic::random_share(&mut rng).map_err(|_| Error::Rng)?;
        let real_msb = Arithmetic::to_binary(real_a).last().unwrap().to_owned();

        mpc.new_secret(&name_a, real_a)?;
        let mpc_msb = mpc.get_msb(&name_a, full_adder)?;
        assert!(real_msb == mpc_msb);
        let mpc_msb = mpc.get_msb(&name_a, parallel_prefix)?;
        assert!(real_msb == mpc_msb);
        Ok(())
    }

    #[test]
    fn loop_all_test() -> Result<(), Error> {
        let k_of_n: Vec<(usize, usize)> = vec![(2, 3), (3, 5), (4, 7)];
        for (k, n) in k_of_n {
            for _ in 0..TIMES {
                mpc_ops_test(k, n)?;
                msb_test(k, n)?;
            }
        }
        Ok(())
    }
}
