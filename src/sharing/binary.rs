use super::*;

pub type BitShare = bool;
pub struct Binary;

impl Binary {
    /// Given [`BitShare`] vector of form <LSB...MSB>, return an [`ArithShare`]
    /// 
    /// Panics if length of bits given does not equal the size of an [`ArithShare`]
    pub fn to_arithmetic(bits: Vec<BitShare>) -> ArithShare {
        assert!(bits.len() == ArithShare::BITS as usize);
        let mut arith_share: ArithShare = ArithShare::default();
        for (i, bit) in bits.iter().enumerate() {
            if *bit {
                arith_share |= 1 << i
            }
        }
        arith_share
    }
}

impl Sharing for Binary {
    type Share = BitShare;

    fn zero() -> Self::Share {
        Self::Share::default()
    }
    fn random_share<T: TryCryptoRng>(rng: &mut T) -> Result<Self::Share, T::Error> {
        Ok(rng.try_next_u32()? & 1 == 1)
    }
    fn add(a: Self::Share, b: Self::Share) -> Self::Share {
        a ^ b
    }
    fn sub(a: Self::Share, b: Self::Share) -> Self::Share {
        a ^ b
    }
}