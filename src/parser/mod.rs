//! # Syntax Analysis and AST Construction
//!
//! This module implements the parser that converts a stream of tokens into
//! an Abstract Syntax Tree (AST). It uses the `nom` parser combinator library
//! for building the parsing logic.

mod expr;
mod helpers;
mod stmt;
mod types;

use nom::IResult;
use nom::branch::alt;
use nom::combinator::map;
use nom::error::{Error, ErrorKind};
use nom::multi::{many0, separated_list0};
use nom::sequence::{delimited, tuple};

use crate::ast::*;
use crate::lexer::Token;
use helpers::{parse_identifier, token};
use stmt::parse_block;
use types::{parse_extern_function, parse_param, parse_struct_def, parse_type};

#[derive(Debug, PartialEq, Clone)]
enum TopLevel {
    Include(String),
    Struct(StructDef),
    Extern(ExternFunction),
    Function(Function),
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

/// Parse a top-level item: include, struct, extern function or function definition
fn parse_top_level(input: &[Token]) -> IResult<&[Token], TopLevel> {
    alt((
        map(parse_include, TopLevel::Include),
        map(parse_struct_def, TopLevel::Struct),
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
    let mut structs = Vec::new();
    let mut extern_functions = Vec::new();
    let mut functions = Vec::new();
    for item in items {
        match item {
            TopLevel::Include(h) => includes.push(h),
            TopLevel::Struct(s) => structs.push(s),
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
        structs,
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

    #[test]
    fn test_parse_struct() {
        let input = "
            struct Point {
                int x;
                int y;
            };

            int main() {
                struct Point p;
                p.x = 10;
                struct Point* ptr = &p;
                ptr->x = 20;
                return ptr->x + p.y;
            }
        ";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        assert_eq!(ast.structs.len(), 1);
        assert_eq!(ast.structs[0].name, "Point");
        assert_eq!(ast.structs[0].fields.len(), 2);
        assert_eq!(ast.structs[0].fields[0].name, "x");
        assert_eq!(ast.structs[0].fields[0].ty, Type::Int);
        assert_eq!(ast.functions.len(), 1);
    }
}
