//! # Syntax Analysis and AST Construction
//!
//! This module implements the parser that converts a stream of tokens into
//! an Abstract Syntax Tree (AST). It uses the `nom` parser combinator library
//! for building the parsing logic.
//!
//! ## Grammar
//!
//! The parser handles the C subset grammar including:
//! - Expressions: arithmetic, comparison, assignment
//! - Statements: variable declarations, assignments, control flow
//! - Functions: declarations and definitions
//! - Control structures: if-else, for loops
//!
//! ## Parser Combinators
//!
//! Uses `nom`'s combinator approach to build modular parsers for each
//! grammar rule. Provides good error messages and recovery for syntax errors.

use nom::{
    IResult,
    branch::alt,
    combinator::{map, opt},
    error::{Error, ErrorKind},
    multi::{many0, separated_list0},
    sequence::{delimited, preceded, terminated, tuple},
};

use crate::ast::*;
use crate::lexer::Token;

#[derive(Debug, PartialEq, Clone)]
enum TopLevel {
    Include(String),
    Extern(ExternFunction),
    Function(Function),
}

/// Helper function to match a specific token
fn token(expected: Token) -> impl Fn(&[Token]) -> IResult<&[Token], Token> {
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

/// Parse a type: int | float | string
fn parse_type(input: &[Token]) -> IResult<&[Token], Type> {
    alt((
        map(token(Token::Int), |_| Type::Int),
        map(token(Token::Float), |_| Type::Float),
        map(token(Token::StringType), |_| Type::String),
    ))(input)
}

/// Parse an identifier
fn parse_identifier(input: &[Token]) -> IResult<&[Token], String> {
    if input.is_empty() {
        return Err(nom::Err::Error(Error::new(input, ErrorKind::Eof)));
    }
    match &input[0] {
        Token::Identifier(name) => Ok((&input[1..], name.clone())),
        _ => Err(nom::Err::Error(Error::new(input, ErrorKind::Tag))),
    }
}

/// Parse a literal
fn parse_literal(input: &[Token]) -> IResult<&[Token], Literal> {
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
fn parse_binop(input: &[Token]) -> IResult<&[Token], BinOp> {
    alt((
        map(token(Token::Plus), |_| BinOp::Plus),
        map(token(Token::Minus), |_| BinOp::Minus),
        map(token(Token::Multiply), |_| BinOp::Multiply),
        map(token(Token::Divide), |_| BinOp::Divide),
        map(token(Token::Equal), |_| BinOp::Equal),
        map(token(Token::NotEqual), |_| BinOp::NotEqual),
        map(token(Token::LessThan), |_| BinOp::LessThan),
        map(token(Token::GreaterThan), |_| BinOp::GreaterThan),
        map(token(Token::LessEqual), |_| BinOp::LessEqual),
        map(token(Token::GreaterEqual), |_| BinOp::GreaterEqual),
    ))(input)
}

/// Parse a primary expression: literal | identifier | (expr) | call
fn parse_primary_expr(input: &[Token]) -> IResult<&[Token], Expr> {
    alt((
        map(parse_literal, Expr::Literal),
        parse_call,
        map(parse_identifier, Expr::Identifier),
        delimited(token(Token::LParen), parse_expr, token(Token::RParen)),
    ))(input)
}

/// Parse a function call: identifier(args)
fn parse_call(input: &[Token]) -> IResult<&[Token], Expr> {
    map(
        tuple((
            parse_identifier,
            delimited(
                token(Token::LParen),
                separated_list0(token(Token::Comma), parse_expr),
                token(Token::RParen),
            ),
        )),
        |(name, args)| Expr::Call { name, args },
    )(input)
}

/// Parse multiplicative expression: primary (*|/ primary)*
fn parse_multiplicative(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, mut expr) = parse_primary_expr(input)?;
    let mut input = input;
    loop {
        let result = opt(tuple((
            alt((token(Token::Multiply), token(Token::Divide))),
            parse_primary_expr,
        )))(input)?;
        if let Some((op_token, right)) = result.1 {
            let op = match op_token {
                Token::Multiply => BinOp::Multiply,
                Token::Divide => BinOp::Divide,
                _ => unreachable!(),
            };
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
            input = result.0;
        } else {
            break;
        }
    }
    Ok((input, expr))
}

/// Parse additive expression: multiplicative (+|- multiplicative)*
fn parse_additive(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, mut expr) = parse_multiplicative(input)?;
    let mut input = input;
    loop {
        let result = opt(tuple((
            alt((token(Token::Plus), token(Token::Minus))),
            parse_multiplicative,
        )))(input)?;
        if let Some((op_token, right)) = result.1 {
            let op = match op_token {
                Token::Plus => BinOp::Plus,
                Token::Minus => BinOp::Minus,
                _ => unreachable!(),
            };
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
            input = result.0;
        } else {
            break;
        }
    }
    Ok((input, expr))
}

/// Parse comparison expression: additive (==|!=|<|>|<=|>= additive)*
fn parse_comparison(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, mut expr) = parse_additive(input)?;
    let mut input = input;
    let result = opt(tuple((parse_binop, parse_additive)))(input)?;
    if let Some((op, right)) = result.1 {
        expr = Expr::Binary {
            left: Box::new(expr),
            op,
            right: Box::new(right),
        };
        input = result.0;
    }
    Ok((input, expr))
}

/// Parse an assignment expression: identifier = expr
fn parse_assignment_expr(input: &[Token]) -> IResult<&[Token], Expr> {
    alt((
        map(
            tuple((parse_identifier, token(Token::Assign), parse_expr)),
            |(name, _, value)| Expr::Assignment {
                name,
                value: Box::new(value),
            },
        ),
        parse_comparison,
    ))(input)
}

/// Parse expression (top level)
fn parse_expr(input: &[Token]) -> IResult<&[Token], Expr> {
    parse_assignment_expr(input)
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
fn parse_block(input: &[Token]) -> IResult<&[Token], Stmt> {
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
fn parse_for(input: &[Token]) -> IResult<&[Token], Stmt> {
    map(
        tuple((
            token(Token::For),
            delimited(
                token(Token::LParen),
                tuple((
                    alt((
                        map(parse_declaration, |s| Some(Box::new(s))),
                        map(parse_expr_stmt, |s| Some(Box::new(s))),
                        map(token(Token::Semicolon), |_| None),
                    )),
                    opt(terminated(parse_expr, token(Token::Semicolon))),
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

/// Parse an expression statement: expr ;
fn parse_expr_stmt(input: &[Token]) -> IResult<&[Token], Stmt> {
    map(terminated(parse_expr, token(Token::Semicolon)), Stmt::Expr)(input)
}

/// Parse a statement
fn parse_stmt(input: &[Token]) -> IResult<&[Token], Stmt> {
    alt((
        parse_declaration,
        parse_return,
        parse_if,
        parse_for,
        parse_block,
        parse_expr_stmt,
    ))(input)
}

/// Parse a function parameter: type identifier
fn parse_param(input: &[Token]) -> IResult<&[Token], (Type, String)> {
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
fn parse_extern_function(input: &[Token]) -> IResult<&[Token], ExternFunction> {
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

/// Parse an include directive token and return header name
fn parse_include(input: &[Token]) -> IResult<&[Token], String> {
    if input.is_empty() {
        return Err(nom::Err::Error(Error::new(input, ErrorKind::Eof)));
    }
    match &input[0] {
        Token::Include(name) => Ok((&input[1..], name.clone())),
        _ => Err(nom::Err::Error(Error::new(input, ErrorKind::Tag))),
    }
}

/// Parse a top-level item: include, extern function or function definition
fn parse_top_level(input: &[Token]) -> IResult<&[Token], TopLevel> {
    alt((
        map(parse_include, TopLevel::Include),
        map(parse_extern_function, TopLevel::Extern),
        map(parse_function, TopLevel::Function),
    ))(input)
}

/// Parse a function: type identifier(params) { body }
fn parse_function(input: &[Token]) -> IResult<&[Token], Function> {
    map(
        tuple((
            parse_type,
            parse_identifier,
            delimited(
                token(Token::LParen),
                separated_list0(token(Token::Comma), parse_param),
                token(Token::RParen),
            ),
            parse_block,
        )),
        |(return_ty, name, params, body)| Function {
            return_ty,
            name,
            params,
            body,
        },
    )(input)
}

/// Parse the program: extern functions and functions
pub fn parse(tokens: &[Token]) -> Result<Program, String> {
    let (remaining, items) =
        many0(parse_top_level)(tokens).map_err(|e| format!("Parse error: {:?}", e))?;
    if !remaining.is_empty() {
        return Err(format!("Unexpected tokens at end: {:?}", remaining));
    }
    let mut includes = Vec::new();
    let mut extern_functions = Vec::new();
    let mut functions = Vec::new();
    for item in items {
        match item {
            TopLevel::Include(h) => includes.push(h),
            TopLevel::Extern(e) => extern_functions.push(e),
            TopLevel::Function(f) => functions.push(f),
        }
    }

    // Map includes to externs using registry
    for header in &includes {
        for ext in crate::header_registry::externs_for_header(header) {
            if !extern_functions.iter().any(|e| e.name == ext.name) {
                extern_functions.push(ext);
            }
        }
    }

    Ok(Program {
        includes,
        extern_functions,
        functions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    #[test]
    fn test_parse_function() {
        let tokens =
            lex("int add(int a, int b) { return a + b; } int main() { return 0; }").unwrap();
        let ast = parse(&tokens).unwrap();
        assert_eq!(ast.functions.len(), 2);
        let func = &ast.functions[0];
        assert_eq!(func.name, "add");
        assert_eq!(func.return_ty, Type::Int);
        assert_eq!(
            func.params,
            vec![(Type::Int, "a".to_string()), (Type::Int, "b".to_string())]
        );
        // Check body
        if let Stmt::Block(stmts) = &func.body {
            assert_eq!(stmts.len(), 1);
            if let Stmt::Return(Some(Expr::Binary { left, op, right })) = &stmts[0] {
                assert_eq!(**left, Expr::Identifier("a".to_string()));
                assert_eq!(*op, BinOp::Plus);
                assert_eq!(**right, Expr::Identifier("b".to_string()));
            } else {
                panic!("Expected return a + b");
            }
        } else {
            panic!("Expected block");
        }
        let func2 = &ast.functions[1];
        assert_eq!(func2.name, "main");
        assert_eq!(func2.return_ty, Type::Int);
        assert_eq!(func2.params, vec![]);
    }

    #[test]
    fn test_parse_extern_function() {
        let tokens = lex("extern int printf(int, int); int main() { return 0; }").unwrap();
        let ast = parse(&tokens).unwrap();
        assert_eq!(ast.extern_functions.len(), 1);
        assert_eq!(ast.functions.len(), 1);
        let extern_func = &ast.extern_functions[0];
        assert_eq!(extern_func.name, "printf");
        assert_eq!(extern_func.return_ty, Type::Int);
        assert_eq!(extern_func.param_types, vec![Type::Int, Type::Int]);
        assert_eq!(extern_func.is_variadic, false);
    }

    #[test]
    fn test_parse_extern_function_variadic() {
        let tokens = lex("extern int printf(int, ...); int main() { return 0; }").unwrap();
        let ast = parse(&tokens).unwrap();
        assert_eq!(ast.extern_functions.len(), 1);
        let extern_func = &ast.extern_functions[0];
        assert_eq!(extern_func.name, "printf");
        assert_eq!(extern_func.param_types, vec![Type::Int]);
        assert_eq!(extern_func.is_variadic, true);
    }

    #[test]
    fn test_parse_include() {
        let tokens = lex("#include <stdio.h> int main() { return 0; }").unwrap();
        let ast = parse(&tokens).unwrap();
        assert_eq!(ast.includes.len(), 1);
        assert_eq!(ast.includes[0], "stdio.h");
    }
}
