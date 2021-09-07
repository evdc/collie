#[derive(Debug)]
pub enum VMError {
    TypeError(String),
    IllegalOpcode
}
