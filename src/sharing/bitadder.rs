use std::iter::zip;

use crate::BitShare;

pub fn full(a: &BitShare, b: &BitShare, c_in: &BitShare) -> (BitShare, BitShare) {
    let t = a ^ b;
    let s = t ^ c_in;
    let c_out = a ^ (t & (c_in ^ a));
    (s, c_out)
}

/// Returns the final carry bit of the sum of `a` and `b` via the full adder
pub fn full_adder(a: &Vec<BitShare>, b: &Vec<BitShare>) -> bool {
    assert!(a.len() == b.len());
    let mut c = false;
    for (bit_a, bit_b) in zip(a, b) {
        (_, c) = full(bit_a, bit_b, &c);
    }
    c
}

/// Returns (g: generation, p: propagation)
/// 
/// Generation = at this index, a carry is guaranteed
/// Propagation = at this index, a carry is guaranteed if and only if a prior carry is passed through
fn parallel_recurse(a: &Vec<BitShare>, b: &Vec<BitShare>, l: usize, r:usize) -> (bool, bool) {
    if r - l == 1 {
        return (a[l] & b[l], a[l] ^ b[l])
    }
    let mid = l + (r - l) / 2;
    let ((gl, pl), (gr, pr)) = (parallel_recurse(a, b, l, mid), parallel_recurse(a, b, mid, r));
    let p = pl & pr;
    // Since pr and gr can never both be true, the 'or' can be replaced with 'xor'
    let g = gr ^ (pr & gl);
    (g, p)
}

/// Returns the final carry bit of the sum of `a` and `b` via the parallel prefix adder
pub fn parallel_prefix(a: &Vec<BitShare>, b: &Vec<BitShare>) -> bool {
    assert!(a.len() == b.len());
    let (g, _) = parallel_recurse(a, b, 0, a.len());
    g
}

