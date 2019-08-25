use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while, take_while1},
    combinator::map,
    number::complete::double,
    //          multi::{many1, separated_nonempty_list},
    //          ParseTo
    IResult,
};

use crate::term::*;

enum Operator0 {
    And,
    Or,
}

enum Operator1 {
    Eq,
    NEq,
    Gt,
    Ge,
    Lt,
    Le,
}

fn w_string(input: &str) -> IResult<&str, Term> {
    let (input, _) = tag("\"")(input)?;
    let (input, s) = take_until("\"")(input)?;
    let (input, _) = tag("\"")(input)?;
    Ok((input, Term::String(s)))
}

fn w_float(input: &str) -> IResult<&str, Term> {
    let (input, n) = double(input)?;
    Ok((input, Term::Float(n)))
}

fn w_operator0(input: &str) -> IResult<&str, Operator0> {
    alt((
        map(tag("and"), |_| Operator0::And),
        map(tag("or"), |_| Operator0::Or),
    ))(input)
}

fn w_operator1(input: &str) -> IResult<&str, Operator1> {
    alt((
        map(tag("=="), |_| Operator1::Eq),
        map(tag("!="), |_| Operator1::NEq),
    ))(input)
}

fn w_op0(input: &str) -> IResult<&str, Term> {
    let (input, left) = w_op1(input)?;
    let (input, _) = take_while(|c: char| c.is_whitespace())(input)?;
    let (input, operator) = w_operator0(input)?;
    let (input, _) = take_while(|c: char| c.is_whitespace())(input)?;
    let (input, right) = w_term(input)?;
    let bl = Box::new(left);
    let br = Box::new(right);
    match operator {
        Operator0::And => Ok((input, Term::And(bl, br))),
        Operator0::Or => Ok((input, Term::Or(bl, br))),
    }
}

fn w_op1(input: &str) -> IResult<&str, Term> {
    let (input, left) = w_value(input)?;
    let (input, _) = take_while(|c: char| c.is_whitespace())(input)?;
    let (input, operator) = w_operator1(input)?;
    let (input, _) = take_while(|c: char| c.is_whitespace())(input)?;
    let (input, right) = w_term(input)?;
    let bl = Box::new(left);
    let br = Box::new(right);
    Ok((
        input,
        match operator {
            Operator1::Eq => Term::Eq(bl, br),
            Operator1::NEq => Term::Not(Box::new(Term::Eq(bl, br))),
            Operator1::Lt => Term::Lt(bl, br),
            Operator1::Le => Term::Le(bl, br),
            Operator1::Gt => Term::Gt(bl, br),
            Operator1::Ge => Term::Ge(bl, br),
        },
    ))
}

fn w_identifier(input: &str) -> IResult<&str, Term> {
    let (input, ident) = take_while1(|c: char| c.is_ascii_alphanumeric())(input)?;
    Ok((input, Term::Variable(ident)))
}

fn w_regex(input: &str) -> IResult<&str, Term> {
    let (input, _) = tag("/")(input)?;
    let (input, re_str) = take_while1(|c: char| c != '/')(input)?;
    let (input, _) = tag("/")(input)?;
    Ok((input, Term::Regex(re_str)))
}

fn w_value(input: &str) -> IResult<&str, Term> {
    alt((w_regex, w_string, w_float, w_identifier))(input)
}

fn w_term(input: &str) -> IResult<&str, Term> {
    alt((w_op0, w_op1, w_value))(input)
}

/// Parse a formula string, return an AST that can subsequently be
/// passed to a call to Context::new().compile()
/// The same AST can be reused for compilation with
/// multiple contexts.
pub fn parse(input: &str) -> IResult<&str, Term> {
    w_term(input)
}
