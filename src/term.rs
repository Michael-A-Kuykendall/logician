//! # Type-Safe Term AST with Sort Inference
//!
//! This module provides the core data structures for building SMT formulas:
//!
//! - [`Sort`] - The type (Bool or Int) of a term
//! - [`Term`] - Immutable AST for logical formulas
//! - [`LogicError`] - All error types for the crate
//!
//! ## Design Philosophy
//!
//! Logician enforces sort correctness through runtime invariants rather than
//! complex type-level machinery. This means:
//!
//! - Simple, readable code
//! - Clear error messages on misuse
//! - Auditable invariant coverage via the tag system
//!
//! ## Example
//!
//! ```rust
//! use logician::term::{Term, Sort};
//!
//! // Boolean variables
//! let x = Term::Var("x".into(), Sort::Bool);
//! let y = Term::Var("y".into(), Sort::Bool);
//!
//! // Build formula: x ∧ (¬y → x)
//! let formula = x.clone().and(y.not().implies(x));
//!
//! // Serializes to SMT-LIB: (and x (or (not (not y)) x))
//! println!("{}", formula);
//! ```

use std::time::Duration;
use thiserror::Error;
use smallvec::SmallVec;

/// SMT sort (type) for terms.
///
/// SMT-LIB supports many sorts, but Logician currently supports:
/// - `Bool` - Boolean values (true/false)
/// - `Int` - Arbitrary precision integers
///
/// # Example
///
/// ```rust
/// use logician::term::Sort;
///
/// let bool_sort = Sort::Bool;
/// let int_sort = Sort::Int;
///
/// assert_eq!(format!("{}", bool_sort), "Bool");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    /// Boolean sort (true/false)
    Bool,
    /// Integer sort (arbitrary precision)
    Int,
}

impl std::fmt::Display for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sort::Bool => write!(f, "Bool"),
            Sort::Int => write!(f, "Int"),
        }
    }
}

/// All errors that can occur in the logician crate.
///
/// This enum covers parsing errors, solver communication errors, type mismatches,
/// and I/O failures. All variants provide actionable error messages.
///
/// # Example
///
/// ```rust
/// use logician::term::LogicError;
///
/// let err = LogicError::Solver("Z3 returned error".into());
/// assert!(err.to_string().contains("solver error"));
/// ```
#[derive(Debug, Error)]
pub enum LogicError {
    /// Parse error with line/column information
    #[error("parse error at {line}:{col}: {msg}")]
    Parse { 
        /// Line number (1-indexed)
        line: usize, 
        /// Column number (1-indexed)
        col: usize, 
        /// Error description
        msg: String 
    },
    
    /// Solver returned an error or failed to respond
    #[error("solver error: {0}")]
    Solver(String),
    
    /// Solver exceeded configured timeout
    #[error("timeout after {0:?}")]
    Timeout(Duration),
    
    /// I/O error communicating with solver process
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Term sort mismatch (e.g., Bool passed where Int expected)
    #[error("sort mismatch: expected {expected}, got {got}")]
    SortMismatch { 
        /// The expected sort
        expected: Sort, 
        /// The actual sort
        got: Sort 
    },
    
    /// Invalid term construction
    #[error("invalid term: {0}")]
    InvalidTerm(String),
}

// # Spell: TermEnum
// Immutable Term AST with sort inference and arity enforcement

/// Wrapper type enforcing minimum 2 terms for [`Term::And`].
///
/// This type exists to make it impossible to construct an And with fewer than 2 children
/// at the type level. The first two arguments are required; additional terms go in the
/// SmallVec alongside this struct.
#[derive(Debug, Clone, PartialEq)]
pub struct And2(
    /// First conjunct (required)
    pub Box<Term>, 
    /// Second conjunct (required)
    pub Box<Term>
);

/// Wrapper type enforcing minimum 2 terms for [`Term::Or`].
///
/// This type exists to make it impossible to construct an Or with fewer than 2 children
/// at the type level. The first two arguments are required; additional terms go in the
/// SmallVec alongside this struct.
#[derive(Debug, Clone, PartialEq)]
pub struct Or2(
    /// First disjunct (required)
    pub Box<Term>, 
    /// Second disjunct (required)
    pub Box<Term>
);

/// Immutable AST for SMT terms with automatic sort inference.
///
/// Terms are the building blocks of SMT formulas. Each term has a [`Sort`] that
/// is automatically inferred from its structure. Sort mismatches are caught
/// immediately via runtime invariants.
///
/// # Variants
///
/// - `Bool(bool)` - Boolean literal
/// - `Int(i64)` - Integer literal  
/// - `Var(name, sort)` - Variable reference
/// - `Not(term)` - Logical negation
/// - `And(...)` - Logical conjunction (2+ terms)
/// - `Or(...)` - Logical disjunction (2+ terms)
/// - `Eq(a, b)` - Equality (same sort required)
/// - `Ite(cond, then, else)` - If-then-else
///
/// # Example
///
/// ```rust
/// use logician::term::{Term, Sort};
///
/// let x = Term::Var("x".into(), Sort::Bool);
/// let y = Term::Var("y".into(), Sort::Bool);
///
/// // x ∧ y
/// let conjunction = x.and(y);
/// assert_eq!(conjunction.sort(), Sort::Bool);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    /// Boolean literal: `true` or `false`
    Bool(bool),
    /// Integer literal
    Int(i64),
    /// Variable with name and sort
    Var(String, Sort),
    /// Logical NOT (argument must be Bool)
    Not(Box<Term>),
    /// Logical AND (minimum 2 arguments, all Bool)
    And(And2, SmallVec<[Box<Term>; 2]>),
    /// Logical OR (minimum 2 arguments, all Bool)
    Or(Or2, SmallVec<[Box<Term>; 2]>),
    /// Equality (both arguments must have same sort)
    Eq(Box<Term>, Box<Term>),
    /// If-then-else (condition Bool, branches same sort)
    Ite(Box<Term>, Box<Term>, Box<Term>),
}

impl Term {
    /// Returns the sort of this term
    pub fn sort(&self) -> Sort {
        match self {
            Term::Bool(_) => Sort::Bool,
            Term::Int(_) => Sort::Int,
            Term::Var(_, sort) => *sort,
            Term::Not(_) => Sort::Bool,
            Term::And(_, _) => Sort::Bool,
            Term::Or(_, _) => Sort::Bool,
            Term::Eq(_, _) => Sort::Bool,
            Term::Ite(_, then_branch, _) => then_branch.sort(),
        }
    }
}

// # Spell: TermBuilders
// Fluent builders with sort gates

impl Term {
    /// Logical NOT (self must be Bool)
    #[allow(clippy::should_implement_trait)]  // We want `.not()` as a fluent builder, not std::ops::Not
    pub fn not(self) -> Term {
        crate::assert_invariant!(
            self.sort() == Sort::Bool,
            "not requires Bool sort",
            "term_not_sort"
        );
        Term::Not(Box::new(self))
    }
    
    /// Logical AND (both must be Bool)
    pub fn and(self, other: Term) -> Term {
        crate::assert_invariant!(
            self.sort() == Sort::Bool,
            "and requires Bool sort for self",
            "term_and_sort_self"
        );
        crate::assert_invariant!(
            other.sort() == Sort::Bool,
            "and requires Bool sort for other",
            "term_and_sort_other"
        );
        Term::And(And2(Box::new(self), Box::new(other)), SmallVec::new())
    }
    
    /// Logical OR (both must be Bool)
    pub fn or(self, other: Term) -> Term {
        crate::assert_invariant!(
            self.sort() == Sort::Bool,
            "or requires Bool sort for self",
            "term_or_sort_self"
        );
        crate::assert_invariant!(
            other.sort() == Sort::Bool,
            "or requires Bool sort for other",
            "term_or_sort_other"
        );
        Term::Or(Or2(Box::new(self), Box::new(other)), SmallVec::new())
    }
    
    /// Equality (both must have same sort)
    pub fn eq(self, other: Term) -> Term {
        crate::assert_invariant!(
            self.sort() == other.sort(),
            "eq requires matching sorts",
            "term_eq_sort"
        );
        Term::Eq(Box::new(self), Box::new(other))
    }
    
    /// Implication: self -> other (both must be Bool)
    pub fn implies(self, other: Term) -> Term {
        crate::assert_invariant!(
            self.sort() == Sort::Bool,
            "implies requires Bool sort for self",
            "term_implies_sort_self"
        );
        crate::assert_invariant!(
            other.sort() == Sort::Bool,
            "implies requires Bool sort for other",
            "term_implies_sort_other"
        );
        // a -> b  ===  !a | b
        self.not().or(other)
    }
    
    /// AND of self with multiple others (all must be Bool)
    pub fn and_many(self, others: Vec<Term>) -> Term {
        crate::assert_invariant!(
            self.sort() == Sort::Bool,
            "and_many requires Bool sort for self",
            "term_and_many_sort_self"
        );
        for (i, t) in others.iter().enumerate() {
            crate::assert_invariant!(
                t.sort() == Sort::Bool,
                &format!("and_many requires Bool sort for term {}", i),
                "term_and_many_sort_other"
            );
        }
        
        if others.is_empty() {
            // Just self, wrap in And with a true literal
            Term::And(And2(Box::new(self), Box::new(Term::Bool(true))), SmallVec::new())
        } else {
            let mut iter = others.into_iter();
            let second = iter.next().unwrap();
            let rest: SmallVec<[Box<Term>; 2]> = iter.map(Box::new).collect();
            Term::And(And2(Box::new(self), Box::new(second)), rest)
        }
    }
    
    /// OR of self with multiple others (all must be Bool)
    pub fn or_many(self, others: Vec<Term>) -> Term {
        crate::assert_invariant!(
            self.sort() == Sort::Bool,
            "or_many requires Bool sort for self",
            "term_or_many_sort_self"
        );
        for (i, t) in others.iter().enumerate() {
            crate::assert_invariant!(
                t.sort() == Sort::Bool,
                &format!("or_many requires Bool sort for term {}", i),
                "term_or_many_sort_other"
            );
        }
        
        if others.is_empty() {
            // Just self, wrap in Or with a false literal
            Term::Or(Or2(Box::new(self), Box::new(Term::Bool(false))), SmallVec::new())
        } else {
            let mut iter = others.into_iter();
            let second = iter.next().unwrap();
            let rest: SmallVec<[Box<Term>; 2]> = iter.map(Box::new).collect();
            Term::Or(Or2(Box::new(self), Box::new(second)), rest)
        }
    }
}

// # Spell: SmtSerializer
// Term to SMT-LIB string via Display

impl std::fmt::Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::Bool(true) => write!(f, "true"),
            Term::Bool(false) => write!(f, "false"),
            Term::Int(i) => write!(f, "{}", i),
            Term::Var(name, _) => write!(f, "{}", name),
            Term::Not(inner) => write!(f, "(not {})", inner),
            Term::And(And2(a, b), rest) => {
                write!(f, "(and {} {}", a, b)?;
                for t in rest {
                    write!(f, " {}", t)?;
                }
                write!(f, ")")
            }
            Term::Or(Or2(a, b), rest) => {
                write!(f, "(or {} {}", a, b)?;
                for t in rest {
                    write!(f, " {}", t)?;
                }
                write!(f, ")")
            }
            Term::Eq(a, b) => write!(f, "(= {} {})", a, b),
            Term::Ite(cond, then_b, else_b) => {
                write!(f, "(ite {} {} {})", cond, then_b, else_b)
            }
        }
    }
}

