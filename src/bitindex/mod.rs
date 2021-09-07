use std::fmt;

#[derive(Debug, Clone)]
pub struct BitIndex {
    data: Vec<u64>
}

impl BitIndex {
    pub fn for_col_len(len: usize) -> Self {
        BitIndex { data: vec![0; len / 64 + 1] }
    }

    pub fn set(&mut self, idx: usize) -> () {
        let block = (idx as u64) >> 6;
        let bit = (idx as u64) % 64;
        self.data[block as usize] |= 1 << bit;
    }

    pub fn inverted(&self) -> BitIndex {
        BitIndex { data: self.data.iter().map(|x| !*x).collect() }
    }

    pub fn select<T>(&self, col: &Vec<T>) -> Vec<T> where T: Clone {
        // Is it faster to do this (iter over set bits)
        // or iter over all indices in `col` and check bit at that index?
        let mut res = Vec::new();
        for block_idx in 0 .. self.data.len() {
            let mut block = self.data[block_idx];
            while block != 0 {
                let mask = (block as i64) & -(block as i64);    // set all bits to 0 except lowest 1
                let tz = block.trailing_zeros();
                let idx = block_idx * 64 + (tz as usize);
                res.push(col[idx].clone());
                block ^= mask as u64;
            }
        }
        res
    }

    pub fn for_each<F>(&self, mut callback: F) -> ()
        where F: FnMut(usize) -> () {

        for block_idx in 0 .. self.data.len() {
            let mut block = self.data[block_idx];
            while block != 0 {
                let mask = (block as i64) & -(block as i64);    // set all bits to 0 except lowest 1
                let tz = block.trailing_zeros();
                let idx = block_idx * 64 + (tz as usize);
                callback(idx);
                block ^= mask as u64;
            }
        }
    }
}

impl fmt::Display for BitIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BitIndex[")?;
        for x in &self.data {
            write!(f, "{:#066b}", x)?;
        }
        write!(f, "]")
    }
}