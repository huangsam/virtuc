//! # Abstract Syntax Tree Definitions
//!
//! This module defines the data structures that represent the Abstract Syntax
//! Tree (AST) nodes for the C subset. Each node corresponds to a construct
//! in the language grammar.
//!
//! ## Node Types
//!
//! - **Expressions**: Binary operations, literals, identifiers, function calls
//! - **Statements**: Variable declarations, assignments, returns, blocks
//! - **Control Flow**: If-else statements, for loops
//! - **Functions**: Function declarations and definitions
//! - **Program**: Top-level program structure
//!
//! ## Design
//!
//! AST nodes are defined as enums and structs with owned data to simplify
//! lifetime management. Each node includes source location information for
//! error reporting and debugging.

/// Represents the primitive types in the C subset.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Type {
    /// 64-bit integer type
    Int,
    /// 64-bit floating-point type
    Float,
}

/// Represents binary operators.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinOp {
    /// Addition
    Plus,
    /// Subtraction
    Minus,
    /// Multiplication
    Multiply,
    /// Division
    Divide,
    /// Equality comparison
    Equal,
    /// Inequality comparison
    NotEqual,
    /// Less than comparison
    LessThan,
    /// Greater than comparison
    GreaterThan,
    /// Less than or equal comparison
    LessEqual,
    /// Greater than or equal comparison
    GreaterEqual,
}

/// Represents literal values.
#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    /// Integer literal
    Int(i64),
    /// Float literal
    Float(f64),
}

/// Represents expressions in the AST.
#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    /// Literal value
    Literal(Literal),
    /// Variable identifier
    Identifier(String),
    /// Binary operation
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    /// Function call
    Call { name: String, args: Vec<Expr> },
    /// Assignment expression
    Assignment { name: String, value: Box<Expr> },
}

/// Represents statements in the AST.
#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    /// Variable declaration
    Declaration {
        ty: Type,
        name: String,
        init: Option<Expr>,
    },
    /// Return statement
    Return(Option<Expr>),
    /// Block of statements
    Block(Vec<Stmt>),
    /// If-else statement
    If {
        cond: Expr,
        then: Box<Stmt>,
        else_: Option<Box<Stmt>>,
    },
    /// For loop
    For {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        update: Option<Expr>,
        body: Box<Stmt>,
    },
    /// Expression statement (for function calls, etc.)
    Expr(Expr),
}

/// Represents a function definition.
#[derive(Debug, PartialEq, Clone)]
pub struct Function {
    /// Return type of the function
    pub return_ty: Type,
    /// Name of the function
    pub name: String,
    /// Parameters: (type, name) pairs
    pub params: Vec<(Type, String)>,
    /// Function body
    pub body: Stmt,
}

/// Represents an extern function declaration.
#[derive(Debug, PartialEq, Clone)]
pub struct ExternFunction {
    /// Return type of the function
    pub return_ty: Type,
    /// Name of the function
    pub name: String,
    /// Parameter types (fixed parameters)
    pub param_types: Vec<Type>,
    /// Whether the function is variadic
    pub is_variadic: bool,
}

/// Represents the top-level program.
#[derive(Debug, PartialEq, Clone)]
pub struct Program {
    /// List of extern function declarations
    pub extern_functions: Vec<ExternFunction>,
    /// List of function definitions
    pub functions: Vec<Function>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_function() {
        let func = Function {
            return_ty: Type::Int,
            name: "add".to_string(),
            params: vec![(Type::Int, "a".to_string()), (Type::Int, "b".to_string())],
            body: Stmt::Block(vec![Stmt::Return(Some(Expr::Binary {
                left: Box::new(Expr::Identifier("a".to_string())),
                op: BinOp::Plus,
                right: Box::new(Expr::Identifier("b".to_string())),
            }))]),
        };
        // Basic construction test
        assert_eq!(func.name, "add");
        assert_eq!(func.return_ty, Type::Int);
    }

    #[test]
    fn test_if_statement() {
        let if_stmt = Stmt::If {
            cond: Expr::Binary {
                left: Box::new(Expr::Identifier("x".to_string())),
                op: BinOp::GreaterThan,
                right: Box::new(Expr::Literal(Literal::Int(0))),
            },
            then: Box::new(Stmt::Return(Some(Expr::Identifier("x".to_string())))),
            else_: Some(Box::new(Stmt::Return(Some(Expr::Literal(Literal::Int(0)))))),
        };
        // Test structure
        if let Stmt::If { cond, then, else_ } = if_stmt {
            assert!(matches!(cond, Expr::Binary { .. }));
            assert!(matches!(*then, Stmt::Return(Some(Expr::Identifier(_)))));
            assert!(else_.is_some());
        } else {
            panic!("Expected If statement");
        }
    }
}
