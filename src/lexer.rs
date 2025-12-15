//! # Lexical Analysis
//!
//! This module handles the lexical analysis phase of the compiler, converting
//! raw source code into a stream of tokens. It uses the `logos` crate for
//! efficient tokenization.
//!
//! ## Supported Tokens
//!
//! The lexer recognizes tokens for the C subset including:
//! - Keywords: `int`, `float`, `if`, `else`, `for`, `return`
//! - Operators: `+`, `-`, `*`, `/`, `=`, `==`, `!=`, `<`, `>`, etc.
//! - Literals: Integer and floating-point numbers
//! - Identifiers: Variable and function names
//! - Punctuation: `(`, `)`, `{`, `}`, `;`, `,`, etc.
//!
//! ## Implementation
//!
//! Uses the `logos` procedural macro to define token patterns and generate
//! the lexer automatically. Handles whitespace, comments, and error recovery.

use logos::Logos;

/// Represents the tokens produced by the lexer.
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")] // Skip whitespace
#[logos(skip r"//[^\n]*")] // Skip single-line comments
pub enum Token {
    /// Integer keyword
    #[token("int")]
    Int,

    /// Float keyword
    #[token("float")]
    Float,

    /// If keyword
    #[token("if")]
    If,

    /// Else keyword
    #[token("else")]
    Else,

    /// For keyword
    #[token("for")]
    For,

    /// Return keyword
    #[token("return")]
    Return,

    /// Identifier (e.g., variable names)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,

    /// Float literal
    #[regex(r"\d+\.\d+")]
    FloatLiteral,

    /// Integer literal
    #[regex(r"\d+")]
    IntLiteral,

    /// Less than or equal operator
    #[token("<=")]
    LessEqual,

    /// Greater than or equal operator
    #[token(">=")]
    GreaterEqual,

    /// Equal operator
    #[token("==")]
    Equal,

    /// Not equal operator
    #[token("!=")]
    NotEqual,

    /// Less than operator
    #[token("<")]
    LessThan,

    /// Greater than operator
    #[token(">")]
    GreaterThan,

    /// Assignment operator
    #[token("=")]
    Assign,

    /// Plus operator
    #[token("+")]
    Plus,

    /// Minus operator
    #[token("-")]
    Minus,

    /// Multiply operator
    #[token("*")]
    Multiply,

    /// Divide operator
    #[token("/")]
    Divide,

    /// Semicolon
    #[token(";")]
    Semicolon,

    /// Comma
    #[token(",")]
    Comma,

    /// Left parenthesis
    #[token("(")]
    LParen,

    /// Right parenthesis
    #[token(")")]
    RParen,

    /// Left brace
    #[token("{")]
    LBrace,

    /// Right brace
    #[token("}")]
    RBrace,
}

/// Lexes the input source code into a vector of tokens.
///
/// # Arguments
///
/// * `input` - The source code string to tokenize.
///
/// # Returns
///
/// A `Result` containing a vector of tokens or a lexing error.
pub fn lex(input: &str) -> Result<Vec<Token>, ()> {
    let mut lexer = Token::lexer(input);
    let mut tokens = Vec::new();

    while let Some(token) = lexer.next() {
        match token {
            Ok(t) => tokens.push(t),
            Err(()) => return Err(()),
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_declaration() {
        let input = "int x = 5;";
        let expected = vec![
            Token::Int,
            Token::Identifier,
            Token::Assign,
            Token::IntLiteral,
            Token::Semicolon,
        ];
        assert_eq!(lex(input).unwrap(), expected);
    }

    #[test]
    fn test_float_declaration() {
        let input = "float y = 3.14;";
        let expected = vec![
            Token::Float,
            Token::Identifier,
            Token::Assign,
            Token::FloatLiteral,
            Token::Semicolon,
        ];
        assert_eq!(lex(input).unwrap(), expected);
    }

    #[test]
    fn test_arithmetic_expression() {
        let input = "x + y * 2";
        let expected = vec![
            Token::Identifier,
            Token::Plus,
            Token::Identifier,
            Token::Multiply,
            Token::IntLiteral,
        ];
        assert_eq!(lex(input).unwrap(), expected);
    }

    #[test]
    fn test_comparison() {
        let input = "a == b";
        let expected = vec![
            Token::Identifier,
            Token::Equal,
            Token::Identifier,
        ];
        assert_eq!(lex(input).unwrap(), expected);
    }

    #[test]
    fn test_invalid_input() {
        let input = "int x = @;";
        assert!(lex(input).is_err());
    }

    #[test]
    fn test_function_declaration() {
        let input = "int add(int a, int b) { return a + b; }";
        let expected = vec![
            Token::Int,
            Token::Identifier,
            Token::LParen,
            Token::Int,
            Token::Identifier,
            Token::Comma,
            Token::Int,
            Token::Identifier,
            Token::RParen,
            Token::LBrace,
            Token::Return,
            Token::Identifier,
            Token::Plus,
            Token::Identifier,
            Token::Semicolon,
            Token::RBrace,
        ];
        assert_eq!(lex(input).unwrap(), expected);
    }

    #[test]
    fn test_if_statement() {
        let input = "if (x > 0) { return x; } else { return 0; }";
        let expected = vec![
            Token::If,
            Token::LParen,
            Token::Identifier,
            Token::GreaterThan,
            Token::IntLiteral,
            Token::RParen,
            Token::LBrace,
            Token::Return,
            Token::Identifier,
            Token::Semicolon,
            Token::RBrace,
            Token::Else,
            Token::LBrace,
            Token::Return,
            Token::IntLiteral,
            Token::Semicolon,
            Token::RBrace,
        ];
        assert_eq!(lex(input).unwrap(), expected);
    }

    #[test]
    fn test_for_loop() {
        let input = "for (int i = 0; i < 10; i = i + 1) { x = x + i; }";
        let expected = vec![
            Token::For,
            Token::LParen,
            Token::Int,
            Token::Identifier,
            Token::Assign,
            Token::IntLiteral,
            Token::Semicolon,
            Token::Identifier,
            Token::LessThan,
            Token::IntLiteral,
            Token::Semicolon,
            Token::Identifier,
            Token::Assign,
            Token::Identifier,
            Token::Plus,
            Token::IntLiteral,
            Token::RParen,
            Token::LBrace,
            Token::Identifier,
            Token::Assign,
            Token::Identifier,
            Token::Plus,
            Token::Identifier,
            Token::Semicolon,
            Token::RBrace,
        ];
        assert_eq!(lex(input).unwrap(), expected);
    }

    #[test]
    fn test_comments_and_whitespace() {
        let input = "int x = 5; // this is a comment\nfloat y;";
        let expected = vec![
            Token::Int,
            Token::Identifier,
            Token::Assign,
            Token::IntLiteral,
            Token::Semicolon,
            Token::Float,
            Token::Identifier,
            Token::Semicolon,
        ];
        assert_eq!(lex(input).unwrap(), expected);
    }
}
