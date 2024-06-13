// fn populate_internal_rounds(
//     poseidon2_cols: &mut Poseidon2WideCols<F>,
//     sbox: &mut Option<&mut [F; NUM_INTERNAL_ROUNDS]>,
// ) -> [F; WIDTH] {
//     let mut state: [F; WIDTH] = poseidon2_cols.internal_rounds_state;
//     let mut sbox_deg_3: [F; NUM_INTERNAL_ROUNDS] = [F::zero(); NUM_INTERNAL_ROUNDS];
//     for r in 0..NUM_INTERNAL_ROUNDS {
//         // Add the round constant to the 0th state element.
//         // Optimization: Since adding a constant is a degree 1 operation, we can avoid adding
//         // columns for it, just like for external rounds.
//         let round = r + NUM_EXTERNAL_ROUNDS / 2;
//         let add_rc = state[0] + (*PART_RC_16_13)[round];
//
//         // Apply the sboxes.
//         // Optimization: since the linear layer that comes after the sbox is degree 1, we can
//         // avoid adding columns for the result of the sbox, just like for external rounds.
//         sbox_deg_3[r] = add_rc * add_rc * add_rc;
//         let sbox_deg_7 = sbox_deg_3[r] * sbox_deg_3[r] * add_rc;
//
//         // Apply the linear layer.
//         state[0] = sbox_deg_7;
//         matmul_internal(&mut state, *MATRIX_DIAG_16_BABYBEAR);
//
//         // Optimization: since we're only applying the sbox to the 0th state element, we only
//         // need to have columns for the 0th state element at every step. This is because the
//         // linear layer is degree 1, so all state elements at the end can be expressed as a
//         // degree-3 polynomial of the state at the beginning of the internal rounds and the 0th
//         // state element at rounds prior to the current round
//         if r < NUM_INTERNAL_ROUNDS - 1 {
//             poseidon2_cols.internal_rounds_s0[r] = state[0];
//         }
//     }
//
//     let ret_state = state;
//
//     if let Some(sbox) = sbox.as_deref_mut() {
//         *sbox = sbox_deg_3;
//     }
//
//     ret_state
// }

use std::borrow::BorrowMut;
use std::ops::Sub;

use super::columns::Poseidon2WideCols;
use crate::poseidon::config::PoseidonConfig;

use hybrid_array::{typenum::B1, Array, ArraySize};
use p3_field::AbstractField;
use p3_symmetric::Permutation;

impl<const WIDTH: usize, C: PoseidonConfig<WIDTH>> Poseidon2WideCols<C::F, C, WIDTH>
where
    C::R_P: Sub<B1>,
    <<C as PoseidonConfig<WIDTH>>::R_P as Sub<B1>>::Output: ArraySize,
{
    pub fn populate_columns(&mut self, input: [C::F; WIDTH]) -> [C::F; WIDTH] {
        let mut state = C::external_linear_layer().permute(input);

        for r in 0..C::r_f() / 2 {
            state = self.populate_external_round(state, r)
        }

        state = self.populate_internal_rounds(state);

        for r in C::r_f() / 2..C::r_f() {
            state = self.populate_external_round(state, r)
        }

        state
    }

    fn populate_external_round(&mut self, input: [C::F; WIDTH], round: usize) -> [C::F; WIDTH] {
        let mut state = {
            let round_state: &mut [C::F; WIDTH] = self.external_rounds_state[round].borrow_mut();

            // Add round constants.
            //
            // Optimization: Since adding a constant is a degree 1 operation, we can avoid adding
            // columns for it, and instead include it in the constraint for the x^3 part of the sbox.
            let round = if round < C::r_f() / 2 {
                round
            } else {
                round + C::r_p()
            };
            let mut add_rc = *round_state;
            for i in 0..WIDTH {
                add_rc[i] += (*C::external_constants())[round][i];
            }

            // Apply the sboxes.
            // Optimization: since the linear layer that comes after the sbox is degree 1, we can
            // avoid adding columns for the result of the sbox, and instead include the x^3 -> x^7
            // part of the sbox in the constraint for the linear layer
            let mut sbox_deg_7: [C::F; WIDTH] = [C::F::zero(); WIDTH];
            let mut sbox_deg_3: [C::F; WIDTH] = [C::F::zero(); WIDTH];
            for i in 0..WIDTH {
                sbox_deg_3[i] = add_rc[i] * add_rc[i] * add_rc[i];
                sbox_deg_7[i] = sbox_deg_3[i] * sbox_deg_3[i] * add_rc[i];
            }

            self.external_rounds_sbox[round] = sbox_deg_3;

            sbox_deg_7
        };

        // Apply the linear layer.
        C::external_linear_layer().permute_mut(&mut state);
        self.external_rounds_state[round] = state;

        state
    }

    fn populate_internal_rounds(&mut self, input: [C::F; WIDTH]) -> [C::F; WIDTH] {
        let mut state: [C::F; WIDTH] = input;
        let mut sbox_deg_3: Array<C::F, C::R_P> = Array::from_fn(|_| C::F::zero());
        for r in 0..C::r_p() {
            // Add the round constant to the 0th state element.
            // Optimization: Since adding a constant is a degree 1 operation, we can avoid adding
            // columns for it, just like for external rounds.
            let round = r + C::r_f() / 2;
            let add_rc = state[0] + (C::internal_constants())[round];

            // Apply the sboxes.
            // Optimization: since the linear layer that comes after the sbox is degree 1, we can
            // avoid adding columns for the result of the sbox, just like for external rounds.
            sbox_deg_3[r] = add_rc.cube();
            let sbox_deg_7 = sbox_deg_3[r].square() * add_rc;

            // Apply the linear layer.
            state[0] = sbox_deg_7;
            C::internal_linear_layer().permute_mut(&mut state);

            // Optimization: since we're only applying the sbox to the 0th state element, we only
            // need to have columns for the 0th state element at every step. This is because the
            // linear layer is degree 1, so all state elements at the end can be expressed as a
            // degree-3 polynomial of the state at the beginning of the internal rounds and the 0th
            // state element at rounds prior to the current round
            if r < C::r_p() - 1 {
                self.internal_rounds_s0[r] = state[0];
            }
        }

        self.internal_rounds_sbox = sbox_deg_3;

        state
    }
}