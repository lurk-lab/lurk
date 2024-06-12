use std::cmp::Ordering;

use ascent::Lattice;
use p3_baby_bear::BabyBear;
use p3_field::{AbstractField, PrimeField32};

use crate::lurk::zstore::Tag;

mod allocation;
mod evaluation;

pub type LE = BabyBear;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Num(LE);

#[derive(Clone, Copy, Debug, Ord, PartialEq, Eq, Hash)]
struct LEWrap(LE);

impl PartialOrd for LEWrap {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0
            .as_canonical_u32()
            .partial_cmp(&other.0.as_canonical_u32())
    }
}

impl Lattice for LEWrap {
    fn meet(self, other: Self) -> Self {
        self.min(other)
    }
    fn join(self, other: Self) -> Self {
        self.max(other)
    }
}

#[derive(Clone, Copy, Debug, Ord, PartialOrd, PartialEq, Eq, Hash)]
pub struct Ptr(pub LE, pub LE);

impl Ptr {
    fn nil() -> Self {
        Self(Tag::Nil.elt(), LE::from_canonical_u32(0))
    }

    fn f(val: LE) -> Self {
        Self(Tag::Num.elt(), val)
    }
    fn is_f(&self) -> bool {
        self.0 == Tag::Num.elt()
    }
    fn is_cons(&self) -> bool {
        self.0 == Tag::Cons.elt()
    }
    fn is_nil(&self) -> bool {
        // TODO: should we also check value?
        self.0 == Tag::Nil.elt()
    }
    fn is_sym(&self) -> bool {
        self.0 == Tag::Sym.elt()
    }
    fn is_fun(&self) -> bool {
        self.0 == Tag::Fun.elt()
    }
    fn is_err(&self) -> bool {
        self.0 == Tag::Err.elt()
    }
}

#[derive(Clone, Copy, Debug, Ord, PartialOrd, PartialEq, Eq, Hash)]
pub struct Wide(pub [LE; 8]);

impl Wide {
    pub fn widen(elt: LE) -> Self {
        let mut v = [LE::zero(); 8];
        v[0] = elt;
        Wide(v)
    }

    pub fn f(&self) -> LE {
        //        assert_eq!(&[0, 0, 0, 0, 0, 0, 0], &self.0[1..]);
        self.0[0]
    }

    pub fn from_slice(elts: &[LE]) -> Self {
        let mut v = [LE::zero(); 8];
        v.copy_from_slice(elts);
        Wide(v)
    }
}

impl From<&Num> for Wide {
    fn from(f: &Num) -> Self {
        Wide::widen(f.0)
    }
}
impl From<Num> for Wide {
    fn from(f: Num) -> Self {
        (&f).into()
    }
}

#[derive(Clone, Copy, Debug, Ord, PartialOrd, PartialEq, Eq, Hash)]
pub struct WidePtr(pub Wide, pub Wide);

impl WidePtr {
    fn nil() -> Self {
        Self(Tag::Nil.value(), Wide::widen(LE::from_canonical_u32(0)))
    }
    fn empty_env() -> Self {
        Self::nil()
    }
}

impl From<&Num> for WidePtr {
    fn from(f: &Num) -> Self {
        WidePtr(Tag::Num.value(), f.into())
    }
}

impl From<Num> for WidePtr {
    fn from(f: Num) -> Self {
        (&f).into()
    }
}
