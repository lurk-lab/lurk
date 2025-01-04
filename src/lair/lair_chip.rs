use p3_air::{Air, AirBuilder, AirBuilderWithPublicValues, BaseAir, PairBuilder};
use p3_field::{AbstractField, Field, PrimeField32};
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use sp1_stark::{
    air::{InteractionScope, MachineAir, MachineProgram},
    Chip,
};

use crate::air::builder::{LookupBuilder, ProvideRecord, RequireRecord};
use crate::gadgets::bytes::trace::BytesChip;

use super::{
    bytecode::Func,
    chipset::Chipset,
    execute::{Shard, MEM_TABLE_SIZES},
    func_chip::FuncChip,
    memory::MemChip,
    provenance::DEPTH_W,
    relations::{MemoryRelation, OuterCallRelation},
};

pub enum LairChip<F, C1: Chipset<F>, C2: Chipset<F>> {
    Func(FuncChip<F, C1, C2>),
    Mem(MemChip<F>),
    Bytes(BytesChip<F>),
    Entrypoint {
        func_idx: usize,
        num_public_values: usize,
    },
    Dummy, // FIXME: the prover requires at least one chip with commit_scope local
}

impl<F: PrimeField32, C1: Chipset<F>, C2: Chipset<F>> LairChip<F, C1, C2> {
    #[inline]
    pub fn entrypoint(func: &Func<F>) -> Self {
        let partial = if func.partial { DEPTH_W } else { 0 };
        let num_public_values = func.input_size + func.output_size + partial;
        Self::Entrypoint {
            func_idx: func.index,
            num_public_values,
        }
    }
}

impl<F: Field + Sync, C1: Chipset<F>, C2: Chipset<F>> BaseAir<F> for LairChip<F, C1, C2> {
    fn width(&self) -> usize {
        match self {
            Self::Func(func_chip) => func_chip.width(),
            Self::Mem(mem_chip) => mem_chip.width(),
            Self::Bytes(bytes_chip) => bytes_chip.width(),
            Self::Entrypoint {
                num_public_values, ..
            } => num_public_values + 1,
            Self::Dummy => 1,
        }
    }
}

pub struct LairMachineProgram;

impl<F: AbstractField> MachineProgram<F> for LairMachineProgram {
    fn pc_start(&self) -> F {
        F::zero()
    }
}

impl<F: PrimeField32, C1: Chipset<F>, C2: Chipset<F>> MachineAir<F> for LairChip<F, C1, C2> {
    type Record = Shard<F>;
    type Program = LairMachineProgram;

    fn name(&self) -> String {
        match self {
            Self::Func(func_chip) => format!("Func[{}]", func_chip.func.name),
            Self::Mem(mem_chip) => format!("Mem[{}-wide]", mem_chip.len),
            Self::Entrypoint { func_idx, .. } => format!("Entrypoint[{func_idx}]"),
            // the following is required by sphinx
            // TODO: engineer our way out of such upstream check
            Self::Bytes(_bytes_chip) => "CPU".to_string(),
            Self::Dummy => "Dummy".to_string(),
        }
    }

    fn generate_trace(&self, shard: &Self::Record, _: &mut Self::Record) -> RowMajorMatrix<F> {
        match self {
            Self::Func(func_chip) => func_chip.generate_trace(shard),
            Self::Mem(mem_chip) => mem_chip.generate_trace(shard),
            Self::Bytes(bytes_chip) => {
                // TODO: Shard the byte events differently?
                if shard.index() == 0 {
                    bytes_chip.generate_trace(&shard.queries().bytes)
                } else {
                    bytes_chip.generate_trace(&Default::default())
                }
            }
            Self::Entrypoint {
                num_public_values, ..
            } => {
                let mut public_values = shard.expect_public_values().to_vec();
                assert_eq!(*num_public_values, public_values.len());
                public_values.push(F::one());
                let height = public_values
                    .len()
                    .next_power_of_two()
                    .max(16 * self.width());
                public_values.resize(height, F::zero());
                RowMajorMatrix::new(public_values, self.width())
            }
            Self::Dummy => {
                let mut vals = vec![F::zero(); 16];
                vals[0] = F::one();
                RowMajorMatrix::new(vals, 1)
            }
        }
    }

    fn generate_dependencies(&self, _: &Self::Record, _: &mut Self::Record) {}

    fn included(&self, shard: &Self::Record) -> bool {
        match self {
            Self::Func(func_chip) => {
                let range = shard.get_func_range(func_chip.func.index);
                !range.is_empty()
            }
            Self::Mem(_mem_chip) => {
                shard.index == 0
                // TODO: This snippet or equivalent is needed for memory sharding
                // let range = shard.get_mem_range(mem_index_from_len(mem_chip.len));
                // !range.is_empty()
            }
            Self::Entrypoint { .. } => shard.index == 0,
            Self::Bytes(..) | Self::Dummy => true,
        }
    }

    fn preprocessed_width(&self) -> usize {
        match self {
            Self::Bytes(bytes_chip) => bytes_chip.preprocessed_width(),
            _ => 0,
        }
    }

    fn generate_preprocessed_trace(&self, _program: &Self::Program) -> Option<RowMajorMatrix<F>> {
        match self {
            Self::Bytes(bytes_chip) => bytes_chip.generate_preprocessed_trace(),
            _ => None,
        }
    }

    fn commit_scope(&self) -> InteractionScope {
        match self {
            Self::Dummy => InteractionScope::Local,
            _ => InteractionScope::Global,
        }
    }
}

impl<AB, C1: Chipset<AB::F>, C2: Chipset<AB::F>> Air<AB> for LairChip<AB::F, C1, C2>
where
    AB: AirBuilderWithPublicValues + LookupBuilder + PairBuilder,
    <AB as AirBuilder>::Var: std::fmt::Debug,
{
    fn eval(&self, builder: &mut AB) {
        match self {
            Self::Func(func_chip) => func_chip.eval(builder),
            Self::Mem(mem_chip) => mem_chip.eval(builder),
            Self::Bytes(bytes_chip) => bytes_chip.eval(builder),
            Self::Entrypoint {
                func_idx,
                num_public_values,
            } => {
                let func_idx = AB::F::from_canonical_usize(*func_idx);
                let main = builder.main();
                let mut public_values = main.row(0).collect::<Vec<_>>();
                // let mut public_values = builder.main().first_row().collect::<Vec<_>>();
                let is_real = public_values.pop().expect("Missing is_real");
                assert_eq!(public_values.len(), *num_public_values);

                // these values aren't correct for all builders!
                let public_values_from_builder = builder.public_values().to_vec();
                for (&a, b) in public_values.iter().zip(public_values_from_builder) {
                    // this is only accounted for by the builder used to collect constraints
                    builder.when(is_real).assert_eq(a, b);
                }

                builder.require(
                    OuterCallRelation(func_idx, public_values),
                    AB::F::zero(),
                    RequireRecord {
                        prev_nonce: AB::F::zero(),
                        prev_count: AB::F::zero(),
                        count_inv: AB::F::one(),
                    },
                    is_real,
                );
            }
            Self::Dummy => {
                // let is_real = builder.main().first_row().collect::<Vec<_>>()[0];
                let is_real = builder.main().row(0).collect::<Vec<_>>()[0];
                builder.assert_eq(is_real, is_real);
                builder.require(
                    MemoryRelation(
                        AB::F::from_canonical_u32(1234),
                        vec![AB::F::from_canonical_u32(1234)],
                    ),
                    AB::F::zero(),
                    RequireRecord {
                        prev_nonce: AB::F::zero(),
                        prev_count: AB::F::zero(),
                        count_inv: AB::F::one(),
                    },
                    is_real,
                );
                builder.provide(
                    MemoryRelation(
                        AB::F::from_canonical_u32(1234),
                        vec![AB::F::from_canonical_u32(1234)],
                    ),
                    ProvideRecord {
                        last_nonce: AB::F::zero(),
                        last_count: AB::F::one(),
                    },
                    is_real,
                );
            }
        }
    }
}

pub fn build_lair_chip_vector<F: PrimeField32, C1: Chipset<F>, C2: Chipset<F>>(
    entry_func_chip: &FuncChip<F, C1, C2>,
) -> Vec<LairChip<F, C1, C2>> {
    let toplevel = &entry_func_chip.toplevel;
    let func = &entry_func_chip.func;
    let mut chip_vector = Vec::with_capacity(2 + toplevel.num_funcs() + MEM_TABLE_SIZES.len());
    chip_vector.push(LairChip::entrypoint(func));
    for func_chip in FuncChip::from_toplevel(toplevel) {
        chip_vector.push(LairChip::Func(func_chip));
    }
    for mem_len in MEM_TABLE_SIZES {
        chip_vector.push(LairChip::Mem(MemChip::new(mem_len)));
    }
    chip_vector.push(LairChip::Bytes(BytesChip::default()));
    chip_vector.push(LairChip::Dummy);
    chip_vector
}

#[inline]
pub fn build_chip_vector_from_lair_chips<
    F: PrimeField32,
    C1: Chipset<F>,
    C2: Chipset<F>,
    I: IntoIterator<Item = LairChip<F, C1, C2>>,
>(
    lair_chips: I,
) -> Vec<Chip<F, LairChip<F, C1, C2>>> {
    lair_chips.into_iter().map(Chip::new).collect()
}

#[inline]
pub fn build_chip_vector<F: PrimeField32, C1: Chipset<F>, C2: Chipset<F>>(
    entry_func_chip: &FuncChip<F, C1, C2>,
) -> Vec<Chip<F, LairChip<F, C1, C2>>> {
    build_chip_vector_from_lair_chips(build_lair_chip_vector(entry_func_chip))
}

#[cfg(test)]
mod tests {
    use crate::lair::{demo_toplevel, execute::QueryRecord, func_chip::FuncChip};

    use super::*;

    use p3_baby_bear::BabyBear;
    use sp1_stark::baby_bear_poseidon2::BabyBearPoseidon2;
    use sp1_stark::{CpuProver, MachineProver, SP1CoreOpts, StarkGenericConfig, StarkMachine};

    #[test]
    fn test_prove_and_verify() {
        sp1_core_machine::utils::setup_logger();
        type F = BabyBear;
        let toplevel = demo_toplevel::<F>();
        let chip = FuncChip::from_name("factorial", &toplevel);
        let mut queries = QueryRecord::new(&toplevel);

        toplevel
            .execute_by_name("factorial", &[F::from_canonical_u8(5)], &mut queries, None)
            .unwrap();

        let config = BabyBearPoseidon2::new();
        let machine = StarkMachine::new(
            config,
            build_chip_vector(&chip),
            queries.expect_public_values().len(),
            true,
        );

        let (pk, vk) = machine.setup(&LairMachineProgram);
        let mut challenger_p = machine.config().challenger();
        let mut challenger_v = machine.config().challenger();
        let mut challenger_d = machine.config().challenger();
        let shard = Shard::new(&queries);

        machine.debug_constraints(&pk, shard.clone(), &mut challenger_d);
        let opts = SP1CoreOpts::default();
        let prover = CpuProver::new(machine);
        let proof = prover
            .prove(&pk, shard, &mut challenger_p, opts)
            .expect("proof generates");
        let config = BabyBearPoseidon2::new();
        let verifier_machine = StarkMachine::new(
            config,
            build_chip_vector(&chip),
            queries.expect_public_values().len(),
            true,
        );
        verifier_machine
            .verify(&vk, &proof, &mut challenger_v)
            .expect("proof verifies");
    }
}
