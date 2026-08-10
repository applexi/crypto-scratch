pub mod error;
pub mod mpc;
pub mod sharing;
pub mod prng;

pub const DEFAULT_N: usize = 3;
pub const DEFAULT_K: usize = 2;

pub use sharing::{
    ArithShare, ArithmeticShares, ArithmeticSharing, BinarySharing, BitShare, BinaryShares, PartyID,
};
