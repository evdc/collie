use crate::bitindex::BitIndex;
use crate::errors::VMError;

use std::fmt;

type EntityT = u64;

// todo: Rc<String> ?
#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    Bool(bool),
    Num(f64),
    Str(String),
    Entity(EntityT),
    Record(Vec<Scalar>)
}

pub trait ColumnT {
    fn filter(&self, val: Scalar) -> Result<BoolColumn, VMError>;
    fn select(&self, mask: &BoolColumn) -> Self;
}

#[derive(Debug)]
pub struct BoolColumn {
    data: BitIndex
}

#[derive(Debug)]
pub struct NumColumn {
    data: Vec<f64>
}

#[derive(Debug)]
pub struct StrColumn {
    data: Vec<String>
}

#[derive(Debug)]
pub struct InlineStrColumn {
    // c.f. Arrow's "Variable Binary" layout
    data: Vec<u8>,
    offsets: Vec<usize>
}

impl InlineStrColumn {
    pub fn from_strs(strs: Vec<&str>) -> Self {
        let mut data = Vec::new();
        let mut offsets = vec![0];
        for s in strs {
            data.extend(s.as_bytes());
            // safe - we know offsets is non-empty, we just initialized it 2 lines ago
            offsets.push(offsets.last().unwrap() + s.len());
        }
        InlineStrColumn { data, offsets }
    }
}

#[derive(Debug)]
pub struct EntityColumn {
    data: Vec<EntityT>
}

fn _filter_eq<T: PartialEq>(col: &Vec<T>, val: T) -> Vec<EntityT> {
    // Find occurrences of `val` and return positions at which they occur.
    // todo: accept arbitrary predicates?
    col.iter()
        .enumerate()
        .filter(|(_i, x)| **x == val)
        .map(|(i, _x)| i as EntityT)
        .collect()
}

fn _filter_eq_bool<T: PartialEq>(col: &Vec<T>, val: T) -> BoolColumn {
    // Find occurences of `val` in `col` and return a boolean mask
    let mut positions = BitIndex::for_col_len(col.len());
    col.iter()
        .enumerate()
        .filter(|(_i, x)| **x == val)
        .for_each(|(i, _x)| positions.set(i));
    BoolColumn { data: positions }
}

impl ColumnT for BoolColumn {
    fn filter(&self, val: Scalar) -> Result<BoolColumn, VMError> {
        if let Scalar::Bool(x) = val {
            match x {
                true => Ok(BoolColumn { data: self.data.clone() }),
                false => Ok(BoolColumn { data: self.data.inverted() })
            }
        } else {
            Err(VMError::TypeError(format!("Expected a boolean value, got: {:?}", val)))
        }
    }

    fn select(&self, _mask: &BoolColumn) -> BoolColumn {
        unimplemented!()    // this one's a bit of a special case
    }
}

impl ColumnT for NumColumn {
    fn filter(&self, val: Scalar) -> Result<BoolColumn, VMError> {
        if let Scalar::Num(x) = val {
            Ok(_filter_eq_bool(&self.data, x))
        } else {
            Err(VMError::TypeError(format!("Expected a numeric value, got: {:?}", val)))
        }
    }

    fn select(&self, mask: &BoolColumn) -> Self {
        let res = mask.data.select(&self.data);
        Self { data: res }
    }
}

impl ColumnT for StrColumn {
    fn filter(&self, val: Scalar) -> Result<BoolColumn, VMError> {
        if let Scalar::Str(x) = val {
            Ok(_filter_eq_bool(&self.data, x))
        } else {
            Err(VMError::TypeError(format!("Expected a string value, got: {:?}", val)))
        }
    }

    fn select(&self, mask: &BoolColumn) -> Self {
        let res = mask.data.select(&self.data);
        Self { data: res }
    }
}

impl ColumnT for EntityColumn {
    fn filter(&self, val: Scalar) -> Result<BoolColumn, VMError> {
        if let Scalar::Entity(x) = val {
            Ok(_filter_eq_bool(&self.data, x))
        } else {
            Err(VMError::TypeError(format!("Expected an entity-id value, got: {:?}", val)))
        }
    }

    fn select(&self, mask: &BoolColumn) -> Self {
        let res = mask.data.select(&self.data);
        Self { data: res }
    }
}


impl ColumnT for InlineStrColumn {
    fn filter(&self, val: Scalar) -> Result<BoolColumn, VMError> {
        if let Scalar::Str(x) = val {
            let scalar_bytes = x.into_bytes();
            let mut positions = BitIndex::for_col_len(self.offsets.len());
            for i in 0 .. self.offsets.len() - 1 {
                let bytes = &self.data[self.offsets[i] .. self.offsets[i+1]];
                if scalar_bytes == bytes {
                    positions.set(i);
                }
            }
            Ok(BoolColumn { data: positions })
        } else {
            Err(VMError::TypeError(format!("Expected a string value, got: {:?}", val)))
        }
    }

    fn select(&self, mask: &BoolColumn) -> Self {
        let mut data = Vec::new();
        let mut offsets = vec![0];
        mask.data.for_each(|idx| {
            let bytes = &self.data[self.offsets[idx] .. self.offsets[idx+1]];
            data.extend(bytes);
            offsets.push(offsets[idx] + bytes.len());
        });
        InlineStrColumn { data: data, offsets }
    }
}

#[derive(Debug)]
pub enum Column {
    Bool(BoolColumn),
    Num(NumColumn),
    Str(StrColumn),
    Entity(EntityColumn),
    InlineStr(InlineStrColumn)
}

impl ColumnT for Column {
    fn filter(&self, val: Scalar) -> Result<BoolColumn, VMError> {
        match self {
            Column::Bool(col)   => col.filter(val),
            Column::Num(col)    => col.filter(val),
            Column::Str(col)    => col.filter(val),
            Column::Entity(col) => col.filter(val),
            Column::InlineStr(col) => col.filter(val)
        }
    }

    fn select(&self, mask: &BoolColumn) -> Self {
        match self {
            Column::Bool(col)   => Column::Bool(col.select(mask)),
            Column::Num(col)    => Column::Num(col.select(mask)),
            Column::Str(col)    => Column::Str(col.select(mask)),
            Column::Entity(col) => Column::Entity(col.select(mask)),
            Column::InlineStr(col) => Column::InlineStr(col.select(mask))
        }
    }
}

impl From<Vec<f64>> for Column {
    fn from(v: Vec<f64>) -> Self {
        Column::Num(NumColumn { data: v })
    }
}

impl From<Vec<String>> for Column {
    fn from(v: Vec<String>) -> Self {
        Column::Str(StrColumn { data: v })
    }
}

// convenience?
impl From<Vec<&str>> for Column {
    fn from(v: Vec<&str>) -> Self {
        let v = v.iter().map(|s| s.to_string()).collect();
        Column::Str(StrColumn { data: v })
    }
}

impl From<Vec<EntityT>> for Column {
    fn from(v: Vec<EntityT>) -> Self {
        Column::Entity(EntityColumn { data: v })
    }
}

impl fmt::Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Column::Bool(c) => write!(f, "Bool[{}]", c.data),
            Column::Num(c) => write!(f, "Num[{:?}]", c.data),
            Column::Str(c) => write!(f, "Str[{:?}]", c.data),
            Column::InlineStr(c) => write!(f, "Str[{:?}]", c.data),
            Column::Entity(c) => write!(f, "Entity[{:?}]", c.data)
        }
    }
}

