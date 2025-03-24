use p3_field::PrimeField32;
use rand::Rng;

/// A random digest generator whose randomness is initiated from system entropy
/// everytime it's called.
pub(crate) fn rand_digest<R: Rng, F: PrimeField32, const SIZE: usize>(rng: &mut R) -> [F; SIZE] {
    let mut res = [F::zero(); SIZE];
    for limb in res.iter_mut().take(SIZE) {
        *limb = F::from_canonical_u32(rng.gen_range(0..F::ORDER_U32));
    }
    res
}
