//! This module defines the Poseidon2 configurations and implements the traits for all the supported
//! widths between 4 and 40.

use std::slice;

use hybrid_array::{typenum::*, Array, ArraySize};
use p3_baby_bear::BabyBear;
use p3_field::{AbstractField, Field};
use p3_poseidon2::{DiffusionPermutation, Poseidon2, Poseidon2ExternalMatrixGeneral};
use p3_symmetric::Permutation;

use super::{constants::*, Poseidon2Chip};

/// A sealed trait that provides the constants required for the Poseidon configuration
trait ConstantsProvided {}

/// The Poseidon configuration trait storing the data needed for
#[allow(private_bounds)]
pub trait PoseidonConfig: Clone + Copy + Sync + ConstantsProvided {
    type F: Field;
    type WIDTH: ArraySize;
    const WIDTH: usize;
    const R_P: usize;
    const R_F: usize;
    type R: ArraySize;

    /// Returns the diagonal matrix for the internal Poseidon permutation
    fn matrix_diag() -> &'static Array<Self::F, Self::WIDTH>;

    /// Returns an iterator of the hasher's round constants
    fn round_constants_iter() -> impl IntoIterator<Item = &'static [Self::F]>;
}

/// The internal diffusion layer for the Poseidon chip, implements the `Permutation` and
/// `DiffusionPermutation` traits needed to compute the Poseidon permutation.
#[derive(Clone)]
pub struct InternalDiffusion {}

macro_rules! impl_poseidon_config {
    ($name:ident, $field:ident, $width_t:ident, $width:literal, $r_p:literal, $r_f:literal, $r_t:ident, $full_rc:ident, $part_rc:ident, $diag:ident) => {
        #[derive(Clone, Copy)]
        pub struct $name;

        impl ConstantsProvided for $name {}

        impl PoseidonConfig for $name {
            type F = $field;
            type WIDTH = $width_t;
            const WIDTH: usize = $width;
            const R_P: usize = $r_p;
            const R_F: usize = $r_f;
            type R = $r_t;

            fn matrix_diag() -> &'static Array<Self::F, Self::WIDTH> {
                Array::from_slice(&*$diag)
            }

            fn round_constants_iter() -> impl IntoIterator<Item = &'static [Self::F]> {
                let first_half = $full_rc.iter().map(|c| c.as_slice()).take(Self::R_F / 2);
                let second_half = $full_rc.iter().map(|c| c.as_slice()).skip(Self::R_F / 2);

                let partial_round_constants = $part_rc.iter().map(slice::from_ref);

                first_half.chain(partial_round_constants).chain(second_half)
            }
        }

        impl Permutation<[$field; $width]> for InternalDiffusion {
            fn permute_mut(&self, input: &mut [$field; $width]) {
                let sum: $field = input.iter().copied().sum();
                for i in 0..$width {
                    input[i] = sum + ($diag[i] + $field::neg_one()) * input[i];
                }
            }
        }

        impl DiffusionPermutation<$field, $width> for InternalDiffusion {}

        impl Poseidon2Chip<$name> {
            /// Returns a Poseidon 2 hasher
            pub fn hasher(
                &self,
            ) -> Poseidon2<$field, Poseidon2ExternalMatrixGeneral, InternalDiffusion, $width, 7>
            {
                let rounds_f = $name::R_F;
                let rounds_p = $name::R_P;

                let external_constants = $full_rc.to_vec();
                let internal_constants = $part_rc.to_vec();

                let external_linear_layer = Poseidon2ExternalMatrixGeneral {};
                let internal_linear_layer = InternalDiffusion {};

                Poseidon2::new(
                    rounds_f,
                    external_constants,
                    external_linear_layer,
                    rounds_p,
                    internal_constants,
                    internal_linear_layer,
                )
            }
        }
    };
}

impl_poseidon_config!(
    BabyBearConfig4,
    BabyBear,
    U4,
    4,
    21,
    8,
    U29,
    FULL_RC_4_8,
    PART_RC_4_21,
    MATRIX_DIAG_4_BABYBEAR
);

impl_poseidon_config!(
    BabyBearConfig8,
    BabyBear,
    U8,
    8,
    12,
    8,
    U20,
    FULL_RC_8_8,
    PART_RC_8_12,
    MATRIX_DIAG_8_BABYBEAR
);

impl_poseidon_config!(
    BabyBearConfig12,
    BabyBear,
    U12,
    12,
    10,
    8,
    U18,
    FULL_RC_12_8,
    PART_RC_12_10,
    MATRIX_DIAG_12_BABYBEAR
);

impl_poseidon_config!(
    BabyBearConfig16,
    BabyBear,
    U16,
    16,
    13,
    8,
    U21,
    FULL_RC_16_8,
    PART_RC_16_13,
    MATRIX_DIAG_16_BABYBEAR
);

impl_poseidon_config!(
    BabyBearConfig20,
    BabyBear,
    U20,
    20,
    18,
    8,
    U26,
    FULL_RC_20_8,
    PART_RC_20_18,
    MATRIX_DIAG_20_BABYBEAR
);

impl_poseidon_config!(
    BabyBearConfig24,
    BabyBear,
    U24,
    24,
    21,
    8,
    U29,
    FULL_RC_24_8,
    PART_RC_24_21,
    MATRIX_DIAG_24_BABYBEAR
);

impl_poseidon_config!(
    BabyBearConfig28,
    BabyBear,
    U28,
    28,
    25,
    8,
    U33,
    FULL_RC_28_8,
    PART_RC_28_25,
    MATRIX_DIAG_28_BABYBEAR
);

impl_poseidon_config!(
    BabyBearConfig32,
    BabyBear,
    U32,
    32,
    30,
    8,
    U38,
    FULL_RC_32_8,
    PART_RC_32_30,
    MATRIX_DIAG_32_BABYBEAR
);

impl_poseidon_config!(
    BabyBearConfig36,
    BabyBear,
    U36,
    36,
    34,
    8,
    U42,
    FULL_RC_36_8,
    PART_RC_36_34,
    MATRIX_DIAG_36_BABYBEAR
);

impl_poseidon_config!(
    BabyBearConfig40,
    BabyBear,
    U40,
    40,
    38,
    8,
    U46,
    FULL_RC_40_8,
    PART_RC_40_38,
    MATRIX_DIAG_40_BABYBEAR
);