use nom::IResult;
use nom::branch::alt;
use nom::combinator::map;
use nom::error::{Error, ErrorKind};

use crate::ast::{BinOp, Literal};
use crate::lexer::Token;

/// Helper function to match a specific token
pub(crate) fn token(expected: Token) -> impl Fn(&[Token]) -> IResult<&[Token], Token> {
    move |input: &[Token]| {
        if input.is_empty() {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Eof)));
        }
        if input[0] == expected {
            Ok((&input[1..], expected.clone()))
        } else {
            Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)))
        }
    }
}

/// Parse an identifier
pub(crate) fn parse_identifier(input: &[Token]) -> IResult<&[Token], String> {
    if input.is_empty() {
        return Err(nom::Err::Error(Error::new(input, ErrorKind::Eof)));
    }
    match &input[0] {
        Token::Identifier(name) => Ok((&input[1..], name.clone())),
        _ => Err(nom::Err::Error(Error::new(input, ErrorKind::Tag))),
    }
}

/// Parse a literal
pub(crate) fn parse_literal(input: &[Token]) -> IResult<&[Token], Literal> {
    if input.is_empty() {
        return Err(nom::Err::Error(Error::new(input, ErrorKind::Eof)));
    }
    match &input[0] {
        Token::IntLiteral(n) => Ok((&input[1..], Literal::Int(*n))),
        Token::FloatLiteral(f) => Ok((&input[1..], Literal::Float(*f))),
        Token::StringLiteral(s) => Ok((&input[1..], Literal::String(s.clone()))),
        _ => Err(nom::Err::Error(Error::new(input, ErrorKind::Tag))),
    }
}

/// Parse a binary operator
pub(crate) fn parse_binop(input: &[Token]) -> IResult<&[Token], BinOp> {
    alt((
        map(token(Token::Plus), |_| BinOp::Plus),
        map(token(Token::Minus), |_| BinOp::Minus),
        map(token(Token::Multiply), |_| BinOp::Multiply),
        map(token(Token::Divide), |_| BinOp::Divide),
        map(token(Token::Percent), |_| BinOp::Modulo),
        map(token(Token::Equal), |_| BinOp::Equal),
        map(token(Token::NotEqual), |_| BinOp::NotEqual),
        map(token(Token::LessThan), |_| BinOp::LessThan),
        map(token(Token::GreaterThan), |_| BinOp::GreaterThan),
        map(token(Token::LessEqual), |_| BinOp::LessEqual),
        map(token(Token::GreaterEqual), |_| BinOp::GreaterEqual),
    ))(input)
}
