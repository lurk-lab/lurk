mod air;
pub mod columns;
pub mod trace;

#[cfg(test)]
mod test {

    use core::array;
    use std::iter::zip;
    use std::marker::PhantomData;
    use std::mem::size_of;

    use crate::poseidon::wide::columns::Poseidon2Cols;
    use crate::{air::debug::debug_constraints_collecting_queries, poseidon::config::*};
    use hybrid_array::{typenum::Sub1, ArraySize};
    use p3_air::{Air, AirBuilder, BaseAir};
    use p3_field::AbstractField;
    use p3_matrix::dense::RowMajorMatrix;
    use p3_matrix::Matrix;
    use p3_symmetric::Permutation;

    struct Chip<C: PoseidonConfig<WIDTH>, const WIDTH: usize> {
        _marker: PhantomData<C>,
    }

    struct Cols<T, C: PoseidonConfig<WIDTH>, const WIDTH: usize>
    where
        Sub1<C::R_P>: ArraySize,
    {
        is_real: T,
        input: [T; WIDTH],
        poseidon: Poseidon2Cols<T, C, WIDTH>,
    }

    impl<C: PoseidonConfig<WIDTH>, const WIDTH: usize> BaseAir<C::F> for Chip<C, WIDTH>
    where
        Sub1<C::R_P>: ArraySize,
    {
        fn width(&self) -> usize {
            size_of::<Cols<u8, C, WIDTH>>()
        }
    }

    impl<AB: AirBuilder<F = C::F>, C: PoseidonConfig<WIDTH>, const WIDTH: usize> Air<AB>
        for Chip<C, WIDTH>
    where
        Sub1<C::R_P>: ArraySize,
    {
        fn eval(&self, builder: &mut AB) {
            let main = builder.main();
            let row = main.row_slice(0);
            let local = Cols::<AB::Var, C, WIDTH>::from_slice(&row);

            local
                .poseidon
                .eval(builder, local.input.map(Into::into), local.is_real.into());
        }
    }

    impl<T, C: PoseidonConfig<WIDTH>, const WIDTH: usize> Cols<T, C, WIDTH>
    where
        Sub1<C::R_P>: ArraySize,
    {
        #[inline]
        fn from_slice(slice: &[T]) -> &Self {
            let num_cols = size_of::<Cols<u8, C, WIDTH>>();
            assert_eq!(slice.len(), num_cols);
            let (_, shorts, _) = unsafe { slice.align_to::<Cols<T, C, WIDTH>>() };
            &shorts[0]
        }
        #[inline]
        fn from_slice_mut(slice: &mut [T]) -> &mut Self {
            let num_cols = size_of::<Cols<u8, C, WIDTH>>();
            assert_eq!(slice.len(), num_cols);
            let (_, shorts, _) = unsafe { slice.align_to_mut::<Cols<T, C, WIDTH>>() };
            &mut shorts[0]
        }
    }

    impl<C: PoseidonConfig<WIDTH>, const WIDTH: usize> Chip<C, WIDTH>
    where
        Sub1<C::R_P>: ArraySize,
    {
        fn generate_trace(
            &self,
            inputs: &[[C::F; WIDTH]],
        ) -> (Vec<[C::F; WIDTH]>, RowMajorMatrix<C::F>) {
            let width = self.width();
            let num_row = inputs.len();
            let height = num_row.next_power_of_two();
            let mut trace = RowMajorMatrix::new(vec![C::F::zero(); width * height], width);

            let outputs = zip(trace.rows_mut(), inputs)
                .map(|(row, &input)| {
                    let cols = Cols::<C::F, C, WIDTH>::from_slice_mut(row);
                    cols.is_real = C::F::one();
                    cols.input = input;
                    cols.poseidon.populate(input)
                })
                .collect();

            (outputs, trace)
        }
    }

    fn test_trace_eq_hash_with<const WIDTH: usize, C: PoseidonConfig<WIDTH>>()
    where
        Sub1<C::R_P>: ArraySize,
    {
        let chip = Chip::<C, WIDTH> {
            _marker: Default::default(),
        };
        let input: [C::F; WIDTH] = array::from_fn(C::F::from_canonical_usize);
        let hasher = C::hasher();

        let expected_output = hasher.permute(input);
        let (output, _trace) = chip.generate_trace(&[input]);

        assert_eq!(output[0], expected_output);
    }

    #[test]
    fn test_trace_eq_hash() {
        test_trace_eq_hash_with::<4, BabyBearConfig4>();
        test_trace_eq_hash_with::<8, BabyBearConfig8>();
        test_trace_eq_hash_with::<12, BabyBearConfig12>();
        test_trace_eq_hash_with::<16, BabyBearConfig16>();
        test_trace_eq_hash_with::<24, BabyBearConfig24>();
        test_trace_eq_hash_with::<40, BabyBearConfig40>();
    }

    fn test_air_constraints_with<const WIDTH: usize, C: PoseidonConfig<WIDTH>>()
    where
        Sub1<C::R_P>: ArraySize,
    {
        let chip = Chip::<C, WIDTH> {
            _marker: Default::default(),
        };
        let public_values = [C::F::zero(); WIDTH];
        let (_outputs, trace) = chip.generate_trace(&[public_values]);

        let _ = debug_constraints_collecting_queries(&chip, &public_values, &trace);
    }

    #[test]
    fn test_air_constraints() {
        test_air_constraints_with::<4, BabyBearConfig4>();
        test_air_constraints_with::<8, BabyBearConfig8>();
        test_air_constraints_with::<12, BabyBearConfig12>();
        test_air_constraints_with::<16, BabyBearConfig16>();
        test_air_constraints_with::<24, BabyBearConfig24>();
        test_air_constraints_with::<40, BabyBearConfig40>();
    }
}
