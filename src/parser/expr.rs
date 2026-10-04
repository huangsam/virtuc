use nom::IResult;
use nom::branch::alt;
use nom::combinator::{map, opt};
use nom::multi::{many1, separated_list0};
use nom::sequence::{delimited, preceded, tuple};

use super::helpers::{parse_binop, parse_identifier, parse_literal, token};
use crate::ast::*;
use crate::lexer::Token;

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

/// Helper to parse the base expression for a member path
fn parse_member_base(input: &[Token]) -> IResult<&[Token], Expr> {
    alt((
        parse_call,
        parse_index,
        map(parse_identifier, Expr::Identifier),
        delimited(token(Token::LParen), parse_expr, token(Token::RParen)),
    ))(input)
}

type MemberAccessItem = (bool, String);
type MemberPath = (Expr, Vec<MemberAccessItem>);

/// Helper to parse member access chain (e.g. `.field` or `->field`)
fn parse_member_access_tail(input: &[Token]) -> IResult<&[Token], Vec<MemberAccessItem>> {
    many1(alt((
        map(preceded(token(Token::Dot), parse_identifier), |f| {
            (false, f)
        }),
        map(preceded(token(Token::Arrow), parse_identifier), |f| {
            (true, f)
        }),
    )))(input)
}

/// Helper to fold a base expression and member accesses into a chain of MemberAccess / ArrowAccess
fn fold_member_access(base: Expr, accesses: &[MemberAccessItem]) -> Expr {
    let mut expr = base;
    for (is_arrow, field) in accesses {
        if *is_arrow {
            expr = Expr::ArrowAccess {
                target: Box::new(expr),
                field: field.clone(),
            };
        } else {
            expr = Expr::MemberAccess {
                target: Box::new(expr),
                field: field.clone(),
            };
        }
    }
    expr
}

/// Parse a member path: base (.field | ->field)+
fn parse_member_path(input: &[Token]) -> IResult<&[Token], MemberPath> {
    let (input, base) = parse_member_base(input)?;
    let (input, accesses) = parse_member_access_tail(input)?;
    Ok((input, (base, accesses)))
}

/// Parse member expression, optionally followed by postfix ++ or --: `base.field(++)?` | `base->field(++)?`
fn parse_member_expr(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, (base, mut accesses)) = parse_member_path(input)?;
    if let Ok((rest, op_token)) = alt((token(Token::PlusPlus), token(Token::MinusMinus)))(input) {
        let bin_op = match op_token {
            Token::PlusPlus => BinOp::Plus,
            Token::MinusMinus => BinOp::Minus,
            _ => unreachable!(),
        };
        let (last_is_arrow, last_field) = accesses.pop().unwrap();
        let target = fold_member_access(base, &accesses);
        let current_val = if last_is_arrow {
            Expr::ArrowAccess {
                target: Box::new(target.clone()),
                field: last_field.clone(),
            }
        } else {
            Expr::MemberAccess {
                target: Box::new(target.clone()),
                field: last_field.clone(),
            }
        };
        let value = Box::new(Expr::Binary {
            left: Box::new(current_val),
            op: bin_op,
            right: Box::new(Expr::Literal(Literal::Int(1))),
        });
        if last_is_arrow {
            Ok((
                rest,
                Expr::ArrowAssignment {
                    target: Box::new(target),
                    field: last_field,
                    value,
                },
            ))
        } else {
            Ok((
                rest,
                Expr::MemberAssignment {
                    target: Box::new(target),
                    field: last_field,
                    value,
                },
            ))
        }
    } else {
        Ok((input, fold_member_access(base, &accesses)))
    }
}

/// Parse a primary expression: literal | member_expr | call | postfix | index
fn parse_primary_expr(input: &[Token]) -> IResult<&[Token], Expr> {
    alt((
        map(parse_literal, Expr::Literal),
        parse_member_expr,
        parse_call,
        parse_array_postfix,
        parse_index,
        parse_postfix,
        map(parse_identifier, Expr::Identifier),
        delimited(token(Token::LParen), parse_expr, token(Token::RParen)),
    ))(input)
}

/// Parse prefix inc/dec: `++id` | `--id` | `++id[expr]` | `--id[expr]` | `++*unary` | `--*unary` | `++path` | `--path`
fn parse_prefix_inc_dec(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, op_token) = alt((token(Token::PlusPlus), token(Token::MinusMinus)))(input)?;
    let op = match op_token {
        Token::PlusPlus => BinOp::Plus,
        Token::MinusMinus => BinOp::Minus,
        _ => unreachable!(),
    };
    if let Ok((rest, (base, mut accesses))) = parse_member_path(input) {
        let (last_is_arrow, last_field) = accesses.pop().unwrap();
        let target = fold_member_access(base, &accesses);
        let current_val = if last_is_arrow {
            Expr::ArrowAccess {
                target: Box::new(target.clone()),
                field: last_field.clone(),
            }
        } else {
            Expr::MemberAccess {
                target: Box::new(target.clone()),
                field: last_field.clone(),
            }
        };
        let value = Box::new(Expr::Binary {
            left: Box::new(current_val),
            op,
            right: Box::new(Expr::Literal(Literal::Int(1))),
        });
        if last_is_arrow {
            return Ok((
                rest,
                Expr::ArrowAssignment {
                    target: Box::new(target),
                    field: last_field,
                    value,
                },
            ));
        } else {
            return Ok((
                rest,
                Expr::MemberAssignment {
                    target: Box::new(target),
                    field: last_field,
                    value,
                },
            ));
        }
    }
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

/// Parse member assignment: `target.field (=|+=|...) expr` or `target->field (=|+=|...) expr`
fn parse_member_assignment(input: &[Token]) -> IResult<&[Token], Expr> {
    let (input, (base, mut accesses)) = parse_member_path(input)?;
    let (input, op_token) = alt((
        token(Token::Assign),
        token(Token::PlusAssign),
        token(Token::MinusAssign),
        token(Token::MultiplyAssign),
        token(Token::DivideAssign),
        token(Token::ModuloAssign),
    ))(input)?;
    let (input, val_expr) = parse_expr(input)?;
    let (last_is_arrow, last_field) = accesses.pop().unwrap();
    let target = fold_member_access(base, &accesses);
    let value = match op_token {
        Token::Assign => Box::new(val_expr),
        op => {
            let bin_op = match op {
                Token::PlusAssign => BinOp::Plus,
                Token::MinusAssign => BinOp::Minus,
                Token::MultiplyAssign => BinOp::Multiply,
                Token::DivideAssign => BinOp::Divide,
                Token::ModuloAssign => BinOp::Modulo,
                _ => unreachable!(),
            };
            let current_val = if last_is_arrow {
                Expr::ArrowAccess {
                    target: Box::new(target.clone()),
                    field: last_field.clone(),
                }
            } else {
                Expr::MemberAccess {
                    target: Box::new(target.clone()),
                    field: last_field.clone(),
                }
            };
            Box::new(Expr::Binary {
                left: Box::new(current_val),
                op: bin_op,
                right: Box::new(val_expr),
            })
        }
    };
    if last_is_arrow {
        Ok((
            input,
            Expr::ArrowAssignment {
                target: Box::new(target),
                field: last_field,
                value,
            },
        ))
    } else {
        Ok((
            input,
            Expr::MemberAssignment {
                target: Box::new(target),
                field: last_field,
                value,
            },
        ))
    }
}

/// Parse an assignment expression: `target.field (=|...) expr` | `identifier (=|...) expr` | `identifier [ expr ] (=|...) expr` | `*unary (=|...) expr`
fn parse_assignment_expr(input: &[Token]) -> IResult<&[Token], Expr> {
    alt((
        parse_member_assignment,
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
pub(crate) fn parse_expr(input: &[Token]) -> IResult<&[Token], Expr> {
    parse_assignment_expr(input)
}
