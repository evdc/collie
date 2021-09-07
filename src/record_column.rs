// An experimental demonstration of how we might store packed struct/record type columns.
// Compare/contrast with the Arrow spec on this
// requires nightly

// n.b. Arrow stores structs by decomposing into one columnar array per field;
// but with a validity bitmap for the whole thing
// which is (sorta) equivalent to an EntityColumn + multiple other columns for entities of that type
// the whole point here is mainly for things like `record Point(x: Num, y: Num)`
// where you will almost always want to access all values of a struct or "row" at the same time

// possible improvements:
// - store strings inline by converting them to bytes (now your records are variable width)
// - if you don't want to store them inline, instead of pointers, try something like Arrow's
//      Dictionary encoding - put the string data out-of-band and store an index?

#![feature(vec_into_raw_parts)]

use std::convert::TryInto;


#[derive(Debug, Clone, PartialEq)]
pub enum Datatype {
    Bool,
    Num,
    Str,
    Entity,
    Record
}


#[derive(Debug)]
pub struct RecordColumn {
    // For now, we only store records of scalars, no nesting
    data: Vec<u8>,
    pub len: usize,         // how many records are stored - not how many bytes
    width: usize,           // how many bytes does each record occupy?
    offsets: Vec<usize>,    // where does each record start?
    fields: Vec<Datatype>
}

fn width_of(dt: &Datatype) -> usize {
    // How many bytes to store each datatype?
    match *dt {
        Datatype::Bool => 1,
        Datatype::Num  => 8,
        Datatype::Str  => 24,   // 3x8 - ptr, len, capacity
        Datatype::Entity => 8,
        Datatype::Record => 24, // ptr, len, capacity, like string?
    }
}

fn read_f64(input: &[u8]) -> f64 {
    let (bytes, _rest) = input.split_at(8);
    f64::from_le_bytes(bytes.try_into().unwrap())
}

fn read_u64(input: &[u8]) -> u64 {
    let (bytes, _rest) = input.split_at(8);
    u64::from_le_bytes(bytes.try_into().unwrap())
}

unsafe fn read_string(input: &[u8]) -> String {
    // SAFETY: MUST have been previously written with RecordColumn::insert
    let (ptr_bytes, rest) = input.split_at(std::mem::size_of::<&u8>());
    let (len_bytes, rest) = rest.split_at(8);
    let (cap_bytes, _)    = rest.split_at(8);
    let ptr = (usize::from_le_bytes(ptr_bytes.try_into().unwrap())) as *mut u8;
    let len = usize::from_le_bytes(len_bytes.try_into().unwrap());
    let cap = usize::from_le_bytes(cap_bytes.try_into().unwrap());
    String::from_raw_parts(ptr, len, cap)
}

impl RecordColumn {
    pub fn for_fields(fields: Vec<Datatype>) -> Self {
        // calculate offsets for fields -- cumulative sum
        let width = fields
            .iter()
            .map(|f| width_of(f))
            .sum();
        RecordColumn { data: Vec::new(), offsets: Vec::new(), fields, len: 0, width }
    }

    pub fn insert(&mut self, data: Vec<Scalar>) -> () {
        // data MUST be a vector of scalars of the same types, in the same order, as self.fields
        // this is not checked - assume our hypothetical compiler generates correct code
        for d in data {
            match d {
                Scalar::Bool(x) => self.data.push(x as u8),
                Scalar::Num(x)  => self.data.extend(x.to_le_bytes()),
                Scalar::Str(x)  => {
                    // requires nightly
                    let (ptr, len, capacity) = x.into_raw_parts();
                    self.data.extend((ptr as u64).to_le_bytes());
                    self.data.extend((len as u64).to_le_bytes());
                    self.data.extend((capacity as u64).to_le_bytes());
                },
                Scalar::Entity(x) => self.data.extend(x.to_le_bytes())
            }
        }
        self.len += 1;
    }

    pub fn get(&self, idx: usize) -> Vec<Scalar> {
        let mut offset = self.width * idx;
        let mut res = Vec::new();
        for dtype in &self.fields {
            let val = match dtype {
                Datatype::Bool => {
                    let v = Scalar::Bool(self.data[offset] != 0);
                    offset += 1;
                    v
                },
                Datatype::Num => {
                    let num = read_f64(&self.data[offset..offset+8]);
                    offset += 8;
                    Scalar::Num(num)
                },
                Datatype::Entity => {
                    let num = read_u64(&self.data[offset..offset+8]);
                    offset += 8;
                    Scalar::Entity(num)
                },
                Datatype::Str => {
                    let slice = &self.data[offset..offset+24];
                    offset += 24;
                    unsafe {
                        // SAFETY: bytes representing string will have been written with `self.insert`
                        Scalar::Str(read_string(slice))
                    }
                }
            };
            res.push(val);
        }
        res
    }
}


fn main() {
    let mut rc = RecordColumn::for_fields(vec![
        Datatype::Bool,
        Datatype::Num,
        Datatype::Str
    ]);

    let a_record = vec![
        Scalar::Bool(true),
        Scalar::Num(12.3),
        Scalar::Str(String::from("hello"))
    ];
    let another_record = a_record.clone();

    rc.insert(a_record);
    rc.insert(another_record);

    println!("{:?}", rc);

    let res = rc.get(1);
    println!("{:?}", res);
}