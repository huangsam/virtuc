use nom::IResult;
use nom::branch::alt;
use nom::combinator::map;
use nom::error::{Error, ErrorKind};
use nom::multi::many0;
use nom::sequence::{delimited, tuple};

use super::helpers::{parse_identifier, token};
use crate::ast::{ExternFunction, StructDef, StructField, Type};
use crate::lexer::Token;

/// Parse a type: (int | float | string | void | struct id) followed by zero or more '*'
pub(crate) fn parse_type(input: &[Token]) -> IResult<&[Token], Type> {
    let (input, base_ty) = alt((
        map(token(Token::Int), |_| Type::Int),
        map(token(Token::Float), |_| Type::Float),
        map(token(Token::StringType), |_| Type::String),
        map(token(Token::Void), |_| Type::Void),
        map(
            tuple((token(Token::Struct), parse_identifier)),
            |(_, name)| Type::Struct(name),
        ),
    ))(input)?;

    let (input, stars) = many0(token(Token::Multiply))(input)?;
    let mut ty = base_ty;
    for _ in stars {
        ty = Type::Pointer(Box::new(ty));
    }
    Ok((input, ty))
}

/// Parse a struct field: type identifier ;
fn parse_struct_field(input: &[Token]) -> IResult<&[Token], StructField> {
    let (input, ty) = parse_type(input)?;
    let (input, name) = parse_identifier(input)?;
    let (input, _) = token(Token::Semicolon)(input)?;
    Ok((input, StructField { ty, name }))
}

/// Parse a struct definition: struct identifier { fields* } ;
pub(crate) fn parse_struct_def(input: &[Token]) -> IResult<&[Token], StructDef> {
    let (input, _) = token(Token::Struct)(input)?;
    let (input, name) = parse_identifier(input)?;
    let (input, fields) = delimited(
        token(Token::LBrace),
        many0(parse_struct_field),
        token(Token::RBrace),
    )(input)?;
    let (input, _) = token(Token::Semicolon)(input)?;
    Ok((input, StructDef { name, fields }))
}

/// Parse a function parameter: type identifier
pub(crate) fn parse_param(input: &[Token]) -> IResult<&[Token], (Type, String)> {
    tuple((parse_type, parse_identifier))(input)
}

fn parse_extern_param_list(input: &[Token]) -> IResult<&[Token], (Vec<Type>, bool)> {
    let mut types = vec![];
    let mut input = input;
    loop {
        if let Ok((rest, ty)) = parse_type(input) {
            types.push(ty);
            input = rest;
            if let Some(&Token::Comma) = input.first() {
                input = &input[1..];
                // continue
            } else {
                return Ok((input, (types, false)));
            }
        } else if let Some(&Token::Ellipsis) = input.first() {
            return Ok((&input[1..], (types, true)));
        } else {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }
    }
}

/// Parse an extern function: extern type identifier(types ...); or extern type identifier(types);
pub(crate) fn parse_extern_function(input: &[Token]) -> IResult<&[Token], ExternFunction> {
    map(
        tuple((
            token(Token::Extern),
            parse_type,
            parse_identifier,
            token(Token::LParen),
            parse_extern_param_list,
            token(Token::RParen),
            token(Token::Semicolon),
        )),
        |(_, return_ty, name, _, (param_types, is_variadic), _, _)| ExternFunction {
            return_ty,
            name,
            param_types,
            is_variadic,
        },
    )(input)
}
