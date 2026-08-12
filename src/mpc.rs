use rand::rngs::SysRng;
use std::{
    collections::{HashMap, HashSet},
    iter::zip,
    ops::Range,
};

use crate::{
    ArithRepSharing, ArithShare, PartyID,
    error::Error,
    sharing::{ArithAddSharing, Arithmetic, Sharing, n_choose_k},
};

pub struct MPC {
    rng: SysRng,
    shared_vars: HashMap<String, HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>>,
    pub k: usize,
    pub n: usize,
}

impl MPC {
    pub fn new(k: usize, n: usize) -> Self {
        let shared_vars = HashMap::new();
        let rng = SysRng;
        MPC {
            rng,
            shared_vars,
            k,
            n,
        }
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

    /// Linearly adds shares of party `id`
    fn party_add(
        &self,
        id: &PartyID,
        shared_a: &HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>,
        shared_b: &HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>,
    ) -> Result<HashMap<Vec<usize>, ArithShare>, Error> {
        let (Some(all_a), Some(all_b)) = (shared_a.get(id), shared_b.get(id)) else {
            return Err(Error::String(format!(
                "Either name_a or name_b does not have the required party: {id}"
            )));
        };
        let mut c = HashMap::new();
        for (subset, a) in all_a {
            let Some(b) = all_b.get(subset) else {
                return Err(Error::String(
                    "Either name_a or name_b does not have a required subset share".to_string(),
                ));
            };
            c.insert(subset.clone(), Arithmetic::add(*a, *b));
        }
        Ok(c)
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
        if shared_a.len() != shared_b.len() {
            return Err(Error::String(format!(
                "Lengths of name_a: {:?} and name_b: {:?} do not match",
                name_a.len(),
                name_b.len()
            )));
        }
        let mut shared_c = HashMap::new();
        // Every party locally adds their shares of a and b
        for id in self.parties() {
            let c = self.party_add(&id, shared_a, shared_b)?;
            shared_c.insert(id, c);
        }
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

    /// Greedily assigns crossterms based on PartyID
    fn greedy_assign(
        &self,
        shared_a: &HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>,
        shared_b: &HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>,
    ) -> Result<HashMap<PartyID, Vec<(Vec<PartyID>, Vec<PartyID>)>>, Error> {
        if shared_a.len() != shared_b.len() {
            return Err(Error::String(format!(
                "Lengths of shared_a: {:?} and shared_b: {:?} do not match",
                shared_a.len(),
                shared_b.len()
            )));
        }
        // Each party is assigned crossterms with the form: (replicated subset, replicated subset)
        let mut assignment: HashMap<PartyID, Vec<(Vec<PartyID>, Vec<PartyID>)>> = HashMap::new();
        let mut crossterms = self.all_crossterms();

        for id in self.parties() {
            let mut to_del = HashSet::new();
            for (subset_a, subset_b) in &crossterms {
                if !(subset_a.contains(&id)) && !(subset_b.contains(&id)) {
                    assignment
                        .entry(id)
                        .or_default()
                        .push((subset_a.clone(), subset_b.clone()));
                    to_del.insert((subset_a.clone(), subset_b.clone()));
                }
            }
            for crossterm in &to_del {
                crossterms.remove(crossterm);
            }
        }
        Ok(assignment)
    }

    /// Given an assignment, return computed results
    ///
    /// If `n - 2(k - 1) == 1`, then all parties' computed result contain some information only they have (a less random n-of-n additive secret sharing)
    fn compute_assignment(
        &self,
        assignment: &HashMap<PartyID, Vec<(Vec<PartyID>, Vec<PartyID>)>>,
        shared_a: &HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>,
        shared_b: &HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>,
    ) -> Result<HashMap<PartyID, ArithShare>, Error> {
        let mut output = HashMap::new();
        // Each party locally computes all assigned and sums them to a single value
        for (id, crossterms) in assignment {
            let (Some(share_a), Some(share_b)) = (shared_a.get(id), shared_b.get(id)) else {
                return Err(Error::String(format!(
                    "Either shared_a or shared_b do not have the required party: {id}"
                )));
            };
            let mut c = Arithmetic::zero();
            for (cross_a, cross_b) in crossterms {
                let (Some(num_a), Some(num_b)) = (share_a.get(cross_a), share_b.get(cross_b))
                else {
                    return Err(Error::String(format!(
                        "The party does not have an assigned subset for the shares given"
                    )));
                };
                c = Arithmetic::add(num_a.wrapping_mul(*num_b), c);
            }
            output.insert(*id, c);
        }
        Ok(output)
    }

    /// Given each parties' created subsets and locally computed shares, does communication between parties
    fn communicate(
        &self,
        party_shares: &mut HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>,
    ) -> Result<(), Error> {
        let init_shares = party_shares.clone();
        for (sender_id, shares) in init_shares {
            for receiver_id in self.parties() {
                if receiver_id != sender_id {
                    for (subset, sender_share) in shares.iter() {
                        if !subset.contains(&receiver_id) {
                            party_shares
                                .entry(receiver_id)
                                .or_default()
                                .entry(subset.clone())
                                .and_modify(|s| *s = Arithmetic::add(*s, *sender_share))
                                .or_insert(*sender_share);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Computes mpc multiplication in a greedy, asymmetric way
    fn mult_greedy(
        &mut self,
        shared_a: &HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>,
        shared_b: &HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>,
    ) -> Result<HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>>, Error> {
        if shared_a.len() != shared_b.len() {
            return Err(Error::String(format!(
                "Lengths of shared_a: {:?} and shared_b: {:?} do not match",
                shared_a.len(),
                shared_b.len()
            )));
        }
        // Greedily assigns all crossterms
        let assignment = self.greedy_assign(shared_a, shared_b)?;
        // Each party locally computes all crossterms and sums them to a single value each
        let computed_assignment = self.compute_assignment(&assignment, shared_a, shared_b)?;
        // Each party then additive secret shares that value based on the number of shares they are supposed to have
        // and assigns each share to a subset they should contribute to
        let num_party_shares = n_choose_k(self.n - 1, self.k - 1);
        let mut shared_c: HashMap<PartyID, HashMap<Vec<PartyID>, ArithShare>> = HashMap::new();
        for (id, secret) in computed_assignment {
            let secret_shares = ArithAddSharing::share(&mut self.rng, secret, num_party_shares)
                .map_err(|_| Error::Rng)?;
            let party_subsets = ArithRepSharing::all_subsets(self.k, self.n, Some(id));
            for (subset, share) in zip(party_subsets, secret_shares) {
                shared_c.entry(id).or_default().insert(subset, share);
            }
        }
        // Each party then gives their (k: subset, v: share) to parties whose index is not within that subset
        self.communicate(&mut shared_c)?;
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
        let shared_c = self.mult_greedy(&shared_a, &shared_b)?;
        self.add_var(out_name, shared_c)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use rand::rngs::SysRng;

    use crate::{
        error::Error, mpc::MPC, sharing::{Arithmetic, Sharing},
    };
    #[test]
    fn test() -> Result<(), Error> {
        let k = 6;
        let n = 11;
        let mut rng = SysRng;
        let mut mpc = MPC::new(k, n);

        let (name_a, name_b, name_c) = ("a".to_string(), "b".to_string(), "c".to_string());
        let real_a = Arithmetic::random_share(&mut rng).map_err(|_| Error::Rng)?;
        let real_b = Arithmetic::random_share(&mut rng).map_err(|_| Error::Rng)?;
        mpc.new_secret(&name_a, real_a)?;
        mpc.new_secret(&name_b, real_b)?;

        // Test addition
        mpc.add(&name_a, &name_b, &name_c)?;

        let crossterms = mpc.all_crossterms();
        // println!("All crossterms:\n {:?}", &crossterms);
        mpc.add(&name_a, &name_b, &name_c)?;
        let c = mpc.open_var(&name_c)?;
        assert!(c == real_a.wrapping_add(real_b));

        let shared_a = mpc.get_shares(&name_a)?;
        let shared_b = mpc.get_shares(&name_b)?;

        let assignment = mpc.greedy_assign(&shared_a, &shared_b)?;
        // println!("Crossterm assignment:\n {:?}", &assignment);
        if assignment.len() != n {
            println!("As expected, there exists a party that was not assigned any shares");
            if !(n - (2 * k - 1) > 1) {
                return Err(Error::String(
                    "There exists a party that was not assigned any shares".to_string(),
                ));
            }
        }

        // All crossterms should have been assigned
        let mut assigned_subsets = HashSet::new();
        for (id, subsets) in &assignment {
            for (subset_a, subset_b) in subsets {
                assigned_subsets.insert((subset_a, subset_b));
                assert!(!subset_a.contains(id) && !subset_b.contains(id));
            }
        }

        if crossterms.len() != assigned_subsets.len() {
            println!("As expected, there are shares that were not assigned to any party:");
            for (subset_a, subset_b) in &crossterms {
                if !assigned_subsets.contains(&(subset_a, subset_b)) {
                    //print!("{:?}", (subset_a, subset_b));
                }
            }
            if !(2 * (k - 1) >= n) {
                return Err(Error::String(
                    "There are shares that were not assigned to any party".to_string(),
                ));
            }
        }
        mpc.mult(&name_a, &name_b, &name_c)?;
        let recon_c = mpc.open_var(&name_c)?;
        let prod = real_a.wrapping_mul(real_b);
        assert!(recon_c == prod);
        Ok(())
    }
}
