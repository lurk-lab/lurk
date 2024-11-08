use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use p3_baby_bear::BabyBear;
use p3_field::AbstractField;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sphinx_core::{
    air::MachineAir,
    stark::{LocalProver, StarkGenericConfig, StarkMachine},
    utils::{BabyBearPoseidon2, SphinxCoreOpts},
};
use std::time::Duration;

use lurk::{
    core::{
        eval_direct::build_lurk_toplevel_native,
        zstore::{lurk_zstore, ZPtr},
    },
    lair::{
        chipset::{Chipset, NoChip},
        execute::{QueryRecord, Shard},
        func_chip::FuncChip,
        lair_chip::{build_chip_vector, build_lair_chip_vector, LairMachineProgram},
        toplevel::Toplevel,
        List,
    },
};

fn get_lcs_args() -> (&'static str, &'static str) {
    ( "When in the Course of human events, it becomes necessary for one people to dissolve the political bands which have connected them with another",
       "There must be some kind of way outta here Said the joker to the thief. There's too much confusion. I can't get no relief.")
}

fn build_lurk_expr(a: &str, b: &str) -> String {
    format!(
        r#"
(letrec ((lte (lambda (a b)
                (if (eq a "") t
                    (if (eq b "") nil
                        (lte (cdr a) (cdr b))))))
         (lcs (lambda (a b)
                (if (eq a "") ""
                    (if (eq b "") ""
                        (if (eq (car a) (car b)) (strcons (car a) (lcs (cdr a) (cdr b)))
                            (if (lte (lcs a (cdr b)) (lcs (cdr a) b)) (lcs (cdr a) b)
                                (lcs a (cdr b)))))))))
  (lcs "{a}" "{b}"))"#
    )
}

fn setup<'a, C: Chipset<BabyBear>>(
    a: &'a str,
    b: &'a str,
    toplevel: &'a Toplevel<BabyBear, C, NoChip>,
) -> (
    List<BabyBear>,
    FuncChip<'a, BabyBear, C, NoChip>,
    QueryRecord<BabyBear>,
) {
    let code = build_lurk_expr(a, b);
    let zstore = &mut lurk_zstore();
    let ZPtr { tag, digest } = zstore.read(&code, &Default::default());

    let mut record = QueryRecord::new(toplevel);
    record.inject_inv_queries("hash4", toplevel, &zstore.hashes4);

    let mut full_input = [BabyBear::zero(); 24];
    full_input[0] = tag.to_field();
    full_input[8..16].copy_from_slice(&digest);

    let args: List<_> = full_input.into();
    let lurk_main = FuncChip::from_name_main("lurk_main", toplevel);

    (args, lurk_main, record)
}

fn evaluation(c: &mut Criterion) {
    let args = get_lcs_args();
    c.bench_function("lcs-evaluation", |b| {
        let (toplevel, ..) = build_lurk_toplevel_native();
        let (args, lurk_main, record) = setup(args.0, args.1, &toplevel);
        b.iter_batched(
            || (args.clone(), record.clone()),
            |(args, mut queries)| {
                lurk_main.execute(&args, &mut queries, None).unwrap();
            },
            BatchSize::SmallInput,
        )
    });
}

fn trace_generation(c: &mut Criterion) {
    let args = get_lcs_args();
    c.bench_function("lcs-trace-generation", |b| {
        let (toplevel, ..) = build_lurk_toplevel_native();
        let (args, lurk_main, mut record) = setup(args.0, args.1, &toplevel);
        lurk_main.execute(&args, &mut record, None).unwrap();
        let lair_chips = build_lair_chip_vector(&lurk_main);
        b.iter(|| {
            lair_chips.par_iter().for_each(|func_chip| {
                let shard = Shard::new(&record);
                func_chip.generate_trace(&shard, &mut Default::default());
            })
        })
    });
}

fn verification(c: &mut Criterion) {
    let args = get_lcs_args();
    c.bench_function("lcs-verification", |b| {
        let (toplevel, ..) = build_lurk_toplevel_native();
        let (args, lurk_main, mut record) = setup(args.0, args.1, &toplevel);

        toplevel
            .execute(lurk_main.func(), &args, &mut record, None)
            .unwrap();
        let config = BabyBearPoseidon2::new();
        let machine = StarkMachine::new(
            config,
            build_chip_vector(&lurk_main),
            record.expect_public_values().len(),
        );
        let (pk, vk) = machine.setup(&LairMachineProgram);
        let mut challenger_p = machine.config().challenger();
        let opts = SphinxCoreOpts::default();
        let shard = Shard::new(&record);
        let proof = machine.prove::<LocalProver<_, _>>(&pk, shard, &mut challenger_p, opts);

        b.iter_batched(
            || machine.config().challenger(),
            |mut challenger| {
                machine.verify(&vk, &proof, &mut challenger).unwrap();
            },
            BatchSize::SmallInput,
        )
    });
}

fn e2e(c: &mut Criterion) {
    let args = get_lcs_args();
    c.bench_function("lcs-e2e", |b| {
        let (toplevel, ..) = build_lurk_toplevel_native();
        let (args, lurk_main, record) = setup(args.0, args.1, &toplevel);

        b.iter_batched(
            || (record.clone(), args.clone()),
            |(mut record, args)| {
                lurk_main.execute(&args, &mut record, None).unwrap();
                let config = BabyBearPoseidon2::new();
                let machine = StarkMachine::new(
                    config,
                    build_chip_vector(&lurk_main),
                    record.expect_public_values().len(),
                );
                let (pk, _) = machine.setup(&LairMachineProgram);
                let mut challenger_p = machine.config().challenger();
                let opts = SphinxCoreOpts::default();
                let shard = Shard::new(&record);
                machine.prove::<LocalProver<_, _>>(&pk, shard, &mut challenger_p, opts);
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group! {
    name = lcs_benches;
    config = Criterion::default()
                .measurement_time(Duration::from_secs(15))
                .sample_size(10);
    targets =
        evaluation,
        trace_generation,
        verification,
        e2e,
}

// `cargo criterion --bench lcs
criterion_main!(lcs_benches);
