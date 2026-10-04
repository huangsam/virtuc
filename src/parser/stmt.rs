use nom::IResult;
use nom::branch::alt;
use nom::combinator::{map, opt};
use nom::multi::{many0, many1};
use nom::sequence::{delimited, preceded, terminated, tuple};

use super::expr::parse_expr;
use super::helpers::{parse_identifier, parse_literal, token};
use super::types::{parse_struct_def, parse_type};
use crate::ast::*;
use crate::lexer::Token;

/// Parse an array declaration: `type identifier [ int_literal ]+ ;`
fn parse_array_declaration(input: &[Token]) -> IResult<&[Token], Stmt> {
    let (input, ty) = parse_type(input)?;
    let (input, name) = parse_identifier(input)?;
    let (input, size_lits) = many1(delimited(
        token(Token::LBracket),
        parse_literal,
        token(Token::RBracket),
    ))(input)?;
    let mut dims = Vec::with_capacity(size_lits.len());
    for size_lit in size_lits {
        match size_lit {
            Literal::Int(n) if n > 0 => dims.push(n as usize),
            _ => {
                return Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Digit,
                )));
            }
        }
    }
    let (input, _) = token(Token::Semicolon)(input)?;
    Ok((input, Stmt::ArrayDeclaration { ty, name, dims }))
}

/// Parse a declaration: type identifier (= expr)? ;
fn parse_declaration(input: &[Token]) -> IResult<&[Token], Stmt> {
    map(
        tuple((
            parse_type,
            parse_identifier,
            opt(preceded(token(Token::Assign), parse_expr)),
            token(Token::Semicolon),
        )),
        |(ty, name, init, _)| Stmt::Declaration { ty, name, init },
    )(input)
}

/// Parse a return statement: return expr? ;
fn parse_return(input: &[Token]) -> IResult<&[Token], Stmt> {
    map(
        tuple((
            token(Token::Return),
            opt(parse_expr),
            token(Token::Semicolon),
        )),
        |(_, expr, _)| Stmt::Return(expr),
    )(input)
}

/// Parse a block: { statements }
pub(crate) fn parse_block(input: &[Token]) -> IResult<&[Token], Stmt> {
    map(
        delimited(
            token(Token::LBrace),
            many0(parse_stmt),
            token(Token::RBrace),
        ),
        Stmt::Block,
    )(input)
}

/// Parse an if statement: if (expr) stmt (else stmt)?
fn parse_if(input: &[Token]) -> IResult<&[Token], Stmt> {
    map(
        tuple((
            token(Token::If),
            delimited(token(Token::LParen), parse_expr, token(Token::RParen)),
            parse_stmt,
            opt(preceded(token(Token::Else), parse_stmt)),
        )),
        |(_, cond, then, else_)| Stmt::If {
            cond,
            then: Box::new(then),
            else_: else_.map(Box::new),
        },
    )(input)
}

/// Parse a for loop: for (init? ; cond? ; update?) stmt
/// All three components (init, cond, update) are optional according to C syntax.
/// - init: Can be a declaration (int i = 0) or expression (i = 0)
/// - cond: Condition checked before each iteration
/// - update: Expression evaluated at end of each iteration
fn parse_for(input: &[Token]) -> IResult<&[Token], Stmt> {
    map(
        tuple((
            token(Token::For),
            delimited(
                token(Token::LParen),
                tuple((
                    alt((
                        map(parse_array_declaration, |s| Some(Box::new(s))),
                        map(parse_declaration, |s| Some(Box::new(s))),
                        map(parse_expr_stmt, |s| Some(Box::new(s))),
                        map(token(Token::Semicolon), |_| None),
                    )),
                    map(
                        tuple((opt(parse_expr), token(Token::Semicolon))),
                        |(cond, _)| cond,
                    ),
                    opt(parse_expr),
                )),
                token(Token::RParen),
            ),
            parse_stmt,
        )),
        |(_, (init, cond, update), body)| Stmt::For {
            init,
            cond,
            update,
            body: Box::new(body),
        },
    )(input)
}

/// Parse a while loop: while (expr) stmt
fn parse_while(input: &[Token]) -> IResult<&[Token], Stmt> {
    map(
        tuple((
            token(Token::While),
            delimited(token(Token::LParen), parse_expr, token(Token::RParen)),
            parse_stmt,
        )),
        |(_, cond, body)| Stmt::While {
            cond,
            body: Box::new(body),
        },
    )(input)
}

/// Parse a break statement: break ;
fn parse_break(input: &[Token]) -> IResult<&[Token], Stmt> {
    map(
        tuple((token(Token::Break), token(Token::Semicolon))),
        |_| Stmt::Break,
    )(input)
}

/// Parse a continue statement: continue ;
fn parse_continue(input: &[Token]) -> IResult<&[Token], Stmt> {
    map(
        tuple((token(Token::Continue), token(Token::Semicolon))),
        |_| Stmt::Continue,
    )(input)
}

/// Parse an expression statement: expr ;
fn parse_expr_stmt(input: &[Token]) -> IResult<&[Token], Stmt> {
    map(terminated(parse_expr, token(Token::Semicolon)), Stmt::Expr)(input)
}

/// Parse a statement
pub(crate) fn parse_stmt(input: &[Token]) -> IResult<&[Token], Stmt> {
    alt((
        map(parse_struct_def, Stmt::StructDef),
        parse_array_declaration,
        parse_declaration,
        parse_return,
        parse_if,
        parse_for,
        parse_while,
        parse_break,
        parse_continue,
        parse_block,
        parse_expr_stmt,
    ))(input)
}
