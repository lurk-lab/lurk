use p3_air::BaseAir;
use p3_field::Field;
use p3_matrix::dense::RowMajorMatrix;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::{
    bytecode::{Block, Ctrl, Func, Op},
    execute::QueryRecord,
    toplevel::Toplevel,
    List,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct ColumnLayout<T> {
    pub(crate) input: T,
    pub(crate) output: T,
    pub(crate) aux: T,
    pub(crate) sel: T,
}
pub type Width = ColumnLayout<usize>;
pub type ColumnIndex = ColumnLayout<usize>;
pub type ColumnSlice<'a, T> = ColumnLayout<&'a [T]>;
pub type ColumnMutSlice<'a, T> = ColumnLayout<&'a mut [T]>;

impl Width {
    #[inline]
    fn len(&self) -> usize {
        self.input + self.output + self.aux + self.sel
    }
}

impl<'a, T> ColumnSlice<'a, T> {
    pub fn from_slice(slice: &'a [T], width: Width) -> Self {
        let (input, slice) = slice.split_at(width.input);
        let (output, slice) = slice.split_at(width.output);
        let (aux, slice) = slice.split_at(width.aux);
        let (sel, slice) = slice.split_at(width.sel);
        assert!(slice.is_empty());
        Self {
            input,
            output,
            aux,
            sel,
        }
    }

    pub fn next_input(&self, index: &mut ColumnIndex) -> &T {
        let t = &self.input[index.input];
        index.input += 1;
        t
    }

    pub fn next_output(&self, index: &mut ColumnIndex) -> &T {
        let t = &self.output[index.output];
        index.output += 1;
        t
    }

    pub fn next_aux(&self, index: &mut ColumnIndex) -> &T {
        let t = &self.aux[index.aux];
        index.aux += 1;
        t
    }
}

impl<'a, T> ColumnMutSlice<'a, T> {
    pub fn from_slice(slice: &'a mut [T], width: Width) -> Self {
        let (input, slice) = slice.split_at_mut(width.input);
        let (output, slice) = slice.split_at_mut(width.output);
        let (aux, slice) = slice.split_at_mut(width.aux);
        let (sel, slice) = slice.split_at_mut(width.sel);
        assert!(slice.is_empty());
        Self {
            input,
            output,
            aux,
            sel,
        }
    }

    pub fn push_input(&mut self, index: &mut ColumnIndex, t: T) {
        self.input[index.input] = t;
        index.input += 1;
    }

    pub fn push_output(&mut self, index: &mut ColumnIndex, t: T) {
        self.output[index.output] = t;
        index.output += 1;
    }

    pub fn push_aux(&mut self, index: &mut ColumnIndex, t: T) {
        self.aux[index.aux] = t;
        index.aux += 1;
    }
}

pub struct FuncChip<'a, F> {
    pub(crate) func: &'a Func<F>,
    pub(crate) toplevel: &'a Toplevel<F>,
    pub(crate) width: Width,
}

impl<'a, F> FuncChip<'a, F> {
    pub fn from_name(name: &'static str, toplevel: &'a Toplevel<F>) -> Self {
        let func = toplevel.get_by_name(name).unwrap();
        let width = func.compute_width(toplevel);
        Self {
            func,
            toplevel,
            width,
        }
    }

    pub fn from_index(idx: usize, toplevel: &'a Toplevel<F>) -> Self {
        let func = toplevel.get_by_index(idx).unwrap();
        let width = func.compute_width(toplevel);
        Self {
            func,
            toplevel,
            width,
        }
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width.len()
    }

    #[inline]
    pub fn func(&self) -> &Func<F> {
        self.func
    }

    #[inline]
    pub fn toplevel(&self) -> &Toplevel<F> {
        self.toplevel
    }
}

impl<'a, F: Sync> BaseAir<F> for FuncChip<'a, F> {
    fn width(&self) -> usize {
        self.width()
    }
}

impl<'a, F: Field + Ord> FuncChip<'a, F> {
    pub fn generate_trace(&self, queries: &QueryRecord<F>) -> RowMajorMatrix<F> {
        let query_map = &queries.record()[self.func.index];
        let width = self.width();
        let mut values: Vec<F> = query_map
            .0
            .par_iter()
            .flat_map(|(args, res)| {
                if res.mult != 0 {
                    let index = &mut ColumnIndex::default();
                    let mut row = vec![F::zero(); width];
                    let slice = &mut ColumnMutSlice::from_slice(&mut row, self.width);
                    slice.push_aux(index, F::from_canonical_u32(res.mult));
                    self.func.populate_row(args, index, slice, queries);
                    row
                } else {
                    vec![F::zero(); width]
                }
            })
            .collect();
        let target_height = query_map.size().next_power_of_two().max(4);
        values.resize(width * target_height, F::zero());
        RowMajorMatrix::new(values, self.width())
    }
}

type Degree = u8;

impl<F> Func<F> {
    pub fn compute_width(&self, toplevel: &Toplevel<F>) -> Width {
        let input = self.input_size;
        let output = self.output_size;
        // first auxiliary is multiplicity
        let mut aux = 1;
        let mut sel = 0;
        let degrees = &mut vec![1; input];
        self.body
            .compute_width(degrees, toplevel, &mut aux, &mut sel);
        Width {
            input,
            output,
            aux,
            sel,
        }
    }
}

impl<F> Block<F> {
    fn compute_width(
        &self,
        degrees: &mut Vec<Degree>,
        toplevel: &Toplevel<F>,
        aux: &mut usize,
        sel: &mut usize,
    ) {
        self.ops
            .iter()
            .for_each(|op| op.compute_width(degrees, toplevel, aux));
        self.ctrl.compute_width(degrees, toplevel, aux, sel);
    }
}

impl<F> Ctrl<F> {
    fn compute_width(
        &self,
        degrees: &mut Vec<Degree>,
        toplevel: &Toplevel<F>,
        aux: &mut usize,
        sel: &mut usize,
    ) {
        match self {
            Ctrl::Return(..) => *sel += 1,
            Ctrl::Match(_, cases) => {
                let degrees_len = degrees.len();
                let mut max_aux = *aux;
                for (_, block) in cases.branches.iter() {
                    let block_aux = &mut aux.clone();
                    block.compute_width(degrees, toplevel, block_aux, sel);
                    degrees.truncate(degrees_len);
                    max_aux = max_aux.max(*block_aux);
                }
                if let Some(block) = &cases.default {
                    let block_aux = &mut aux.clone();
                    *block_aux += cases.branches.size();
                    block.compute_width(degrees, toplevel, block_aux, sel);
                    degrees.truncate(degrees_len);
                    max_aux = max_aux.max(*block_aux);
                }
                *aux = max_aux;
            }
            Ctrl::If(_, t, f) => {
                let degrees_len = degrees.len();
                let t_aux = &mut aux.clone();
                // for proof of inequality we need inversion
                *t_aux += 1;
                t.compute_width(degrees, toplevel, t_aux, sel);
                degrees.truncate(degrees_len);
                let f_aux = &mut aux.clone();
                f.compute_width(degrees, toplevel, f_aux, sel);
                degrees.truncate(degrees_len);
                *aux = (*t_aux).max(*f_aux);
            }
        }
    }
}

impl<F> Op<F> {
    fn compute_width(&self, degrees: &mut Vec<Degree>, toplevel: &Toplevel<F>, aux: &mut usize) {
        match self {
            Op::Const(..) => {
                degrees.push(0);
            }
            Op::Add(a, b) | Op::Sub(a, b) => {
                let deg = degrees[*a].max(degrees[*b]);
                degrees.push(deg);
            }
            Op::Mul(a, b) => {
                let deg = degrees[*a] + degrees[*b];
                // degree less than 2 does not need allocation
                if deg < 2 {
                    degrees.push(deg);
                } else {
                    degrees.push(1);
                    *aux += 1;
                }
            }
            Op::Inv(a) => {
                let a_deg = degrees[*a];
                if a_deg == 0 {
                    degrees.push(0);
                } else {
                    degrees.push(1);
                    *aux += 1;
                }
            }
            Op::Call(f_idx, ..) => {
                let func = toplevel.get_by_index(*f_idx as usize).unwrap();
                let out_size = func.output_size;
                *aux += out_size;
                degrees.extend(vec![1; out_size]);
            }
        }
    }
}

impl<F: Field + Ord> Func<F> {
    pub fn populate_row(
        &self,
        args: &[F],
        index: &mut ColumnIndex,
        slice: &mut ColumnMutSlice<'_, F>,
        queries: &QueryRecord<F>,
    ) {
        assert_eq!(self.input_size(), args.len(), "Argument mismatch");
        // Variable to value map
        let map = &mut args.iter().map(|arg| (*arg, 1)).collect();
        // One column per input
        args.iter().for_each(|arg| slice.push_input(index, *arg));
        self.body.populate_row(map, index, slice, queries);
    }
}

impl<F: Field + Ord> Block<F> {
    fn populate_row(
        &self,
        map: &mut Vec<(F, Degree)>,
        index: &mut ColumnIndex,
        slice: &mut ColumnMutSlice<'_, F>,
        queries: &QueryRecord<F>,
    ) {
        self.ops
            .iter()
            .for_each(|op| op.populate_row(map, index, slice, queries));
        self.ctrl.populate_row(map, index, slice, queries);
    }
}

impl<F: Field + Ord> Ctrl<F> {
    fn populate_row(
        &self,
        map: &mut Vec<(F, Degree)>,
        index: &mut ColumnIndex,
        slice: &mut ColumnMutSlice<'_, F>,
        queries: &QueryRecord<F>,
    ) {
        match self {
            Ctrl::Return(ident, out) => {
                slice.sel[*ident] = F::one();
                out.iter().for_each(|i| slice.push_output(index, map[*i].0))
            }
            Ctrl::Match(t, cases) => {
                let (t, _) = map[*t];
                if let Some(branch) = cases.branches.get(&t) {
                    branch.populate_row(map, index, slice, queries);
                } else {
                    let branch = cases.default.as_ref().expect("No match");
                    branch.populate_row(map, index, slice, queries);
                    for (f, _) in cases.branches.iter() {
                        slice.push_aux(index, (t - *f).inverse());
                    }
                }
            }
            Ctrl::If(b, t, f) => {
                let (b, _) = map[*b];
                if b != F::zero() {
                    t.populate_row(map, index, slice, queries);
                    slice.push_aux(index, b.inverse());
                } else {
                    f.populate_row(map, index, slice, queries);
                }
            }
        }
    }
}

impl<F: Field + Ord> Op<F> {
    fn populate_row(
        &self,
        map: &mut Vec<(F, Degree)>,
        index: &mut ColumnIndex,
        slice: &mut ColumnMutSlice<'_, F>,
        queries: &QueryRecord<F>,
    ) {
        match self {
            Op::Const(f) => map.push((*f, 0)),
            Op::Add(a, b) => {
                let (a, a_deg) = map[*a];
                let (b, b_deg) = map[*b];
                let deg = a_deg.max(b_deg);
                map.push((a + b, deg));
            }
            Op::Sub(a, b) => {
                let (a, a_deg) = map[*a];
                let (b, b_deg) = map[*b];
                let deg = a_deg.max(b_deg);
                map.push((a - b, deg));
            }
            Op::Mul(a, b) => {
                let (a, a_deg) = map[*a];
                let (b, b_deg) = map[*b];
                let deg = a_deg + b_deg;
                let f = a * b;
                if deg < 2 {
                    map.push((f, deg));
                } else {
                    map.push((f, 1));
                    slice.push_aux(index, f);
                }
            }
            Op::Inv(a) => {
                let (a, a_deg) = map[*a];
                let f = a.inverse();
                if a_deg == 0 {
                    map.push((f, 0));
                } else {
                    map.push((f, 1));
                    slice.push_aux(index, f);
                }
            }
            Op::Call(idx, inp) => {
                let args = inp.iter().map(|a| map[*a].0).collect::<List<_>>();
                let query_map = &queries.record()[*idx as usize];
                let result = query_map.get(&args).expect("Cannot find query result");
                for f in result.output.iter() {
                    map.push((*f, 1));
                    slice.push_aux(index, *f);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        func,
        lair::{
            demo_toplevel, execute::QueryRecord, field_from_i32, toplevel::Toplevel, trace::Width,
        },
    };

    use p3_baby_bear::BabyBear as F;
    use p3_field::AbstractField;

    use super::FuncChip;

    #[test]
    fn lair_width_test() {
        let toplevel = demo_toplevel::<F>();

        let factorial = toplevel.get_by_name("factorial").unwrap();
        let out = factorial.compute_width(&toplevel);
        let expected_width = Width {
            input: 1,
            output: 1,
            aux: 4,
            sel: 2,
        };
        assert_eq!(out, expected_width);
    }

    #[test]
    fn lair_trace_test() {
        let toplevel = demo_toplevel::<F>();
        let factorial_chip = FuncChip::from_name("factorial", &toplevel);
        let fib_chip = FuncChip::from_name("fib", &toplevel);
        let queries = &mut QueryRecord::new(&toplevel);

        let args = [F::from_canonical_u32(5)].into();
        queries.record_event(&toplevel, factorial_chip.func.index, args);
        let trace = factorial_chip.generate_trace(queries);
        let expected_trace = [
            // in order: n, output, mult, 1/n, fact(n-1), n*fact(n-1), and selectors
            0, 1, 1, 0, 0, 0, 0, 1, //
            1, 1, 1, 1, 1, 1, 1, 0, //
            2, 2, 1, 1, 2, 1006632961, 1, 0, //
            3, 6, 1, 2, 6, 1342177281, 1, 0, //
            4, 24, 1, 6, 24, 1509949441, 1, 0, //
            5, 120, 1, 24, 120, 1610612737, 1, 0, //
            // dummy
            0, 0, 0, 0, 0, 0, 0, 0, //
            0, 0, 0, 0, 0, 0, 0, 0, //
        ]
        .into_iter()
        .map(field_from_i32)
        .collect::<Vec<_>>();
        assert_eq!(trace.values, expected_trace);

        let args = [F::from_canonical_u32(7)].into();
        queries.record_event(&toplevel, fib_chip.func.index, args);
        let trace = fib_chip.generate_trace(queries);

        let expected_trace = [
            // in order: n, output, mult, fib(n-1), fib(n-2), 1/n, 1/(n-1), and selectors
            0, 1, 1, 0, 0, 0, 0, 1, 0, 0, //
            1, 1, 2, 0, 0, 0, 0, 0, 1, 0, //
            2, 2, 2, 1, 1, 1006632961, 1, 0, 0, 1, //
            3, 3, 2, 2, 1, 1342177281, 1006632961, 0, 0, 1, //
            4, 5, 2, 3, 2, 1509949441, 1342177281, 0, 0, 1, //
            5, 8, 2, 5, 3, 1610612737, 1509949441, 0, 0, 1, //
            6, 13, 1, 8, 5, 1677721601, 1610612737, 0, 0, 1, //
            7, 21, 1, 13, 8, 862828252, 1677721601, 0, 0, 1, //
        ]
        .into_iter()
        .map(field_from_i32)
        .collect::<Vec<_>>();
        assert_eq!(trace.values, expected_trace);
    }

    #[test]
    fn lair_match_trace_test() {
        let func_e = func!(
        fn test(n, m): 1 {
            let one = num(1);
            match n {
                0 => {
                    return one
                }
                1 => {
                    return m
                }
                2 => {
                    let res = mul(m, m);
                    return res
                }
                3 => {
                    let res = mul(m, m);
                    let res = mul(res, res);
                    return res
                }
            };
            let pred = sub(n, one);
            let res = call(test, pred, m);
            return res
        });
        let toplevel = Toplevel::<F>::new(&[func_e]);
        let test_chip = FuncChip::from_name("test", &toplevel);

        let expected_width = Width {
            input: 2,
            output: 1,
            aux: 6,
            sel: 5,
        };
        assert_eq!(test_chip.width, expected_width);

        let args = [F::from_canonical_u32(5), F::from_canonical_u32(2)].into();
        let queries = &mut QueryRecord::new(&toplevel);
        queries.record_event(&toplevel, test_chip.func.index, args);
        let trace = test_chip.generate_trace(queries);
        let expected_trace = [
            // The big numbers in the trace are the inverted elements, the witnesses of
            // the inequalities that appear on the default case. Note that the branch
            // that does not follow the default will reuse the slots for the inverted
            // elements to minimize the number of columns
            3, 2, 16, 1, 4, 16, 0, 0, 0, 0, 0, 0, 1, 0, //
            4, 2, 16, 1, 16, 1509949441, 1342177281, 1006632961, 1, 0, 0, 0, 0, 1, //
            5, 2, 16, 1, 16, 1610612737, 1509949441, 1342177281, 1006632961, 0, 0, 0, 0, 1, //
            // dummy
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
        ]
        .into_iter()
        .map(field_from_i32)
        .collect::<Vec<_>>();
        assert_eq!(trace.values, expected_trace);
    }

    #[test]
    fn lair_inner_match_trace_test() {
        let func_e = func!(
        fn test(n, m): 1 {
            let zero = num(0);
            let one = num(1);
            let two = num(2);
            let three = num(3);
            match n {
                0 => {
                    match m {
                        0 => {
                            return zero;
                        }
                        1 => {
                            return one;
                        }
                    }
                }
                1 => {
                    match m {
                        0 => {
                            return two;
                        }
                        1 => {
                            return three;
                        }
                    }
                }
            }
        });
        let toplevel = Toplevel::<F>::new(&[func_e]);
        let test_chip = FuncChip::from_name("test", &toplevel);

        let expected_width = Width {
            input: 2,
            output: 1,
            aux: 1,
            sel: 4,
        };
        assert_eq!(test_chip.width, expected_width);

        let zero = [F::from_canonical_u32(0), F::from_canonical_u32(0)].into();
        let one = [F::from_canonical_u32(0), F::from_canonical_u32(1)].into();
        let two = [F::from_canonical_u32(1), F::from_canonical_u32(0)].into();
        let three = [F::from_canonical_u32(1), F::from_canonical_u32(1)].into();
        let queries = &mut QueryRecord::new(&toplevel);
        queries.record_event(&toplevel, test_chip.func.index, zero);
        queries.record_event(&toplevel, test_chip.func.index, one);
        queries.record_event(&toplevel, test_chip.func.index, two);
        queries.record_event(&toplevel, test_chip.func.index, three);
        let trace = test_chip.generate_trace(queries);

        let expected_trace = [
            // two inputs, one output, multiplicity, selectors
            0, 0, 0, 1, 1, 0, 0, 0, //
            0, 1, 1, 1, 0, 1, 0, 0, //
            1, 0, 2, 1, 0, 0, 1, 0, //
            1, 1, 3, 1, 0, 0, 0, 1, //
        ]
        .into_iter()
        .map(field_from_i32)
        .collect::<Vec<_>>();
        assert_eq!(trace.values, expected_trace);
    }
}
