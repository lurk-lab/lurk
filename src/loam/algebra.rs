use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use thiserror::Error;

use super::Attribute;

#[derive(Error, Debug, PartialEq)]
pub enum AlgebraError {
    #[error("Duplicate Attribute")]
    DuplicateAttribute,
}

pub trait Algebra<A: Attribute, T>: PartialEq + Sized {
    fn and(&self, other: &Self) -> Option<Self>;
    fn or(&self, other: &Self) -> Self;
    fn not(&self) -> Self;
    fn project<I: Into<HashSet<A>>>(&self, attrs: I) -> Self;
    fn remove<I: Into<HashSet<A>>>(&self, attrs: I) -> Self;
    fn rename<I: Into<HashMap<A, A>>>(&self, mapping: I) -> Result<Self, AlgebraError>;
    fn compose(&self, other: &Self) -> Option<Self>;

    fn is_negated(&self) -> bool;
    fn disjunction(&self) -> &Option<BTreeSet<BTreeMap<A, T>>>;
}