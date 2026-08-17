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

    /// Greedily assigns items based on party id
    fn greedy_assign<T: Eq + Hash + Clone, F: Fn(&PartyID, &T) -> bool>(
        &self,
        mut items: HashSet<T>,
        can_take: F,
    ) -> Result<HashMap<PartyID, Vec<T>>, Error> {
        // Each party is assigned crossterms with the form: (replicated subset, replicated subset)
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

    /// Given an assignment, return computed results
    ///
    /// If `n - 2(k - 1) == 1`, then all parties' computed result contain some information only they have (a less random n-of-n additive secret sharing)
    fn compute_assignment<T: Sharing>(
        &self,
        assignment: &HashMap<PartyID, Vec<(Vec<PartyID>, Vec<PartyID>)>>,
        shared_a: &HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>,
        shared_b: &HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>,
    ) -> Result<HashMap<PartyID, T::Share>, Error> {
        let mut output = HashMap::new();
        // Each party locally computes all assigned and sums them to a single value
        for (id, crossterms) in assignment {
            let (Some(share_a), Some(share_b)) = (shared_a.get(id), shared_b.get(id)) else {
                return Err(Error::String(format!(
                    "Either shared_a or shared_b do not have the required party: {id}"
                )));
            };
            let mut c = T::zero();
            for (cross_a, cross_b) in crossterms {
                let (Some(num_a), Some(num_b)) = (share_a.get(cross_a), share_b.get(cross_b))
                else {
                    return Err(Error::String(format!(
                        "The party does not have an assigned subset for the shares given"
                    )));
                };
                c = T::add(T::mul(*num_a, *num_b), c);
            }
            output.insert(*id, c);
        }
        Ok(output)
    }

    /// Given each parties' created subsets and locally computed shares, does communication between parties
    fn communicate<T: Sharing>(
        &self,
        party_shares: &mut HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>,
        communication_set: HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>>,
    ) -> Result<(), Error> {
        for (_, shares) in communication_set {
            for receiver_id in self.parties() {
                for (subset, sender_share) in shares.iter() {
                    if !subset.contains(&receiver_id) {
                        party_shares
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
        let computed_assignment = self.compute_assignment::<T>(&assignment, shared_a, shared_b)?;

        // MPC system uses PRNG to simulate converting the single value to a replicated secret sharing
        let mut shared_c: HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>> = HashMap::new();
        let mut communication_set: HashMap<PartyID, HashMap<Vec<PartyID>, T::Share>> = HashMap::new();
        for (sender_id, secret) in computed_assignment {
            let mut rest = ReplicatedSharing::<T>::all_subsets(self.k, self.n, Some(sender_id));
            let correction_subset = rest.pop().ok_or(Error::String("Subsets failed".to_string()))?;
            let mut sum_randoms = T::zero();
            for random_subset in rest {
                // Every party who should have `random_subset` "locally" draws from the seed to get sender's share
                let random = self.prng.from_seed::<T>(&random_subset)?;
                for receiver_id in self.parties() {
                    if !random_subset.contains(&receiver_id) {
                        shared_c.entry(receiver_id)
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
        }

        // Each party then gives their (k: subset, v: share) to parties whose index is not within that subset
        self.communicate::<T>(&mut shared_c, communication_set)?;
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

    /// Given an arithmetic sharing, returns a vector of bit sharings
    ///
    /// Based on Rep3 Equation 2
    fn shared_arith_to_bits(
        &self,
        subset: &Vec<PartyID>,
        share: &HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>,
    ) -> Result<Vec<HashMap<PartyID, HashMap<Vec<PartyID>, BitShare>>>, Error> {
        // An arithmetic sharing is converted into a vector of binary sharings
        let mut bit_sharings: Vec<HashMap<PartyID, HashMap<Vec<PartyID>, BitShare>>> = Vec::new();

        for i in 0..ArithShare::BITS {
            let mut bit_sharing: HashMap<PartyID, HashMap<Vec<usize>, BitShare>> = HashMap::new();
            for id in self.parties() {
                let arith_share = share
                    .get(&id)
                    .and_then(|x| x.get(subset))
                    .unwrap_or(&Arithmetic::zero())
                    .clone();
                let bit = Arithmetic::to_binary(arith_share)
                    .get(i as usize)
                    .unwrap_or(&Binary::zero())
                    .clone();
                let mut bit_share: HashMap<Vec<PartyID>, BitShare> = HashMap::new();
                // All subsets the party should have in a k-of-n RSS
                let new_subsets = ArithRepSharing::all_subsets(self.k, self.n, Some(id));
                for new_subset in new_subsets {
                    // Assumes subsets are ordered (which they are currently using combinations)
                    // If the party has that original arithmetic subset share, then it has the bits (sparse embedding)
                    if new_subset == *subset {
                        bit_share.insert(new_subset, bit);
                    } else {
                        bit_share.insert(new_subset, Binary::zero());
                    }
                }
                bit_sharing.insert(id, bit_share);
            }
            bit_sharings.push(bit_sharing);
        }
        Ok(bit_sharings)
    }

    /// Opens the MSB of an variable `name_a` using a given bit adder function
    pub fn get_msb(&mut self, name: &String, bit_adder: BitAdder<Self>) -> Result<bool, Error> {
        if self.n - 2 * (self.k - 1) != 1 {
            return Err(Error::String(
                "The following must hold true as this function uses mpc mult: n - 2(k - 1) == 1"
                    .to_string(),
            ));
        }
        let share = self.get_shares(name)?;
        let bit_sharings: Vec<Vec<HashMap<PartyID, HashMap<Vec<PartyID>, BitShare>>>> =
            ArithRepSharing::all_subsets(self.k, self.n, None)
                .iter()
                .map(|subset| self.shared_arith_to_bits(subset, &share))
                .collect::<Result<Vec<_>, _>>()?;
        let mut iter = bit_sharings.into_iter();
        let first = iter
            .next()
            .ok_or(Error::String("No bit sharings".to_string()))?;
        let sum = iter.try_fold(first, |a, b| bit_adder(self, &a, &b))?;
        let last = sum
            .last()
            .ok_or(Error::String("Summed shares has no MSB".to_string()))?;
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
