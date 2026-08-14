use std::iter::zip;

use crate::{error::Error};

pub trait BitOps {
    type Bit: Clone;

    fn xor(&mut self, a: &Self::Bit, b: &Self::Bit) -> Result<Self::Bit, Error>;
    fn and(&mut self, a: &Self::Bit, b: &Self::Bit) -> Result<Self::Bit, Error>;
    fn zero(&mut self) -> Result<Self::Bit, Error>;
}

pub type BitAdder<T> = fn(ops: &mut T, a: &Vec<<T as BitOps>::Bit>, b: &Vec<<T as BitOps>::Bit>) -> Result<Vec<<T as BitOps>::Bit>, Error>;

pub fn full<T: BitOps>(ops: &mut T, a: &T::Bit, b: &T::Bit, c_in: &T::Bit) -> Result<(T::Bit, T::Bit), Error> {
    let t = ops.xor(a, b)?;
    let s = ops.xor(&t, c_in)?;
    // a ^ (t & (c_in ^ a))
    let c_in_xor_a = ops.xor(c_in, a)?;
    let t_and = ops.and(&t, &c_in_xor_a)?;
    let c_out = ops.xor(a, &t_and)?;
    Ok((s, c_out))
}

/// Returns the sum of `a` and `b` via full adder
pub fn full_adder<T: BitOps>(ops: &mut T, a: &Vec<T::Bit>, b: &Vec<T::Bit>) -> Result<Vec<T::Bit>, Error> {
    assert!(a.len() == b.len());
    let mut sum = Vec::new();
    let mut c = ops.zero()?;
    for (bit_a, bit_b) in zip(&a[..a.len()], &b[..b.len()]) {
        let (s, c_out) = full(ops, bit_a, bit_b, &c)?;
        sum.push(s);
        c = c_out
    }
    Ok(sum)
}

struct PRNode<T: BitOps> {
    value: (T::Bit, T::Bit),
    left: Option<Box<PRNode<T>>>,
    right: Option<Box<PRNode<T>>>
}

impl<T: BitOps> PRNode<T> {
    pub fn new(g: &T::Bit, p: &T::Bit) -> Self {
        PRNode { value: (g.clone(), p.clone()), left: None, right: None }
    }

    /// In-order traversal, pass left (LSB...) carries into right (...MSB) subtrees
    /// 
    /// Returns `Vec<T::Bit>` through `sum`
    pub fn sum(&mut self, ops: &mut T, c_in: &T::Bit, sum: &mut Vec<T::Bit>) -> Result<(), Error> {
        if self.left.is_none() && self.right.is_none() {
            let (_, p) = &self.value;
            let s = ops.xor(&p, c_in)?;
            sum.push(s);
            return Ok(());
        }
        let left = self.left.as_mut().ok_or(Error::String("PRNode recurse error".to_string()))?;
        let right = self.right.as_mut().ok_or(Error::String("PRNode recurse error".to_string()))?;
        left.sum(ops, c_in, sum)?;
        let (g, p) = left.value.clone();
        let p_and = ops.and(&p, c_in)?;
        let c_out = ops.xor(&g, &p_and)?;
        right.sum(ops, &c_out, sum)?;
        Ok(())
    }
}

/// Returns (g: generation, p: propagation)
/// 
/// Generation = at this index, a carry is guaranteed
/// Propagation = at this index, a carry is guaranteed if and only if a prior carry is passed through
fn parallel_recurse<T: BitOps>(ops: &mut T, a: &Vec<T::Bit>, b: &Vec<T::Bit>, l: usize, r:usize, node: &mut PRNode<T>) -> Result<(T::Bit, T::Bit), Error> {
    if r - l == 1 {
        let g = ops.and(&a[l], &b[l])?;
        let p = ops.xor(&a[l], &b[l])?;
        node.value = (g.clone(), p.clone());
        node.left = None;
        node.right = None;
        return Ok((g, p))
    }
    let zero = ops.zero()?;
    node.left = Some(Box::new(PRNode::new(&zero ,&zero)));
    node.right = Some(Box::new(PRNode::new(&zero ,&zero)));
    let mid = l + (r - l) / 2;
    let left = node.left.as_mut().ok_or(Error::String("Parallel recurse error".to_string()))?;
    let right = node.right.as_mut().ok_or(Error::String("Parallel recurse error".to_string()))?;
    let (Ok((gl, pl)), Ok((gr, pr))) = (parallel_recurse(ops, a, b, l, mid, left), parallel_recurse(ops, a, b, mid, r, right)) else {
        return Err(Error::String("Error within parallel recurse".to_string()))
    };
    let p = ops.and(&pl, &pr)?;
    // Since pr and gr can never both be true, the 'or' can be replaced with 'xor'
    let pg = ops.and(&pr, &gl)?;
    let g = ops.xor(&gr, &pg)?;
    node.value = (g.clone(), p.clone());
    Ok((g, p))
}

/// Returns the sum of `a` and `b` via parallel-prefix adder
pub fn parallel_prefix<T: BitOps>(ops: &mut T, a: &Vec<T::Bit>, b: &Vec<T::Bit>) -> Result<Vec<T::Bit>, Error> {
    assert!(a.len() == b.len());
    let zero = ops.zero()?;
    let mut root = PRNode::new(&zero ,&zero);
    parallel_recurse(ops, a, b, 0, a.len(), &mut root)?;
    let mut sum = Vec::new();
    root.sum(ops, &zero, &mut sum)?;
    Ok(sum)
}
