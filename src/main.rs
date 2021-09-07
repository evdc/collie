use std::rc::Rc;

mod column;
mod bitindex;
mod errors;
mod opcode;
use crate::column::*;
use crate::opcode::Op;
use crate::errors::VMError;

// TODO
// - wrap Scalar::Str in rc
// - profile, try to figure out how bad rc overhead is
// - consider alternatives to rc, most likely unsafe moving of ptrs, or implementing your own Heap
// - ... all the language features ...
// - figure out what to do about non-primitive type columns:
//  - list-valued columns (necessary for current impl of group by/agg)
//  - struct/record-type columns, unless you're *very* religious about normalization.

#[derive(Debug)]
pub enum Value {
    // A value on the Stack.
    Scalar(Scalar),
    ColumnRef(Rc<Column>)
}

pub struct VM {
    code: Vec<Op>,
    ip: usize,
    stack: Vec<Value>,
    columns: Vec<Rc<Column>>
}

// so what SHOULD be done with the col reference when pushing on stack
// if we wanted to avoid the overhead of RC?
// Op::Col can "move" ownership of the ref from `self.columns` to `self.stack` theoretically,
// but unless we std::mem::take the val out of the vec (or remove it, and shift the rest of the elems)
// a ref will also remain in the vec too which Rust considers invalid
// we *think* that only one of these will be used at a time -- because of the serial nature of
// push/pop off the stack -- and because 1) only Op::Col will refer into `self.columns`, other opcodes
// (or their helpers) never work with column indices directly, they just pop them off the stack and
// 2) a correct compiler will never generate two Op::Col(i) for the same i, without some other
// opcode in between that pops that ColumnRef off the stack. But the compiler doesn't know that.
// so I think we have to use Rc here, or unsafe.
// profile and see how big the overhead of refcounting is -- likely not that bad, if it's amortized
//  over columns.


impl VM {
    pub fn new(columns: Vec<Column>) -> Self {
        // take ownership of columns and wrap them in rc's
        let rcs = columns.into_iter().map(|col| Rc::new(col)).collect();
        VM { code: Vec::new(), ip: 0, stack: Vec::new(), columns: rcs }
    }

    // Associated functions so they can borrow part of self, rather than borrowing all of self as mut
    fn pop_scalar(stack: &mut Vec<Value>) -> Result<Scalar, VMError> {
        if let Some(Value::Scalar(s)) = stack.pop() { return Ok(s); }
        return Err(VMError::TypeError(format!("expected a scalar value")))
    }

    fn pop_column(stack: &mut Vec<Value>) -> Result<Rc<Column>, VMError> {
        if let Some(Value::ColumnRef(c)) = stack.pop() { return Ok(c); }
        return Err(VMError::TypeError(format!("expected a column value")))
    }

    fn expect_col_bool(v: Rc<Column>) -> Result<BoolColumn, VMError> {
        // is there a better way to do this?
        let res = Rc::try_unwrap(v).unwrap();
        if let Column::Bool(inner) = res {
            return Ok(inner);
        }
        Err(VMError::TypeError(format!("Type error: expected a boolean column, found: {:?}", res)))
    }

    pub fn run(&mut self, code: Vec<Op>) -> Result<(), VMError>  {
        self.code = code;

        while self.ip < self.code.len() {
            let op = &self.code[self.ip];
            self.ip += 1;

            println!("Stack: {:?}", self.stack);
            println!("Op: {:?}", op);

            match op {

                Op::Lit(s) => self.stack.push(Value::Scalar(s.clone())),

                // panics(?) if idx is not a valid column idx
                Op::Col(idx) => self.stack.push(
                    Value::ColumnRef(self.columns[*idx].clone())    // Clone the RC = inc reference
                ),

                Op::FilterEq => {
                    // TOS is a scalar. TOS-1 is a column.
                    // Push a new column of positions
                    let s = VM::pop_scalar(&mut self.stack)?;
                    let col = VM::pop_column(&mut self.stack)?;
                    let new_col = Column::Bool(col.filter(s)?);
                    self.stack.push(Value::ColumnRef(Rc::new(new_col)));
                },

                Op::Select(_) => {
                    // todo: select multiple
                    let data = VM::pop_column(&mut self.stack)?;
                    let selector = VM::pop_column(&mut self.stack)?;
                    let selector = VM::expect_col_bool(selector)?;
                    let new_col = Column::from(data.select(&selector));
                    self.stack.push(Value::ColumnRef(Rc::new(new_col)));
                }

                _ => { return Err(VMError::IllegalOpcode); }

            }
        }


        Ok(())
    }

}


fn test_vm() {
    let persons: Vec<Column> = vec![
        Column::from(vec!["alice", "bob", "carol", "dave"]),  // col 0 - name
        Column::from(vec![18.0, 42.0, 34.0, 20.0]),      // col 1 - age
        Column::InlineStr(InlineStrColumn::from_strs(vec!["f", "m", "f", "m"]))
    ];

    let code = vec![
        Op::Col(2),                 // load column 2 (sex)
        Op::Lit(Scalar::Str("f".to_string())),  // load scalar
        Op::FilterEq,               // pop both and push a bit mask of equal positions
        Op::Col(0),                 // load col 0 (name)
        Op::Select(1)               // pop the column and bit mask, push a new column

    ];

    let mut vm = VM::new(persons);
    vm.run(code);
    println!("{:?}", vm.stack);
}


fn main() {
    test_vm()
}
