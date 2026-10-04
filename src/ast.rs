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

/// Represents the primitive and derived types in the C subset.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Type {
    /// 64-bit integer type
    Int,
    /// 64-bit floating-point type
    Float,
    /// String type (const char*)
    String,
    /// Void type (for functions without return value)
    Void,
    /// Pointer type (e.g. int*, void*)
    Pointer(Box<Type>),
    /// Struct type by name (e.g. `struct Point`)
    Struct(String),
}

/// Represents a field in a struct definition.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructField {
    /// Field type
    pub ty: Type,
    /// Field name
    pub name: String,
}

/// Represents a struct definition (`struct Name { ... };`).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructDef {
    /// Name of the struct
    pub name: String,
    /// List of fields in order
    pub fields: Vec<StructField>,
}

/// Represents binary operators.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BinOp {
    /// Addition
    Plus,
    /// Subtraction
    Minus,
    /// Multiplication
    Multiply,
    /// Division
    Divide,
    /// Modulo / remainder
    Modulo,
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

/// Represents unary operators.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UnaryOp {
    /// Negation (-)
    Neg,
    /// Logical NOT (!)
    Not,
    /// Address-of (&)
    AddrOf,
    /// Dereference (*)
    Deref,
}

/// Represents literal values.
#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    /// Integer literal
    Int(i64),
    /// Float literal
    Float(f64),
    /// String literal
    String(String),
}

/// Represents expressions in the AST.
#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    /// Literal value
    Literal(Literal),
    /// Variable identifier
    Identifier(String),
    /// Unary operation
    Unary {
        /// The unary operator
        op: UnaryOp,
        /// The operand expression
        expr: Box<Expr>,
    },
    /// Binary operation
    Binary {
        /// Left-hand side operand
        left: Box<Expr>,
        /// Binary operator
        op: BinOp,
        /// Right-hand side operand
        right: Box<Expr>,
    },
    /// Short-circuiting logical AND (&&)
    LogicalAnd {
        /// Left-hand side operand
        left: Box<Expr>,
        /// Right-hand side operand
        right: Box<Expr>,
    },
    /// Short-circuiting logical OR (||)
    LogicalOr {
        /// Left-hand side operand
        left: Box<Expr>,
        /// Right-hand side operand
        right: Box<Expr>,
    },
    /// Array indexing access: `identifier[index]` or `identifier[i][j]...`
    Index {
        /// Name of the array variable
        name: String,
        /// Index expressions for each dimension
        indices: Vec<Expr>,
    },
    /// Array indexing assignment: `identifier[index] = value` or `identifier[i][j]... = value`
    IndexAssignment {
        /// Name of the array variable
        name: String,
        /// Index expressions for each dimension
        indices: Vec<Expr>,
        /// Value to assign
        value: Box<Expr>,
    },
    /// Pointer dereference assignment: `*target = value`
    DerefAssignment {
        /// Target pointer expression being dereferenced
        target: Box<Expr>,
        /// Value to assign
        value: Box<Expr>,
    },
    /// Function call
    Call {
        /// Name of the function being called
        name: String,
        /// Arguments passed to the function
        args: Vec<Expr>,
    },
    /// Assignment expression
    Assignment {
        /// Name of the variable being assigned to
        name: String,
        /// Value expression to assign
        value: Box<Expr>,
    },
    /// Struct member access: `target.field`
    MemberAccess {
        /// Target expression evaluating to a struct
        target: Box<Expr>,
        /// Field name
        field: String,
    },
    /// Struct pointer member access: `target->field`
    ArrowAccess {
        /// Target expression evaluating to a pointer to a struct
        target: Box<Expr>,
        /// Field name
        field: String,
    },
    /// Struct member assignment: `target.field = value`
    MemberAssignment {
        /// Target expression evaluating to a struct
        target: Box<Expr>,
        /// Field name
        field: String,
        /// Value to assign
        value: Box<Expr>,
    },
    /// Struct pointer member assignment: `target->field = value`
    ArrowAssignment {
        /// Target expression evaluating to a pointer to a struct
        target: Box<Expr>,
        /// Field name
        field: String,
        /// Value to assign
        value: Box<Expr>,
    },
}

/// Represents statements in the AST.
#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    /// Variable declaration
    Declaration {
        /// Type of the variable
        ty: Type,
        /// Variable identifier name
        name: String,
        /// Optional initializer expression
        init: Option<Expr>,
    },
    /// Fixed-size array declaration: `type name[size];` or `type name[d1][d2]...;`
    ArrayDeclaration {
        /// Base element type of the array
        ty: Type,
        /// Array identifier name
        name: String,
        /// Fixed dimension sizes
        dims: Vec<usize>,
    },
    /// Return statement
    Return(Option<Expr>),
    /// Block of statements
    Block(Vec<Stmt>),
    /// If-else statement
    If {
        /// Branching condition expression
        cond: Expr,
        /// Then block statement
        then: Box<Stmt>,
        /// Optional else block statement
        else_: Option<Box<Stmt>>,
    },
    /// For loop
    For {
        /// Optional loop initialization statement
        init: Option<Box<Stmt>>,
        /// Optional loop condition expression
        cond: Option<Expr>,
        /// Optional loop update expression
        update: Option<Expr>,
        /// Loop body statement
        body: Box<Stmt>,
    },
    /// While loop
    While {
        /// Loop condition expression
        cond: Expr,
        /// Loop body statement
        body: Box<Stmt>,
    },
    /// Break statement
    Break,
    /// Continue statement
    Continue,
    /// Struct definition statement
    StructDef(StructDef),
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
    /// List of include directives (header names)
    pub includes: Vec<String>,
    /// List of struct definitions
    pub structs: Vec<StructDef>,
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

    #[test]
    fn test_struct_definition() {
        let def = StructDef {
            name: "Point".to_string(),
            fields: vec![
                StructField {
                    ty: Type::Int,
                    name: "x".to_string(),
                },
                StructField {
                    ty: Type::Int,
                    name: "y".to_string(),
                },
            ],
        };
        assert_eq!(def.name, "Point");
        assert_eq!(def.fields.len(), 2);
    }
}
