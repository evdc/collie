use crate::Scalar;

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Lit(Scalar),
    Col(usize),
    Select(usize),
    FilterEq,
    AddVs,
    DivVs,
}