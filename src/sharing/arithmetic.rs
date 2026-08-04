use super::*;

pub type ArithShare = u16;
pub struct Arithmetic;

impl Arithmetic {
    /// Returns [`BitShare`] vector of form: <LSB...MSB>
    pub fn to_binary(a: ArithShare) -> Vec<BitShare> {
        let mut bits: Vec<BitShare> = Vec::new();
        for i in 0..ArithShare::BITS {
            bits.push((1 & a >> i) == 1);
        }
        bits
    }
}

impl Sharing for Arithmetic {
    type Share = ArithShare;

    fn zero() -> Self::Share {
        Self::Share::default()
    }
    fn random_share<T: TryCryptoRng>(rng: &mut T) -> Result<Self::Share, T::Error> {
        rng.try_next_u32().map(|val| val as Self::Share)
    }
    fn add(a: Self::Share, b: Self::Share) -> Self::Share {
        a.wrapping_add(b)
    }
    fn sub(a: Self::Share, b: Self::Share) -> Self::Share {
        a.wrapping_sub(b)
    }
}