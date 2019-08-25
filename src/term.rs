/// A compiled term represented as an abstract syntax tree.
#[derive(Debug)]
pub enum Term<'a> {
    Int(i64),
    Float(f64),
    String(&'a str),
    Regex(&'a str),
    Variable(&'a str),
    LoadedTerm(i64),
    Not(Box<Term<'a>>),
    Eq(Box<Term<'a>>, Box<Term<'a>>),
    Or(Box<Term<'a>>, Box<Term<'a>>),
    And(Box<Term<'a>>, Box<Term<'a>>),
    Lt(Box<Term<'a>>, Box<Term<'a>>),
    Le(Box<Term<'a>>, Box<Term<'a>>),
    Gt(Box<Term<'a>>, Box<Term<'a>>),
    Ge(Box<Term<'a>>, Box<Term<'a>>),
}
