//! # SMT-LIB 2 Lexical Tokenizer
//!
//! Tokenizes SMT-LIB 2 solver output into a stream of tokens with position
//! tracking for error reporting.
//!
//! ## Token Types
//!
//! The tokenizer handles the full SMT-LIB 2 token grammar:
//!
//! - Parentheses: `(`, `)`
//! - Simple symbols: `sat`, `unsat`, `unknown`, `model`, `define-fun`
//! - Quoted symbols (pipe-delimited): `|some symbol|`
//! - String literals: `"hello world"`
//! - Numeric literals: `42`, `-17`
//! - Keywords: `:name`, `:named`
//! - Comments: `; this is a comment`

// The tokenizer is a separate module that will replace the inline tokenization
// in parser.rs in a future change. Currently unused from that module, hence
// dead-code allowances.
#![allow(dead_code)]

use crate::term::LogicError;

/// A single lexical token from SMT-LIB 2 output.
#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    /// Left parenthesis
    LParen,
    /// Right parenthesis
    RParen,
    /// A simple symbol or keyword
    Symbol(&'a str),
    /// A string literal (contents without quotes)
    String(&'a str),
    /// A numeric literal
    Numeral(i64),
    /// A keyword starting with `:`
    Keyword(&'a str),
}

/// Position in the input source for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// 1-indexed line number
    pub line: usize,
    /// 1-indexed column number
    pub col: usize,
    /// Byte offset from start of input
    pub offset: usize,
}

/// Errors that can occur during tokenization.
#[derive(Debug, Clone)]
pub struct TokenizeError {
    /// Position where the error occurred
    pub position: Position,
    /// Description of what went wrong
    pub message: String,
}

impl std::fmt::Display for TokenizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "tokenize error at {}:{}: {}",
            self.position.line, self.position.col, self.message
        )
    }
}

impl std::error::Error for TokenizeError {}

impl From<TokenizeError> for LogicError {
    fn from(e: TokenizeError) -> Self {
        LogicError::Parse {
            line: e.position.line,
            col: e.position.col,
            msg: e.message,
        }
    }
}

/// Iterator over tokens in SMT-LIB 2 input.
pub struct Tokenizer<'a> {
    input: &'a str,
    pos: Position,
    chars: std::str::CharIndices<'a>,
}

impl<'a> Tokenizer<'a> {
    /// Create a new tokenizer for the given input.
    pub fn new(input: &'a str) -> Self {
        Tokenizer {
            input,
            pos: Position {
                line: 1,
                col: 1,
                offset: 0,
            },
            chars: input.char_indices(),
        }
    }

    /// Peek at the next character without consuming it.
    fn peek(&self) -> Option<char> {
        self.input[self.pos.offset..].chars().next()
    }

    /// Advance one character, updating position tracking.
    fn advance(&mut self) {
        if let Some((_, c)) = self.chars.next() {
            self.pos.offset += c.len_utf8();
            if c == '\n' {
                self.pos.line += 1;
                self.pos.col = 1;
            } else {
                self.pos.col += 1;
            }
        }
    }

    /// Match and advance if the next characters match a literal string.
    fn match_str(&mut self, expected: &str) -> bool {
        let remaining = &self.input[self.pos.offset..];
        if remaining.starts_with(expected) {
            for _ in 0..expected.chars().count() {
                self.advance();
            }
            true
        } else {
            false
        }
    }

    /// Skip whitespace characters.
    fn skip_whitespace(&mut self) {
        loop {
            self.skip_comments();
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    /// Skip comments (semicolon to end of line).
    fn skip_comments(&mut self) {
        while self.peek() == Some(';') {
            // Consume until end of line
            while let Some(c) = self.peek() {
                self.advance();
                if c == '\n' {
                    break;
                }
            }
        }
    }

    /// Read a simple symbol (alphanumeric + `_` `-` `!` `.` `+` `*` `/` `%` `?` `=`
    /// `<` `>` `@` `$` `~` `&` `^`).
    fn read_symbol(&mut self) -> &'a str {
        let start = self.pos.offset;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric()
                || matches!(
                    c,
                    '_' | '-'
                        | '!'
                        | '.'
                        | '+'
                        | '*'
                        | '/'
                        | '%'
                        | '?'
                        | '='
                        | '<'
                        | '>'
                        | '@'
                        | '$'
                        | '~'
                        | '&'
                        | '^'
                )
            {
                self.advance();
            } else {
                break;
            }
        }
        &self.input[start..self.pos.offset]
    }

    /// Read a numeric literal (digits, optional leading `-`).
    fn read_number(&mut self) -> i64 {
        let start = self.pos.offset;
        // consume leading `-`
        if self.peek() == Some('-') {
            self.advance();
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        let s = &self.input[start..self.pos.offset];
        s.parse().unwrap_or(0)
    }

    /// Read a quoted string literal (between double quotes).
    fn read_string(&mut self) -> Result<&'a str, TokenizeError> {
        let start = self.pos.offset; // after opening `"`
        loop {
            match self.peek() {
                None => {
                    return Err(TokenizeError {
                        position: self.pos,
                        message: "unterminated string literal".into(),
                    });
                }
                Some('"') => {
                    self.advance();
                    break;
                }
                Some(_) => {
                    self.advance();
                }
            }
        }
        Ok(&self.input[start..self.pos.offset - 1])
    }

    /// Read a pipe-delimited quoted symbol.
    fn read_pipe_symbol(&mut self) -> Result<&'a str, TokenizeError> {
        let start = self.pos.offset; // after opening `|`
        loop {
            match self.peek() {
                None => {
                    return Err(TokenizeError {
                        position: self.pos,
                        message: "unterminated pipe symbol".into(),
                    });
                }
                Some('|') => {
                    self.advance();
                    break;
                }
                Some(_) => {
                    self.advance();
                }
            }
        }
        Ok(&self.input[start..self.pos.offset - 1])
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Result<Token<'a>, TokenizeError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace();
        let c = self.peek()?;

        Some(match c {
            '(' => {
                self.advance();
                Ok(Token::LParen)
            }
            ')' => {
                self.advance();
                Ok(Token::RParen)
            }
            '"' => {
                self.advance();
                self.read_string().map(Token::String)
            }
            '|' => {
                self.advance();
                self.read_pipe_symbol().map(|s| {
                    // Pipe symbols are treated as regular symbols
                    Token::Symbol(s)
                })
            }
            ':' => {
                self.advance();
                let name = self.read_symbol();
                Ok(Token::Keyword(name))
            }
            '-' | '0'..='9' => {
                // Negative numbers vs symbols starting with `-`
                if c == '-' {
                    let saved = self.pos;
                    self.advance();
                    if self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        // It's a negative number — rewind and re-read as number
                        // Actually we already advanced past `-`. Just read digits.
                        while let Some(c) = self.peek() {
                            if c.is_ascii_digit() {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        let s = &self.input[saved.offset..self.pos.offset];
                        Ok(Token::Numeral(s.parse().unwrap_or(0)))
                    } else {
                        // It's a symbol starting with `-`
                        // Rewind and re-read as symbol
                        self.pos = saved;
                        Ok(Token::Symbol(self.read_symbol()))
                    }
                } else {
                    Ok(Token::Numeral(self.read_number()))
                }
            }
            _ if c.is_alphanumeric()
                || matches!(
                    c,
                    '_' | '!'
                        | '.'
                        | '+'
                        | '*'
                        | '/'
                        | '%'
                        | '?'
                        | '='
                        | '<'
                        | '>'
                        | '@'
                        | '$'
                        | '~'
                        | '&'
                        | '^'
                ) =>
            {
                Ok(Token::Symbol(self.read_symbol()))
            }
            _ => {
                let saved = self.pos;
                self.advance();
                Err(TokenizeError {
                    position: saved,
                    message: format!("unexpected character '{}'", c),
                })
            }
        })
    }
}

/// Collect all tokens from input, returning an error on the first lexical error.
pub fn tokenize<'a>(input: &'a str) -> Result<Vec<Token<'a>>, TokenizeError> {
    Tokenizer::new(input).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_basic_tokens() {
        let tokens = tokenize("(sat)").unwrap();
        assert_eq!(
            tokens,
            vec![Token::LParen, Token::Symbol("sat"), Token::RParen]
        );
    }

    #[test]
    fn t_unsat() {
        let tokens = tokenize("unsat").unwrap();
        assert_eq!(tokens, vec![Token::Symbol("unsat")]);
    }

    #[test]
    fn t_nested() {
        let tokens = tokenize("(assert (= x y))").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::LParen,
                Token::Symbol("assert"),
                Token::LParen,
                Token::Symbol("="),
                Token::Symbol("x"),
                Token::Symbol("y"),
                Token::RParen,
                Token::RParen,
            ]
        );
    }

    #[test]
    fn t_numerals() {
        let tokens = tokenize("42 -17 0").unwrap();
        assert_eq!(
            tokens,
            vec![Token::Numeral(42), Token::Numeral(-17), Token::Numeral(0),]
        );
    }

    #[test]
    fn t_keywords() {
        let tokens = tokenize(":named :name").unwrap();
        assert_eq!(
            tokens,
            vec![Token::Keyword("named"), Token::Keyword("name")]
        );
    }

    #[test]
    fn t_string() {
        let tokens = tokenize("\"hello world\"").unwrap();
        assert_eq!(tokens, vec![Token::String("hello world")]);
    }

    #[test]
    fn t_pipe_symbol() {
        let tokens = tokenize("|some symbol|").unwrap();
        assert_eq!(tokens, vec![Token::Symbol("some symbol")]);
    }

    #[test]
    fn t_comment() {
        let tokens = tokenize("sat ; this is a comment\nunsat").unwrap();
        assert_eq!(tokens, vec![Token::Symbol("sat"), Token::Symbol("unsat")]);
    }

    #[test]
    fn t_empty_input() {
        let tokens = tokenize("").unwrap();
        assert!(tokens.is_empty());
    }

    #[test]
    fn t_unterminated_string() {
        let err = tokenize("\"hello").unwrap_err();
        assert!(err.message.contains("unterminated string"));
    }

    #[test]
    fn t_unterminated_pipe() {
        let err = tokenize("|hello").unwrap_err();
        assert!(err.message.contains("unterminated pipe"));
    }
}
