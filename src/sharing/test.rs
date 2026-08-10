
use std::iter::zip;

use crate::sharing::bitadder::{full, full_adder, parallel_prefix};

use super::*;
use rand::rngs::SysRng;

const TIMES: usize = 20;

#[test]
fn sharing_correctness_test() -> Result<(), Error>{
    let k = 3;
    let n = 5;
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

#[test]
fn to_from_arith_binary_test() -> Result<(), Error> {
    let k = 2;
    let n = 3;
    let arithmetic = ArithmeticSharing::new(k, n);
    let mut rng = SysRng;

    let secret_arith = Arithmetic::random_share(&mut rng)
        .expect("Could not generate random arithmetic share");
    let arith_shares = arithmetic.share(&mut rng, secret_arith)?;

    let bits_from_arith: HashMap<Vec<usize>, ArithShare> = arith_shares
        .values()
        .flat_map(|x| x.iter().map(|(k, v)| (k.clone(), *v)))
        .collect();
    let arith: Vec<ArithShare> = bits_from_arith.into_values().collect();
    let bits_from_arith: Vec<Vec<BitShare>> = arith
        .iter()
        .map(|x| Arithmetic::to_binary(*x))
        .collect();
    let bits_to_arith: Vec<ArithShare> = bits_from_arith
        .iter()
        .map(|x| Binary::to_arithmetic(x.to_vec()))
        .collect();
    assert!(arith == bits_to_arith);
    Ok(())
}

#[test]
fn bit_adder_test() -> Result<(), Error> {
    let mut rng = SysRng;

    let a = Arithmetic::random_share(&mut rng)
        .expect("Could not generate random arithmetic share");
    let b = Arithmetic::random_share(&mut rng)
        .expect("Could not generate random arithmetic share");
    let c_out = u32::from(a) + u32::from(b) > u32::from(u16::MAX);
    let c = Arithmetic::add(a, b);

    let a = Arithmetic::to_binary(a);
    let b = Arithmetic::to_binary(b);
    let c = Arithmetic::to_binary(c);

    let mut c_in = false;
    let mut s = Vec::new();
    for (a_bit, b_bit) in zip(&a, &b) {
        let (s_bit, c_bit) = full(a_bit, b_bit, &c_in);
        s.push(s_bit);
        c_in = c_bit;
    }

    assert!(c == s && c_in == c_out);
    let c_in = full_adder(&a, &b);
    assert!(c_in == c_out);

    let x = parallel_prefix(&a, &b);
    assert!(c_out == x);
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

