use std::mem::size_of;

use crate::poseidon::config::PoseidonConfig;

use hybrid_array::{typenum::*, Array, ArraySize};
use sp1_derive::AlignedBorrow;

/// Columns for the "narrow" Poseidon2 chip.
///
/// As an optimization, we can represent all of the internal rounds without columns for intermediate
/// states except for the 0th element. This is because the linear layer that comes after the sbox is
/// degree 1, so all state elements at the end can be expressed as a degree-3 polynomial of:
/// 1) the 0th state element at rounds prior to the current round
/// 2) the rest of the state elements at the beginning of the internal rounds
#[derive(Clone, Debug, AlignedBorrow)]
#[repr(C)]
pub struct Poseidon2Cols<T, C: PoseidonConfig<WIDTH>, const WIDTH: usize>
where
    Sub1<C::R_P>: ArraySize,
{
    /// Input states for all external rounds
    pub(crate) external_rounds_state: Array<[T; WIDTH], C::R_F>,
    /// Intermediary SBox results for external rounds
    pub(crate) external_rounds_sbox: Array<[T; WIDTH], C::R_F>,

    /// Initial state before internal rounds
    pub(crate) internal_rounds_state_init: [T; WIDTH],
    /// Expected value of the first state element in all but the first internal rounds
    pub(crate) internal_rounds_state0: Array<T, Sub1<C::R_P>>,
    /// Intermediary SBox results for internal rounds
    pub(crate) internal_rounds_sbox: Array<T, C::R_P>,
}

impl<T, C: PoseidonConfig<WIDTH>, const WIDTH: usize> Poseidon2Cols<T, C, WIDTH>
where
    Sub1<C::R_P>: ArraySize,
{
    pub const fn num_cols() -> usize {
        size_of::<Poseidon2Cols<u8, C, WIDTH>>()
    }
}
