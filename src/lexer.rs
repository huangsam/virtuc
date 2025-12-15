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

use crate::error::LexerError;
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

    /// String keyword (for const char*)
    #[token("string")]
    StringType,

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

    /// Extern keyword
    #[token("extern")]
    Extern,
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    Identifier(String),

    /// Float literal
    #[regex(r"\d+\.\d+", |lex| lex.slice().parse::<f64>().unwrap())]
    FloatLiteral(f64),

    /// Integer literal
    #[regex(r"\d+", |lex| lex.slice().parse::<i64>().unwrap())]
    IntLiteral(i64),

    /// String literal
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        // Strip surrounding quotes and unescape common C-style escapes
        let s = lex.slice();
        let inner = &s[1..s.len()-1];
        unescape_c_string(inner)
    })]
    StringLiteral(String),



    /// Include directive: #include <header.h>
    #[regex(r"#include\s*<[^>]+>", |lex| {
        let s = lex.slice();
        let start = s.find('<').map(|i| i+1).unwrap_or(0);
        let end = s.find('>').unwrap_or(s.len());
        s[start..end].to_string()
    })]
    Include(String),

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

    /// Ellipsis for variadic functions
    #[token("...")]
    Ellipsis,
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
pub fn lex(input: &str) -> Result<Vec<Token>, LexerError> {
    let lexer = Token::lexer(input);
    let mut tokens = Vec::new();

    for token in lexer {
        match token {
            Ok(t) => tokens.push(t),
            Err(_) => return Err(LexerError),
        }
    }

    Ok(tokens)
}

// Helper: Unescape a C-style string body (no surrounding quotes)
fn unescape_c_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\'') => out.push('\''),
                Some('"') => out.push('"'),
                Some('0') => out.push('\0'),
                Some('x') => {
                    // parse up to two hex digits
                    let hi = chars.next();
                    let lo = if let Some(_c2) = hi { chars.next() } else { None };
                    if let (Some(h), Some(l)) = (hi, lo) {
                        if let (Some(hv), Some(lv)) = (h.to_digit(16), l.to_digit(16)) {
                            let val = (hv * 16 + lv) as u8;
                            out.push(val as char);
                        } else {
                            out.push('x');
                            out.push(h);
                            out.push(l);
                        }
                    } else if let Some(h) = hi {
                        if let Some(hv) = h.to_digit(16) {
                            let val = hv as u8;
                            out.push(val as char);
                        } else {
                            out.push('x');
                            out.push(h);
                        }
                    }
                }
                Some(other) => {
                    // Unknown escape, keep as-is
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_declaration() {
        let input = "int x = 5;";
        let expected = vec![
            Token::Int,
            Token::Identifier("x".to_string()),
            Token::Assign,
            Token::IntLiteral(5),
            Token::Semicolon,
        ];
        assert_eq!(lex(input).unwrap(), expected);
    }

    #[test]
    fn test_float_declaration() {
        let input = "float y = 3.14;";
        let expected = vec![
            Token::Float,
            Token::Identifier("y".to_string()),
            Token::Assign,
            Token::FloatLiteral(3.14),
            Token::Semicolon,
        ];
        assert_eq!(lex(input).unwrap(), expected);
    }

    #[test]
    fn test_arithmetic_expression() {
        let input = "x + y * 2";
        let expected = vec![
            Token::Identifier("x".to_string()),
            Token::Plus,
            Token::Identifier("y".to_string()),
            Token::Multiply,
            Token::IntLiteral(2),
        ];
        assert_eq!(lex(input).unwrap(), expected);
    }

    #[test]
    fn test_comparison() {
        let input = "a == b";
        let expected = vec![
            Token::Identifier("a".to_string()),
            Token::Equal,
            Token::Identifier("b".to_string()),
        ];
        assert_eq!(lex(input).unwrap(), expected);
    }

    #[test]
    fn test_invalid_input() {
        let input = "int x = @;";
        assert!(lex(input).is_err());
    }

    #[test]
    fn test_string_literal_unescape() {
        let input = r#"int main() { printf("Hello\n"); }"#;
        let tokens = lex(input).unwrap();
        // find StringLiteral token
        assert!(tokens.iter().any(|t| matches!(t, Token::StringLiteral(s) if s == "Hello\n")));
    }

    #[test]
    fn test_function_declaration() {
        let input = "int add(int a, int b) { return a + b; }";
        let expected = vec![
            Token::Int,
            Token::Identifier("add".to_string()),
            Token::LParen,
            Token::Int,
            Token::Identifier("a".to_string()),
            Token::Comma,
            Token::Int,
            Token::Identifier("b".to_string()),
            Token::RParen,
            Token::LBrace,
            Token::Return,
            Token::Identifier("a".to_string()),
            Token::Plus,
            Token::Identifier("b".to_string()),
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
            Token::Identifier("x".to_string()),
            Token::GreaterThan,
            Token::IntLiteral(0),
            Token::RParen,
            Token::LBrace,
            Token::Return,
            Token::Identifier("x".to_string()),
            Token::Semicolon,
            Token::RBrace,
            Token::Else,
            Token::LBrace,
            Token::Return,
            Token::IntLiteral(0),
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
            Token::Identifier("i".to_string()),
            Token::Assign,
            Token::IntLiteral(0),
            Token::Semicolon,
            Token::Identifier("i".to_string()),
            Token::LessThan,
            Token::IntLiteral(10),
            Token::Semicolon,
            Token::Identifier("i".to_string()),
            Token::Assign,
            Token::Identifier("i".to_string()),
            Token::Plus,
            Token::IntLiteral(1),
            Token::RParen,
            Token::LBrace,
            Token::Identifier("x".to_string()),
            Token::Assign,
            Token::Identifier("x".to_string()),
            Token::Plus,
            Token::Identifier("i".to_string()),
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
            Token::Identifier("x".to_string()),
            Token::Assign,
            Token::IntLiteral(5),
            Token::Semicolon,
            Token::Float,
            Token::Identifier("y".to_string()),
            Token::Semicolon,
        ];
        assert_eq!(lex(input).unwrap(), expected);
    }
}
