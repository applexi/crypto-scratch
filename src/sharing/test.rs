use crate::sharing::bitadder::{full_adder, parallel_prefix};

use super::*;
use rand::rngs::SysRng;

const TIMES: usize = 20;

#[test]
fn sharing_correctness_test() -> Result<(), Error> {
    let k = 3;
    let n = 5;
    let mut rng = SysRng;

    let secret_arith =
        Arithmetic::random_share(&mut rng).expect("Could not generate random arithmetic share");
    let arith_shares = ArithRepSharing::share(&mut rng, secret_arith, k, n)?;
    let secret_bit =
        Binary::random_share(&mut rng).expect("Could not generate random binary share");
    let binary_shares = BitRepSharing::share(&mut rng, secret_bit, k, n)?;

    let k_subsets: Vec<Vec<usize>> = (1..n + 1).combinations(k).collect();
    for k_subset in k_subsets {
        let k_arith_shares: HashMap<usize, HashMap<Vec<usize>, ArithShare>> = arith_shares
            .iter()
            .filter_map(|(i, x)| k_subset.contains(i).then(|| (*i, x.clone())))
            .collect();
        let k_binary_shares: HashMap<usize, HashMap<Vec<usize>, BitShare>> = binary_shares
            .iter()
            .filter_map(|(i, x)| k_subset.contains(i).then(|| (*i, x.clone())))
            .collect();
        let r_secret_arith = ArithRepSharing::reconstruct(&k_arith_shares, k, n);
        let r_secret_binary = BitRepSharing::reconstruct(&k_binary_shares, k, n);
        assert!(secret_arith == r_secret_arith);
        assert!(secret_bit == r_secret_binary);
    }
    Ok(())
}

#[test]
fn to_from_arith_binary_test() -> Result<(), Error> {
    let k = 2;
    let n = 3;
    let mut rng = SysRng;

    let secret_arith =
        Arithmetic::random_share(&mut rng).expect("Could not generate random arithmetic share");
    let arith_shares = ArithRepSharing::share(&mut rng, secret_arith, k, n)?;

    let bits_from_arith: HashMap<Vec<usize>, ArithShare> = arith_shares
        .values()
        .flat_map(|x| x.iter().map(|(k, v)| (k.clone(), *v)))
        .collect();
    let arith: Vec<ArithShare> = bits_from_arith.into_values().collect();
    let bits_from_arith: Vec<Vec<BitShare>> =
        arith.iter().map(|x| Arithmetic::to_binary(*x)).collect();
    let bits_to_arith: Vec<ArithShare> = bits_from_arith
        .iter()
        .map(|x| Binary::to_arithmetic(x.to_vec()))
        .collect();
    assert!(arith == bits_to_arith);
    Ok(())
}

pub struct NonMPC;

impl BitOps for NonMPC {
    type Bit = bool;
    fn and(&mut self, a: &Self::Bit, b: &Self::Bit) -> Result<Self::Bit, Error> {
        Ok(a & b)
    }
    fn xor(&mut self, a: &Self::Bit, b: &Self::Bit) -> Result<Self::Bit, Error> {
        Ok(a ^ b)
    }
    fn zero(&mut self) -> Result<Self::Bit, Error> {
        Ok(false)
    }
}

#[test]
fn bit_adder_test() -> Result<(), Error> {
    let mut rng = SysRng;

    let a = Arithmetic::random_share(&mut rng).expect("Could not generate random arithmetic share");
    let b = Arithmetic::random_share(&mut rng).expect("Could not generate random arithmetic share");
    let c = Arithmetic::add(a, b);

    let a = Arithmetic::to_binary(a);
    let b = Arithmetic::to_binary(b);
    let c = Arithmetic::to_binary(c);

    let mut non_mpc = NonMPC;
    let added_c = full_adder(&mut non_mpc, &a, &b)?;
    assert!(c == added_c);

    let added_c = parallel_prefix(&mut non_mpc, &a, &b)?;
    assert!(c == added_c);
    Ok(())
}

#[test]
fn loop_all_test() -> Result<(), Error> {
    for _ in 0..TIMES {
        sharing_correctness_test()?;
        to_from_arith_binary_test()?;
        bit_adder_test()?;
    }
    Ok(())
}
