use std::{collections::HashMap};
use rand::rngs::SysRng;

use crate::{ArithShare, ArithmeticShares, ArithmeticSharing, BinarySharing, PartyID, error::Error, sharing::{Arithmetic, Sharing}};


pub struct MPC {
    rng: SysRng,
    shared_vars: HashMap<String, HashMap<PartyID, ArithmeticShares>>,
    arith_sharing: ArithmeticSharing,
    bit_sharing: BinarySharing,
}

impl MPC {
    pub fn new(k: usize, n: usize) -> Self {
        let arith_sharing = ArithmeticSharing::new(k, n);
        let bit_sharing = BinarySharing::new(k, n);
        let shared_vars = HashMap::new();
        let rng = SysRng;
        MPC { rng, shared_vars, arith_sharing, bit_sharing }
    }

    /// Create a new arithmetic variable with value `secret` that is automatically shared amongst the `n` parties
    pub fn new_secret(&mut self, name: String, secret: ArithShare) -> Result<(), Error> {
        let arith_shares = self.arith_sharing.share(&mut self.rng, secret)?;
        self.shared_vars.insert(name, arith_shares);
        Ok(())
    }

    fn add_var(&mut self, name:String, value: HashMap<PartyID, ArithmeticShares>) -> Result<(), Error> {
        self.shared_vars.insert(name, value);
        Ok(())
    }

    /// Deletes a variable
    pub fn delete_var(&mut self, name: String) -> Result<(), Error> {
        self.shared_vars.remove(&name);
        Ok(())
    }

    fn party_add(&self, id: &usize, shared_a: &HashMap<usize, HashMap<Vec<usize>, ArithShare>>, shared_b: &HashMap<usize, HashMap<Vec<usize>, ArithShare>>) -> Result<HashMap<Vec<usize>, ArithShare>, Error> {
        let (Some(all_a), Some(all_b)) = (shared_a.get(id), shared_b.get(id)) else {
                return Err(Error::String("Either name_a or name_b does not have a required party".to_string()))
        };
        let mut c = HashMap::new();
        for (subset, a) in all_a {
            let Some(b) = all_b.get(subset) else {
                return Err(Error::String("Either name_a or name_b does not have a required subset share".to_string()))
            };
            c.insert(subset.clone(), Arithmetic::add(*a, *b));
        }
        Ok(c)
    }

    pub fn add(&mut self, name_a: String, name_b: String, out_name: String) -> Result<(), Error> {
        let (Some(shared_a), Some(shared_b)) = (self.shared_vars.get(&name_a), self.shared_vars.get(&name_b)) else {
            return Err(Error::String(format!("Either name_a: {name_a} or name_b: {name_b} is not existent")))
        };
        if shared_a.len() != shared_b.len() {
            return Err(Error::String(format!("Lengths of name_a: {:?} and name_b: {:?} do not match", name_a.len(), name_b.len())))
        };
        let mut shared_c = HashMap::new();
        // Every party locally adds their shares of a and b
        for id in shared_a.keys() {
            let c = self.party_add(id, shared_a, shared_b)?;
            shared_c.insert(*id, c);
        }
        self.add_var(out_name, shared_c)?;
        Ok(())
    }

    
    pub fn mult(&mut self, name_a: String, name_b: String, out_name: String) -> Result<(), Error> {
        
        Ok(())
    }
}