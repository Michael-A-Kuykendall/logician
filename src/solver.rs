//! # Stateful SMT Solver Session
//!
//! This module provides the [`Solver`] struct for interactive SMT solving.
//!
//! ## Features
//!
//! - **Incremental solving** - Assert constraints, check, add more, check again
//! - **Push/pop scopes** - Explore branches without re-sending everything
//! - **Model extraction** - Get satisfying assignments after SAT
//! - **Tracing** - Optional SMT-LIB trace file for debugging
//!
//! ## Example
//!
//! ```rust,no_run
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
//! let mut solver = Solver::new(config)?;
//!
//! // Declare variable
//! solver.declare("x", &Sort::Int)?;
//!
//! // Assert constraint
//! let x = Term::Var("x".into(), Sort::Int);
//! solver.assert(&x.eq(Term::Int(42)))?;
//!
//! // Check and get model
//! if let Response::Sat = solver.check()? {
//!     if let Response::Model(bindings) = solver.get_model()? {
//!         println!("x = {:?}", bindings);
//!     }
//! }
//! # Ok::<(), logician::term::LogicError>(())
//! ```

use crate::driver::{Config, Driver, launch};
use crate::parser::{parse, Response};
use crate::term::{LogicError, Sort, Term};
use std::io::{BufRead, Write};

/// Stateful SMT solver session.
///
/// A `Solver` manages a single solver process and provides methods for:
/// - Declaring variables ([`declare`](Self::declare))
/// - Asserting constraints ([`assert`](Self::assert))
/// - Checking satisfiability ([`check`](Self::check))
/// - Extracting models ([`get_model`](Self::get_model))
/// - Managing assertion scopes ([`push`](Self::push), [`pop`](Self::pop))
///
/// # Lifecycle
///
/// 1. Create with [`Solver::new`] (spawns solver process)
/// 2. Declare variables
/// 3. Assert constraints
/// 4. Call [`check`](Self::check) to get SAT/UNSAT/Unknown
/// 5. If SAT, optionally call [`get_model`](Self::get_model)
/// 6. Solver is dropped when it goes out of scope
///
/// # Tracing
///
/// If `config.trace` is true, all SMT-LIB commands are written to `trace_<pid>.smt2`
/// for debugging.
pub struct Solver {
    /// The configuration used to launch this solver
    pub config: Config,
    /// The underlying process driver
    pub driver: Driver,
    /// Optional trace file for debugging
    pub trace_file: Option<std::fs::File>,
}

impl Solver {
    /// Create a new solver session
    #[cfg(not(feature = "tokio"))]
    pub fn new(config: Config) -> Result<Self, LogicError> {
        let trace_file = if config.trace {
            let path = format!("trace_{}.smt2", std::process::id());
            Some(std::fs::File::create(&path)?)
        } else {
            None
        };
        
        let driver = launch(&config)?;
        
        // Set solver options
        let mut solver = Solver { config, driver, trace_file };
        solver.send("(set-option :print-success true)")?;
        solver.send("(set-logic ALL)")?;
        
        Ok(solver)
    }
    
    /// Create a new solver session (async version)
    #[cfg(feature = "tokio")]
    pub async fn new(config: Config) -> Result<Self, LogicError> {
        let trace_file = if config.trace {
            let path = format!("trace_{}.smt2", std::process::id());
            Some(std::fs::File::create(&path)?)
        } else {
            None
        };
        
        let driver = launch(&config)?;
        
        let mut solver = Solver { config, driver, trace_file };
        solver.send("(set-option :print-success true)").await?;
        solver.send("(set-logic ALL)").await?;
        
        Ok(solver)
    }
    
    /// Send a command and trace it
    #[cfg(not(feature = "tokio"))]
    fn send(&mut self, cmd: &str) -> Result<(), LogicError> {
        // Trace
        if let Some(ref mut f) = self.trace_file {
            writeln!(f, "{}", cmd)?;
        }
        
        // Write to solver
        writeln!(self.driver.stdin, "{}", cmd)?;
        self.driver.stdin.flush()?;
        
        // Read response
        let mut line = String::new();
        self.driver.stdout.read_line(&mut line)?;
        
        if !line.trim().is_empty() && line.trim() != "success" {
            // Could be an error
            if line.contains("error") {
                return Err(LogicError::Solver(line.trim().to_string()));
            }
        }
        
        Ok(())
    }
    
    /// Send a command and trace it (async version)
    #[cfg(feature = "tokio")]
    async fn send(&mut self, cmd: &str) -> Result<(), LogicError> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        
        // Trace
        if let Some(ref mut f) = self.trace_file {
            writeln!(f, "{}", cmd)?;
        }
        
        // Write to solver
        self.driver.stdin.write_all(cmd.as_bytes()).await.map_err(|e| LogicError::Io(e))?;
        self.driver.stdin.write_all(b"\n").await.map_err(|e| LogicError::Io(e))?;
        self.driver.stdin.flush().await.map_err(|e| LogicError::Io(e))?;
        
        // Read response
        let mut line = String::new();
        self.driver.stdout.read_line(&mut line).await.map_err(|e| LogicError::Io(e))?;
        
        if !line.trim().is_empty() && line.trim() != "success" {
            if line.contains("error") {
                return Err(LogicError::Solver(line.trim().to_string()));
            }
        }
        
        Ok(())
    }
    
    /// Send command and get response
    #[cfg(not(feature = "tokio"))]
    fn query(&mut self, cmd: &str) -> Result<String, LogicError> {
        // Trace
        if let Some(ref mut f) = self.trace_file {
            writeln!(f, "{}", cmd)?;
        }
        
        // Write to solver
        writeln!(self.driver.stdin, "{}", cmd)?;
        self.driver.stdin.flush()?;
        
        // Read response - may be multiline for models
        let mut result = String::new();
        
        // First line
        self.driver.stdout.read_line(&mut result)?;
        
        // If it starts with '(' and doesn't end balanced, read more
        if result.trim().starts_with('(') {
            let mut depth = 0i32;
            for c in result.chars() {
                match c {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
            }
            while depth > 0 {
                let mut line = String::new();
                self.driver.stdout.read_line(&mut line)?;
                for c in line.chars() {
                    match c {
                        '(' => depth += 1,
                        ')' => depth -= 1,
                        _ => {}
                    }
                }
                result.push_str(&line);
            }
        }
        
        Ok(result)
    }
    
    /// Send command and get response (async version)
    #[cfg(feature = "tokio")]
    async fn query(&mut self, cmd: &str) -> Result<String, LogicError> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        
        // Trace
        if let Some(ref mut f) = self.trace_file {
            writeln!(f, "{}", cmd)?;
        }
        
        // Write to solver
        self.driver.stdin.write_all(cmd.as_bytes()).await.map_err(|e| LogicError::Io(e))?;
        self.driver.stdin.write_all(b"\n").await.map_err(|e| LogicError::Io(e))?;
        self.driver.stdin.flush().await.map_err(|e| LogicError::Io(e))?;
        
        // Read response
        let mut result = String::new();
        self.driver.stdout.read_line(&mut result).await.map_err(|e| LogicError::Io(e))?;
        
        if result.trim().starts_with('(') {
            let mut depth = 0i32;
            for c in result.chars() {
                match c {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
            }
            while depth > 0 {
                let mut line = String::new();
                self.driver.stdout.read_line(&mut line).await.map_err(|e| LogicError::Io(e))?;
                for c in line.chars() {
                    match c {
                        '(' => depth += 1,
                        ')' => depth -= 1,
                        _ => {}
                    }
                }
                result.push_str(&line);
            }
        }
        
        Ok(result)
    }
    
    /// Assert a term
    #[cfg(not(feature = "tokio"))]
    pub fn assert(&mut self, term: &Term) -> Result<(), LogicError> {
        let cmd = format!("(assert {})", term);
        self.send(&cmd)
    }
    
    /// Assert a term (async version)
    #[cfg(feature = "tokio")]
    pub async fn assert(&mut self, term: &Term) -> Result<(), LogicError> {
        let cmd = format!("(assert {})", term);
        self.send(&cmd).await
    }
    
    /// Declare a constant
    #[cfg(not(feature = "tokio"))]
    pub fn declare(&mut self, name: &str, sort: &Sort) -> Result<(), LogicError> {
        let sort_str = match sort {
            Sort::Bool => "Bool",
            Sort::Int => "Int",
        };
        let cmd = format!("(declare-const {} {})", name, sort_str);
        self.send(&cmd)
    }
    
    /// Declare a constant (async version)
    #[cfg(feature = "tokio")]
    pub async fn declare(&mut self, name: &str, sort: &Sort) -> Result<(), LogicError> {
        let sort_str = match sort {
            Sort::Bool => "Bool",
            Sort::Int => "Int",
        };
        let cmd = format!("(declare-const {} {})", name, sort_str);
        self.send(&cmd).await
    }
    
    /// Check satisfiability
    #[cfg(not(feature = "tokio"))]
    pub fn check(&mut self) -> Result<Response, LogicError> {
        let result = self.query("(check-sat)")?;
        parse(&result)
    }
    
    /// Check satisfiability (async version)
    #[cfg(feature = "tokio")]
    pub async fn check(&mut self) -> Result<Response, LogicError> {
        let result = self.query("(check-sat)").await?;
        parse(&result)
    }
    
    /// Get model (after sat)
    #[cfg(not(feature = "tokio"))]
    pub fn get_model(&mut self) -> Result<Response, LogicError> {
        let result = self.query("(get-model)")?;
        parse(&result)
    }
    
    /// Get model (after sat) - async version
    #[cfg(feature = "tokio")]
    pub async fn get_model(&mut self) -> Result<Response, LogicError> {
        let result = self.query("(get-model)").await?;
        parse(&result)
    }
    
    /// Push scope
    #[cfg(not(feature = "tokio"))]
    pub fn push(&mut self, n: usize) -> Result<(), LogicError> {
        let cmd = format!("(push {})", n);
        self.send(&cmd)
    }
    
    /// Push scope (async version)
    #[cfg(feature = "tokio")]
    pub async fn push(&mut self, n: usize) -> Result<(), LogicError> {
        let cmd = format!("(push {})", n);
        self.send(&cmd).await
    }
    
    /// Pop scope
    #[cfg(not(feature = "tokio"))]
    pub fn pop(&mut self, n: usize) -> Result<(), LogicError> {
        let cmd = format!("(pop {})", n);
        self.send(&cmd)
    }
    
    /// Pop scope (async version)
    #[cfg(feature = "tokio")]
    pub async fn pop(&mut self, n: usize) -> Result<(), LogicError> {
        let cmd = format!("(pop {})", n);
        self.send(&cmd).await
    }
}