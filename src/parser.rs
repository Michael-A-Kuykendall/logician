//! # SMT Solver Output Parser
//!
//! This module provides parsing for SMT-LIB 2 solver responses.
//!
//! ## Supported Responses
//!
//! - `sat` / `unsat` / `unknown` - Satisfiability results
//! - `(model ...)` - Variable assignments (from `get-model`)
//! - `(error "...")` - Solver error messages
//!
//! ## Parser Design
//!
//! The parser is a hand-written recursive descent parser that:
//!
//! - Handles nested S-expressions
//! - Parses `define-fun` declarations in models
//! - Provides line/column error positions
//! - Does **not** use regex (per spell requirements)
//!
//! ## Example
//!
//! ```rust
//! use logician::parser::{parse, Response, Value};
//!
//! // Simple response
//! let resp = parse("sat").unwrap();
//! assert_eq!(resp, Response::Sat);
//!
//! // Unknown response
//! let resp = parse("unknown").unwrap();
//! assert_eq!(resp, Response::Unknown);
//! ```

/// Solver response variants.
///
/// This enum represents all possible responses from an SMT solver.
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// The assertions are satisfiable
    Sat,
    /// The assertions are unsatisfiable
    Unsat,
    /// The solver could not determine satisfiability
    Unknown,
    /// A model (satisfying assignment) - list of (name, value) pairs
    Model(Vec<(String, Value)>),
    /// Solver returned an error
    Error(String),
}

/// Value types that can appear in solver models.
///
/// When you call `get-model` after a `sat` result, the solver returns
/// variable assignments. This enum represents the possible value types.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Complex expression that couldn't be parsed to a simple type
    Unsupported(String),
}

// # Spell: RobustSexpParser
// Recursive descent parser for solver S-expression output

use crate::term::LogicError;

/// Position in input for error reporting
#[derive(Debug, Clone, Copy)]
struct Pos {
    line: usize,
    col: usize,
    offset: usize,
}

/// Parser state
struct Parser<'a> {
    input: &'a str,
    pos: Pos,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Parser {
            input,
            pos: Pos { line: 1, col: 1, offset: 0 },
        }
    }
    
    fn peek(&self) -> Option<char> {
        self.input[self.pos.offset..].chars().next()
    }
    
    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos.offset += c.len_utf8();
            if c == '\n' {
                self.pos.line += 1;
                self.pos.col = 1;
            } else {
                self.pos.col += 1;
            }
        }
    }
    
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }
    
    fn error(&self, msg: &str) -> LogicError {
        LogicError::Parse {
            line: self.pos.line,
            col: self.pos.col,
            msg: msg.to_string(),
        }
    }
    
    fn parse_symbol(&mut self) -> Result<String, LogicError> {
        self.skip_whitespace();
        let start = self.pos.offset;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '!' || c == '.' {
                self.advance();
            } else {
                break;
            }
        }
        let end = self.pos.offset;
        if start == end {
            Err(self.error("expected symbol"))
        } else {
            Ok(self.input[start..end].to_string())
        }
    }
    
    fn parse_number(&mut self) -> Result<i64, LogicError> {
        self.skip_whitespace();
        let start = self.pos.offset;
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
        let end = self.pos.offset;
        let s = &self.input[start..end];
        s.parse().map_err(|_| self.error(&format!("invalid number: {}", s)))
    }
    
    fn parse_sexp(&mut self) -> Result<Sexp, LogicError> {
        self.skip_whitespace();
        match self.peek() {
            Some('(') => {
                self.advance();
                let mut items = Vec::new();
                loop {
                    self.skip_whitespace();
                    if self.peek() == Some(')') {
                        self.advance();
                        break;
                    }
                    items.push(self.parse_sexp()?);
                }
                Ok(Sexp::List(items))
            }
            Some(c) if c.is_ascii_digit() => {
                Ok(Sexp::Num(self.parse_number()?))
            }
            Some('-') => {
                // Check if next char is a digit (negative number) or not (symbol)
                let remaining = &self.input[self.pos.offset..];
                if remaining.len() > 1 && remaining.chars().nth(1).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    Ok(Sexp::Num(self.parse_number()?))
                } else {
                    Ok(Sexp::Sym(self.parse_symbol()?))
                }
            }
            Some(_) => {
                Ok(Sexp::Sym(self.parse_symbol()?))
            }
            None => Err(self.error("unexpected EOF")),
        }
    }
}

/// S-expression AST
#[derive(Debug, Clone)]
enum Sexp {
    Sym(String),
    Num(i64),
    List(Vec<Sexp>),
}

impl Sexp {
    fn as_sym(&self) -> Option<&str> {
        match self {
            Sexp::Sym(s) => Some(s),
            _ => None,
        }
    }
}

/// Parse SMT solver output into a [`Response`].
///
/// This function handles all standard SMT-LIB 2 response formats:
///
/// - Simple: `sat`, `unsat`, `unknown`
/// - Wrapped: `(sat)`, `(unsat)`, `(unknown)`
/// - Models: `(model (define-fun x () Bool true) ...)`
/// - Errors: `(error "message")`
///
/// # Arguments
///
/// * `input` - Raw string output from the solver
///
/// # Returns
///
/// A [`Response`] variant, or [`LogicError::Parse`] on malformed input.
///
/// # Example
///
/// ```rust
/// use logician::parser::{parse, Response};
///
/// assert_eq!(parse("sat").unwrap(), Response::Sat);
/// assert_eq!(parse("unsat").unwrap(), Response::Unsat);
/// ```
pub fn parse(input: &str) -> Result<Response, LogicError> {
    // Check for simple responses first
    let trimmed = input.trim();
    if trimmed == "sat" {
        return Ok(Response::Sat);
    }
    if trimmed == "unsat" {
        return Ok(Response::Unsat);
    }
    if trimmed == "unknown" {
        return Ok(Response::Unknown);
    }
    
    // Try parsing as multiple S-expressions (handles multi-line define-fun)
    if let Ok(bindings) = try_parse_multiple_sexps(input) {
        if !bindings.is_empty() {
            return Ok(Response::Model(bindings));
        }
    }
    
    // Parse as single S-expression
    let mut parser = Parser::new(input);
    let sexp = parser.parse_sexp()?;
    
    // Handle (sat), (unsat), (unknown)
    if let Sexp::List(items) = &sexp {
        if items.len() == 1 {
            if let Some(sym) = items[0].as_sym() {
                match sym {
                    "sat" => return Ok(Response::Sat),
                    "unsat" => return Ok(Response::Unsat),
                    "unknown" => return Ok(Response::Unknown),
                    _ => {}
                }
            }
        }
        
        // Handle (error "message")
        if !items.is_empty() {
            if let Some("error") = items[0].as_sym() {
                let msg = if items.len() > 1 {
                    format!("{:?}", items[1])
                } else {
                    "unknown error".to_string()
                };
                return Ok(Response::Error(msg));
            }
        }
        
        // Handle (model ...) - contains define-fun declarations
        if !items.is_empty() {
            if let Some("model") = items[0].as_sym() {
                return parse_model(&items[1..]);
            }
        }
    }
    
    Err(LogicError::Parse {
        line: 1,
        col: 1,
        msg: "unrecognized response format".to_string(),
    })
}

fn try_parse_multiple_sexps(input: &str) -> Result<Vec<(String, Value)>, LogicError> {
    let mut parser = Parser::new(input);
    let mut bindings = Vec::new();
    
    loop {
        parser.skip_whitespace();
        if parser.peek().is_none() {
            break;
        }
        match parser.parse_sexp() {
            Ok(sexp) => {
                if let Sexp::List(items) = &sexp {
                    if !items.is_empty() {
                        if let Some("define-fun") = items[0].as_sym() {
                            if let Ok(binding) = parse_define_fun(&sexp) {
                                bindings.push(binding);
                            }
                        }
                    }
                }
                // Continue even if not a define-fun or parsing failed
            }
            Err(_) => {
                // If we can't parse, return what we have so far
                break;
            }
        }
    }
    
    Ok(bindings)
}

fn parse_model(items: &[Sexp]) -> Result<Response, LogicError> {
    let mut bindings = Vec::new();
    for item in items {
        if let Sexp::List(parts) = item {
            if !parts.is_empty() {
                if let Some("define-fun") = parts[0].as_sym() {
                    bindings.push(parse_define_fun(item)?);
                }
            }
        }
    }
    Ok(Response::Model(bindings))
}

fn parse_define_fun(sexp: &Sexp) -> Result<(String, Value), LogicError> {
    // (define-fun name () Sort value)
    if let Sexp::List(items) = sexp {
        if items.len() >= 5 {
            let name = items[1].as_sym().unwrap_or("?").to_string();
            let value = parse_value(&items[4])?;
            return Ok((name, value));
        }
    }
    Err(LogicError::Parse {
        line: 0,
        col: 0,
        msg: "invalid define-fun".to_string(),
    })
}

fn parse_value(sexp: &Sexp) -> Result<Value, LogicError> {
    match sexp {
        Sexp::Sym(s) => {
            match s.as_str() {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                _ => Ok(Value::Unsupported(s.clone())),
            }
        }
        Sexp::Num(n) => Ok(Value::Int(*n)),
        Sexp::List(items) => {
            // Handle (- n) for negative numbers
            if items.len() == 2 {
                if let Some("-") = items[0].as_sym() {
                    if let Sexp::Num(n) = &items[1] {
                        return Ok(Value::Int(-n));
                    }
                }
            }
            // Complex expressions -> Unsupported
            Ok(Value::Unsupported(format!("{:?}", sexp)))
        }
    }
}
