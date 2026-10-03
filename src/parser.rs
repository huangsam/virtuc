//! # Syntax Analysis and AST Construction
//!
//! This module implements the parser that converts a stream of tokens into
//! an Abstract Syntax Tree (AST). It uses the `nom` parser combinator library
//! for building the parsing logic.
//!
//! ## Grammar
//!
//! The parser handles the C subset grammar including:
//! - Expressions: arithmetic, comparison, logical, assignment, calls, array indexing, and pointer dereferencing
//! - Statements: variable/array declarations, assignments, returns, blocks, break/continue
//! - Functions: declarations (`extern`) and definitions
//! - Control structures: `if`-`else`, `for` loops, `while` loops
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
    multi::{many0, many1, separated_list0},
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

/// Parse a type: (int | float | string | void) followed by zero or more '*'
fn parse_type(input: &[Token]) -> IResult<&[Token], Type> {
    let (input, base_ty) = alt((
        map(token(Token::Int), |_| Type::Int),
        map(token(Token::Float), |_| Type::Float),
        map(token(Token::StringType), |_| Type::String),
        map(token(Token::Void), |_| Type::Void),
    ))(input)?;

    let (input, stars) = many0(token(Token::Multiply))(input)?;
    let mut ty = base_ty;
    for _ in stars {
        ty = Type::Pointer(Box::new(ty));
    }
    Ok((input, ty))
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
        map(token(Token::Percent), |_| BinOp::Modulo),
        map(token(Token::Equal), |_| BinOp::Equal),
        map(token(Token::NotEqual), |_| BinOp::NotEqual),
        map(token(Token::LessThan), |_| BinOp::LessThan),
        map(token(Token::GreaterThan), |_| BinOp::GreaterThan),
        map(token(Token::LessEqual), |_| BinOp::LessEqual),
        map(token(Token::GreaterEqual), |_| BinOp::GreaterEqual),
    ))(input)
}

/// Parse postfix expression: identifier++ | identifier--
fn parse_postfix(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, name) = parse_identifier(input)?;
    let (input, op_token) = alt((token(Token::PlusPlus), token(Token::MinusMinus)))(input)?;
    let op = match op_token {
        Token::PlusPlus => BinOp::Plus,
        Token::MinusMinus => BinOp::Minus,
        _ => unreachable!(),
    };
    Ok((
        input,
        Expr::Assignment {
            name: name.clone(),
            value: Box::new(Expr::Binary {
                left: Box::new(Expr::Identifier(name)),
                op,
                right: Box::new(Expr::Literal(Literal::Int(1))),
            }),
        },
    ))
}

/// Parse an array index expression: `identifier [ expr ]+`
fn parse_index(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, name) = parse_identifier(input)?;
    let (input, indices) = many1(delimited(
        token(Token::LBracket),
        parse_expr,
        token(Token::RBracket),
    ))(input)?;
    Ok((input, Expr::Index { name, indices }))
}

/// Parse array postfix: `identifier [ expr ]+ (++|--)`
fn parse_array_postfix(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, name) = parse_identifier(input)?;
    let (input, indices) = many1(delimited(
        token(Token::LBracket),
        parse_expr,
        token(Token::RBracket),
    ))(input)?;
    let (input, op_token) = alt((token(Token::PlusPlus), token(Token::MinusMinus)))(input)?;
    let op = match op_token {
        Token::PlusPlus => BinOp::Plus,
        Token::MinusMinus => BinOp::Minus,
        _ => unreachable!(),
    };
    Ok((
        input,
        Expr::IndexAssignment {
            name: name.clone(),
            indices: indices.clone(),
            value: Box::new(Expr::Binary {
                left: Box::new(Expr::Index { name, indices }),
                op,
                right: Box::new(Expr::Literal(Literal::Int(1))),
            }),
        },
    ))
}

/// Parse a primary expression: literal | identifier | (expr) | call | postfix | index
fn parse_primary_expr(input: &[Token]) -> IResult<&[Token], Expr> {
    alt((
        map(parse_literal, Expr::Literal),
        parse_call,
        parse_array_postfix,
        parse_index,
        parse_postfix,
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

/// Parse prefix inc/dec: `++id` | `--id` | `++id[expr]` | `--id[expr]` | `++*unary` | `--*unary`
fn parse_prefix_inc_dec(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, op_token) = alt((token(Token::PlusPlus), token(Token::MinusMinus)))(input)?;
    let op = match op_token {
        Token::PlusPlus => BinOp::Plus,
        Token::MinusMinus => BinOp::Minus,
        _ => unreachable!(),
    };
    if let Ok((rest, (_, target))) = tuple((token(Token::Multiply), parse_unary))(input) {
        return Ok((
            rest,
            Expr::DerefAssignment {
                target: Box::new(target.clone()),
                value: Box::new(Expr::Binary {
                    left: Box::new(Expr::Unary {
                        op: UnaryOp::Deref,
                        expr: Box::new(target),
                    }),
                    op,
                    right: Box::new(Expr::Literal(Literal::Int(1))),
                }),
            },
        ));
    }
    if let Ok((rest, (name, indices))) = tuple((
        parse_identifier,
        many1(delimited(
            token(Token::LBracket),
            parse_expr,
            token(Token::RBracket),
        )),
    ))(input)
    {
        return Ok((
            rest,
            Expr::IndexAssignment {
                name: name.clone(),
                indices: indices.clone(),
                value: Box::new(Expr::Binary {
                    left: Box::new(Expr::Index { name, indices }),
                    op,
                    right: Box::new(Expr::Literal(Literal::Int(1))),
                }),
            },
        ));
    }
    let (input, name) = parse_identifier(input)?;
    Ok((
        input,
        Expr::Assignment {
            name: name.clone(),
            value: Box::new(Expr::Binary {
                left: Box::new(Expr::Identifier(name)),
                op,
                right: Box::new(Expr::Literal(Literal::Int(1))),
            }),
        },
    ))
}

/// Parse unary expression: (-|!|&|*) unary | (++|--) identifier | primary
fn parse_unary(input: &[Token]) -> IResult<&[Token], Expr> {
    alt((
        parse_prefix_inc_dec,
        map(
            tuple((
                alt((
                    token(Token::Minus),
                    token(Token::Bang),
                    token(Token::Ampersand),
                    token(Token::Multiply),
                )),
                parse_unary,
            )),
            |(op_token, expr)| {
                let op = match op_token {
                    Token::Minus => UnaryOp::Neg,
                    Token::Bang => UnaryOp::Not,
                    Token::Ampersand => UnaryOp::AddrOf,
                    Token::Multiply => UnaryOp::Deref,
                    _ => unreachable!(),
                };
                Expr::Unary {
                    op,
                    expr: Box::new(expr),
                }
            },
        ),
        parse_primary_expr,
    ))(input)
}

/// Parse multiplicative expression: unary (*|/|% unary)*
/// Implements left-associative parsing for *, /, and % operators.
/// Higher precedence than addition, so parses before additive.
fn parse_multiplicative(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, mut expr) = parse_unary(input)?;
    let mut input = input;
    // Loop to handle left-associative chaining: a * b % c -> ((a * b) % c)
    loop {
        let result = opt(tuple((
            alt((
                token(Token::Multiply),
                token(Token::Divide),
                token(Token::Percent),
            )),
            parse_unary,
        )))(input)?;
        if let Some((op_token, right)) = result.1 {
            let op = match op_token {
                Token::Multiply => BinOp::Multiply,
                Token::Divide => BinOp::Divide,
                Token::Percent => BinOp::Modulo,
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
/// Implements left-associative parsing for + and - operators.
/// Lower precedence than multiplication, so these parse after multiplicative.
fn parse_additive(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, mut expr) = parse_multiplicative(input)?;
    let mut input = input;
    // Loop to handle left-associative chaining: a + b - c -> ((a + b) - c)
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
            // Left-associate: wrap left side with new operator
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
/// Handles comparison operators with lowest precedence.
/// Unlike +/-, comparisons are non-associative (a < b < c is not allowed in C).
fn parse_comparison(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, mut expr) = parse_additive(input)?;
    let mut input = input;
    // Attempt to parse one comparison operator and right operand
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

/// Parse logical AND expression: comparison (&& comparison)*
fn parse_logical_and(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, mut expr) = parse_comparison(input)?;
    let mut input = input;
    loop {
        let result = opt(tuple((token(Token::LogicalAnd), parse_comparison)))(input)?;
        if let Some((_, right)) = result.1 {
            expr = Expr::LogicalAnd {
                left: Box::new(expr),
                right: Box::new(right),
            };
            input = result.0;
        } else {
            break;
        }
    }
    Ok((input, expr))
}

/// Parse logical OR expression: logical_and (|| logical_and)*
fn parse_logical_or(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, mut expr) = parse_logical_and(input)?;
    let mut input = input;
    loop {
        let result = opt(tuple((token(Token::LogicalOr), parse_logical_and)))(input)?;
        if let Some((_, right)) = result.1 {
            expr = Expr::LogicalOr {
                left: Box::new(expr),
                right: Box::new(right),
            };
            input = result.0;
        } else {
            break;
        }
    }
    Ok((input, expr))
}

/// Parse array assignment: `identifier [ expr ]+ (=|+=|-=|*=|/=|%=) expr`
fn parse_array_assignment(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, name) = parse_identifier(input)?;
    let (input, indices) = many1(delimited(
        token(Token::LBracket),
        parse_expr,
        token(Token::RBracket),
    ))(input)?;
    let (input, op_token) = alt((
        token(Token::Assign),
        token(Token::PlusAssign),
        token(Token::MinusAssign),
        token(Token::MultiplyAssign),
        token(Token::DivideAssign),
        token(Token::ModuloAssign),
    ))(input)?;
    let (input, value) = parse_expr(input)?;
    match op_token {
        Token::Assign => Ok((
            input,
            Expr::IndexAssignment {
                name,
                indices,
                value: Box::new(value),
            },
        )),
        op => {
            let bin_op = match op {
                Token::PlusAssign => BinOp::Plus,
                Token::MinusAssign => BinOp::Minus,
                Token::MultiplyAssign => BinOp::Multiply,
                Token::DivideAssign => BinOp::Divide,
                Token::ModuloAssign => BinOp::Modulo,
                _ => unreachable!(),
            };
            Ok((
                input,
                Expr::IndexAssignment {
                    name: name.clone(),
                    indices: indices.clone(),
                    value: Box::new(Expr::Binary {
                        left: Box::new(Expr::Index { name, indices }),
                        op: bin_op,
                        right: Box::new(value),
                    }),
                },
            ))
        }
    }
}

/// Parse dereference assignment: *unary (=|+=|-=|*=|/=|%=) expr
fn parse_deref_assignment(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, _) = token(Token::Multiply)(input)?;
    let (input, target) = parse_unary(input)?;
    let (input, op_token) = alt((
        token(Token::Assign),
        token(Token::PlusAssign),
        token(Token::MinusAssign),
        token(Token::MultiplyAssign),
        token(Token::DivideAssign),
        token(Token::ModuloAssign),
    ))(input)?;
    let (input, value) = parse_expr(input)?;
    match op_token {
        Token::Assign => Ok((
            input,
            Expr::DerefAssignment {
                target: Box::new(target),
                value: Box::new(value),
            },
        )),
        op => {
            let bin_op = match op {
                Token::PlusAssign => BinOp::Plus,
                Token::MinusAssign => BinOp::Minus,
                Token::MultiplyAssign => BinOp::Multiply,
                Token::DivideAssign => BinOp::Divide,
                Token::ModuloAssign => BinOp::Modulo,
                _ => unreachable!(),
            };
            Ok((
                input,
                Expr::DerefAssignment {
                    target: Box::new(target.clone()),
                    value: Box::new(Expr::Binary {
                        left: Box::new(Expr::Unary {
                            op: UnaryOp::Deref,
                            expr: Box::new(target),
                        }),
                        op: bin_op,
                        right: Box::new(value),
                    }),
                },
            ))
        }
    }
}

/// Parse an assignment expression: `identifier (=|...) expr` | `identifier [ expr ] (=|...) expr` | `*unary (=|...) expr`
fn parse_assignment_expr(input: &[Token]) -> IResult<&[Token], Expr> {
    alt((
        parse_deref_assignment,
        parse_array_assignment,
        map(
            tuple((parse_identifier, token(Token::Assign), parse_expr)),
            |(name, _, value)| Expr::Assignment {
                name,
                value: Box::new(value),
            },
        ),
        map(
            tuple((
                parse_identifier,
                alt((
                    token(Token::PlusAssign),
                    token(Token::MinusAssign),
                    token(Token::MultiplyAssign),
                    token(Token::DivideAssign),
                    token(Token::ModuloAssign),
                )),
                parse_expr,
            )),
            |(name, op_token, value)| {
                let op = match op_token {
                    Token::PlusAssign => BinOp::Plus,
                    Token::MinusAssign => BinOp::Minus,
                    Token::MultiplyAssign => BinOp::Multiply,
                    Token::DivideAssign => BinOp::Divide,
                    Token::ModuloAssign => BinOp::Modulo,
                    _ => unreachable!(),
                };
                Expr::Assignment {
                    name: name.clone(),
                    value: Box::new(Expr::Binary {
                        left: Box::new(Expr::Identifier(name)),
                        op,
                        right: Box::new(value),
                    }),
                }
            },
        ),
        parse_logical_or,
    ))(input)
}

/// Parse expression (top level)
fn parse_expr(input: &[Token]) -> IResult<&[Token], Expr> {
    parse_assignment_expr(input)
}

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
fn parse_stmt(input: &[Token]) -> IResult<&[Token], Stmt> {
    alt((
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
        assert!(!extern_func.is_variadic);
    }

    #[test]
    fn test_parse_extern_function_variadic() {
        let tokens = lex("extern int printf(int, ...); int main() { return 0; }").unwrap();
        let ast = parse(&tokens).unwrap();
        assert_eq!(ast.extern_functions.len(), 1);
        let extern_func = &ast.extern_functions[0];
        assert_eq!(extern_func.name, "printf");
        assert_eq!(extern_func.param_types, vec![Type::Int]);
        assert!(extern_func.is_variadic);
    }

    #[test]
    fn test_parse_include() {
        let tokens = lex("#include <stdio.h> int main() { return 0; }").unwrap();
        let ast = parse(&tokens).unwrap();
        assert_eq!(ast.includes.len(), 1);
        assert_eq!(ast.includes[0], "stdio.h");
    }

    #[test]
    fn test_parse_array() {
        let tokens = lex("int main() { int arr[5]; arr[0] = 42; return arr[0]; }").unwrap();
        let ast = parse(&tokens).unwrap();
        assert_eq!(ast.functions.len(), 1);
        if let Stmt::Block(stmts) = &ast.functions[0].body {
            assert_eq!(stmts.len(), 3);
            match &stmts[0] {
                Stmt::ArrayDeclaration { ty, name, dims } => {
                    assert_eq!(*ty, Type::Int);
                    assert_eq!(name, "arr");
                    assert_eq!(*dims, vec![5]);
                }
                _ => panic!("Expected array declaration"),
            }
            match &stmts[1] {
                Stmt::Expr(Expr::IndexAssignment {
                    name,
                    indices,
                    value,
                }) => {
                    assert_eq!(name, "arr");
                    assert_eq!(indices.len(), 1);
                    assert_eq!(indices[0], Expr::Literal(Literal::Int(0)));
                    assert_eq!(**value, Expr::Literal(Literal::Int(42)));
                }
                _ => panic!("Expected index assignment"),
            }
            match &stmts[2] {
                Stmt::Return(Some(Expr::Index { name, indices })) => {
                    assert_eq!(name, "arr");
                    assert_eq!(indices.len(), 1);
                    assert_eq!(indices[0], Expr::Literal(Literal::Int(0)));
                }
                _ => panic!("Expected return with index expr"),
            }
        } else {
            panic!("Expected block body");
        }
    }

    #[test]
    fn test_parse_2d_array() {
        let tokens =
            lex("int main() { int grid[3][4]; grid[1][2] = 42; return grid[1][2]; }").unwrap();
        let ast = parse(&tokens).unwrap();
        assert_eq!(ast.functions.len(), 1);
        if let Stmt::Block(stmts) = &ast.functions[0].body {
            assert_eq!(stmts.len(), 3);
            match &stmts[0] {
                Stmt::ArrayDeclaration { ty, name, dims } => {
                    assert_eq!(*ty, Type::Int);
                    assert_eq!(name, "grid");
                    assert_eq!(*dims, vec![3, 4]);
                }
                _ => panic!("Expected 2D array declaration"),
            }
            match &stmts[1] {
                Stmt::Expr(Expr::IndexAssignment {
                    name,
                    indices,
                    value,
                }) => {
                    assert_eq!(name, "grid");
                    assert_eq!(indices.len(), 2);
                    assert_eq!(indices[0], Expr::Literal(Literal::Int(1)));
                    assert_eq!(indices[1], Expr::Literal(Literal::Int(2)));
                    assert_eq!(**value, Expr::Literal(Literal::Int(42)));
                }
                _ => panic!("Expected 2D index assignment"),
            }
            match &stmts[2] {
                Stmt::Return(Some(Expr::Index { name, indices })) => {
                    assert_eq!(name, "grid");
                    assert_eq!(indices.len(), 2);
                    assert_eq!(indices[0], Expr::Literal(Literal::Int(1)));
                    assert_eq!(indices[1], Expr::Literal(Literal::Int(2)));
                }
                _ => panic!("Expected return with 2D index expr"),
            }
        } else {
            panic!("Expected block body");
        }
    }

    #[test]
    fn test_parse_pointers() {
        let tokens =
            lex("void swap(int* a, int* b) { int temp = *a; *a = *b; *b = temp; }").unwrap();
        let ast = parse(&tokens).unwrap();
        assert_eq!(ast.functions.len(), 1);
        let f = &ast.functions[0];
        assert_eq!(f.name, "swap");
        assert_eq!(f.return_ty, Type::Void);
        assert_eq!(
            f.params,
            vec![
                (Type::Pointer(Box::new(Type::Int)), "a".to_string()),
                (Type::Pointer(Box::new(Type::Int)), "b".to_string()),
            ]
        );
        if let Stmt::Block(stmts) = &f.body {
            assert_eq!(stmts.len(), 3);
            // int temp = *a;
            match &stmts[0] {
                Stmt::Declaration { ty, name, init } => {
                    assert_eq!(*ty, Type::Int);
                    assert_eq!(name, "temp");
                    match init {
                        Some(Expr::Unary { op, expr }) => {
                            assert_eq!(*op, UnaryOp::Deref);
                            assert_eq!(**expr, Expr::Identifier("a".to_string()));
                        }
                        _ => panic!("Expected *a init"),
                    }
                }
                _ => panic!("Expected temp decl"),
            }
            // *a = *b;
            match &stmts[1] {
                Stmt::Expr(Expr::DerefAssignment { target, value }) => {
                    assert_eq!(**target, Expr::Identifier("a".to_string()));
                    match &**value {
                        Expr::Unary { op, expr } => {
                            assert_eq!(*op, UnaryOp::Deref);
                            assert_eq!(**expr, Expr::Identifier("b".to_string()));
                        }
                        _ => panic!("Expected *b rhs"),
                    }
                }
                _ => panic!("Expected *a = *b deref assignment"),
            }
        }
    }
}
