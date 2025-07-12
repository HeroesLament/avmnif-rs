#[derive(Debug, Copy, Clone)]
pub struct Term(usize);

pub type NifResult<T> = Result<T, Term>;

impl Term {
    pub fn from_i64(n: i64) -> Term {
        // temp Stub
        Term(n as usize)
    }
}
