//! # Stateful SMT Solver Session
//!
//! This module provides the [`Solver`] struct for interactive SMT solving.
//!
//! ## Features
//!
//! - **Incremental solving** - Assert constraints, check, add more, check again
//! - **Push/pop scopes** - Explore branches without re-sending everything
//! - **Model extraction** - Get satisfying assignments after SAT
//! - **Tracing** - Optional SMT-LIB trace file for debugging (owned by [`Driver`])
//! - **Cleanup** - Dropping the [`Solver`] terminates the solver process tree
//!
//! ## Example
//!
//! ```rust,ignore
//! use logician::driver::Config;
//! use logician::solver::Solver;
//! use logician::parser::Response;
//! use logician::term::{Term, Sort};
//! use std::time::Duration;
//!
//! let config = Config {
//!     program: "z3".into(),
//!     args: vec!["-in".into()],
//!     timeout: Duration::from_secs(30),
//!     trace: false,
//! };
//!
//! let mut solver = Solver::new(config).unwrap();
//! solver.declare("x", &Sort::Int).unwrap();
//!
//! let x = Term::Var("x".into(), Sort::Int);
//! solver.assert(&x.eq(Term::Int(42))).unwrap();
//!
//! if let Response::Sat = solver.check().unwrap() {
//!     if let Response::Model(bindings) = solver.get_model().unwrap() {
//!         println!("x = {:?}", bindings);
//!     }
//! }
//! # Ok::<(), logician::term::LogicError>(())
//! ```

use crate::driver::{launch, Config, Driver};
use crate::parser::{parse, Response};
use crate::term::{LogicError, Sort, Term};

/// Stateful SMT solver session.
///
/// A `Solver` manages a single solver process (via [`Driver`]) and provides
/// methods for declaring variables, asserting constraints, checking
/// satisfiability, and extracting models. Dropping the `Solver` terminates the
/// underlying solver process tree — there are no orphan processes.
pub struct Solver {
    /// The configuration used to launch this solver
    pub config: Config,
    /// The underlying process driver
    pub driver: Driver,
    /// Current push/pop scope depth (0 = base level)
    scope_depth: u32,
}

impl Solver {
    /// Create a new solver session
    #[cfg(not(feature = "tokio"))]
    pub fn new(config: Config) -> Result<Self, LogicError> {
        let driver = launch(&config)?;
        let mut solver = Solver {
            config,
            driver,
            scope_depth: 0,
        };
        solver.send("(set-option :print-success true)")?;
        solver.send("(set-logic ALL)")?;
        Ok(solver)
    }

    /// Create a new solver session (async version)
    #[cfg(feature = "tokio")]
    pub async fn new(config: Config) -> Result<Self, LogicError> {
        let driver = launch(&config).await?;
        let mut solver = Solver {
            config,
            driver,
            scope_depth: 0,
        };
        solver.send("(set-option :print-success true)").await?;
        solver.send("(set-logic ALL)").await?;
        Ok(solver)
    }

    /// Send a command (no response expected beyond `success`).
    #[cfg(not(feature = "tokio"))]
    fn send(&mut self, cmd: &str) -> Result<(), LogicError> {
        self.driver.send(cmd)
    }

    /// Send a command and return its (possibly multi-line) response.
    #[cfg(not(feature = "tokio"))]
    fn query(&mut self, cmd: &str) -> Result<String, LogicError> {
        self.driver.query(cmd)
    }

    /// Send a command (no response expected beyond `success`).
    #[cfg(feature = "tokio")]
    async fn send(&mut self, cmd: &str) -> Result<(), LogicError> {
        self.driver.send(cmd).await
    }

    /// Send a command and return its (possibly multi-line) response.
    #[cfg(feature = "tokio")]
    async fn query(&mut self, cmd: &str) -> Result<String, LogicError> {
        self.driver.query(cmd).await
    }

    /// Declare a constant
    #[cfg(not(feature = "tokio"))]
    pub fn declare(&mut self, name: &str, sort: &Sort) -> Result<(), LogicError> {
        let sort_str = match sort {
            Sort::Bool => "Bool",
            Sort::Int => "Int",
        };
        self.send(&format!("(declare-const {} {})", name, sort_str))
    }

    /// Declare a constant (async version)
    #[cfg(feature = "tokio")]
    pub async fn declare(&mut self, name: &str, sort: &Sort) -> Result<(), LogicError> {
        let sort_str = match sort {
            Sort::Bool => "Bool",
            Sort::Int => "Int",
        };
        self.send(&format!("(declare-const {} {})", name, sort_str))
            .await
    }

    /// Assert a term
    #[cfg(not(feature = "tokio"))]
    pub fn assert(&mut self, term: &Term) -> Result<(), LogicError> {
        self.send(&format!("(assert {})", term))
    }

    /// Assert a term (async version)
    #[cfg(feature = "tokio")]
    pub async fn assert(&mut self, term: &Term) -> Result<(), LogicError> {
        self.send(&format!("(assert {})", term)).await
    }

    /// Check satisfiability
    #[cfg(not(feature = "tokio"))]
    pub fn check(&mut self) -> Result<Response, LogicError> {
        parse(&self.query("(check-sat)")?)
    }

    /// Check satisfiability (async version)
    #[cfg(feature = "tokio")]
    pub async fn check(&mut self) -> Result<Response, LogicError> {
        parse(&self.query("(check-sat)").await?)
    }

    /// Get model (after sat)
    #[cfg(not(feature = "tokio"))]
    pub fn get_model(&mut self) -> Result<Response, LogicError> {
        parse(&self.query("(get-model)")?)
    }

    /// Get model (after sat) - async version
    #[cfg(feature = "tokio")]
    pub async fn get_model(&mut self) -> Result<Response, LogicError> {
        parse(&self.query("(get-model)").await?)
    }

    /// Report current push/pop scope depth.
    pub fn scope_depth(&self) -> u32 {
        self.scope_depth
    }

    /// Push scope
    #[cfg(not(feature = "tokio"))]
    pub fn push(&mut self, n: usize) -> Result<(), LogicError> {
        let n = n as u32;
        self.send(&format!("(push {})", n))?;
        self.scope_depth += n;
        Ok(())
    }

    /// Push scope (async version)
    #[cfg(feature = "tokio")]
    pub async fn push(&mut self, n: usize) -> Result<(), LogicError> {
        let n = n as u32;
        self.send(&format!("(push {})", n)).await?;
        self.scope_depth += n;
        Ok(())
    }

    /// Pop scope
    #[cfg(not(feature = "tokio"))]
    pub fn pop(&mut self, n: usize) -> Result<(), LogicError> {
        let n = n as u32;
        if n > self.scope_depth {
            return Err(LogicError::ScopeUnderflow {
                depth: self.scope_depth,
                requested: n,
            });
        }
        self.send(&format!("(pop {})", n))?;
        self.scope_depth -= n;
        Ok(())
    }

    /// Pop scope (async version)
    #[cfg(feature = "tokio")]
    pub async fn pop(&mut self, n: usize) -> Result<(), LogicError> {
        let n = n as u32;
        if n > self.scope_depth {
            return Err(LogicError::ScopeUnderflow {
                depth: self.scope_depth,
                requested: n,
            });
        }
        self.send(&format!("(pop {})", n)).await?;
        self.scope_depth -= n;
        Ok(())
    }

    /// Assert a term with a name for unsat-core tracking.
    ///
    /// The named assertion can be referenced in unsat-core output:
    /// `(assert (! term :named label))`
    #[cfg(not(feature = "tokio"))]
    pub fn assert_named(&mut self, term: &Term, label: &str) -> Result<(), LogicError> {
        self.send(&format!("(assert (! {} :named {}))", term, label))
    }

    /// Assert a term with a name for unsat-core tracking (async).
    #[cfg(feature = "tokio")]
    pub async fn assert_named(&mut self, term: &Term, label: &str) -> Result<(), LogicError> {
        self.send(&format!("(assert (! {} :named {}))", term, label))
            .await
    }

    /// Get the unsat core (list of named assertion labels) after an UNSAT result.
    #[cfg(not(feature = "tokio"))]
    pub fn get_unsat_core(&mut self) -> Result<Vec<String>, LogicError> {
        let raw = self.query("(get-unsat-core)")?;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed == "()" {
            return Ok(Vec::new());
        }
        // Parse S-expression list: (label1 label2 ...)
        let inner = trimmed
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(trimmed);
        let labels: Vec<String> = inner
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Ok(labels)
    }

    /// Get the unsat core (async).
    #[cfg(feature = "tokio")]
    pub async fn get_unsat_core(&mut self) -> Result<Vec<String>, LogicError> {
        let raw = self.query("(get-unsat-core)").await?;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed == "()" {
            return Ok(Vec::new());
        }
        let inner = trimmed
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(trimmed);
        let labels: Vec<String> = inner
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Ok(labels)
    }
}
