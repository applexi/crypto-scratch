use std::iter::zip;

use crate::BitShare;

pub fn full(a: &BitShare, b: &BitShare, c_in: &BitShare) -> (BitShare, BitShare) {
    let t = a ^ b;
    let s = t ^ c_in;
    let c_out = a ^ (t & (c_in ^ a));
    (s, c_out)
}

pub fn full_adder(a: &Vec<BitShare>, b: &Vec<BitShare>) -> bool {
    assert!(a.len() == b.len());
    let mut c = false;
    for (bit_a, bit_b) in zip(a, b) {
        (_, c) = full(bit_a, bit_b, &c);
    }
    c
}

/// returns (g: generation, p: propagation)
fn parallel_recurse(a: &Vec<BitShare>, b: &Vec<BitShare>, l: usize, r:usize) -> (bool, bool) {
    if r - l == 1 {
        return (a[l] & b[l], a[l] ^ b[l])
    }
    let mid = l + (r - l) / 2;
    let ((gl, pl), (gr, pr)) = (parallel_recurse(a, b, l, mid), parallel_recurse(a, b, mid, r));
    let p = pl & pr;
    let g = gr | (pr & gl);
    (g, p)
}

pub fn parallel_prefix(a: &Vec<BitShare>, b: &Vec<BitShare>) -> bool {
    assert!(a.len() == b.len());
    let (g, _) = parallel_recurse(a, b, 0, a.len());
    g
}

