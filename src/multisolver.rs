//! # Multi-Solver with Automatic Fallback
//!
//! This module provides [`MultiSolver`] for trying multiple SMT solvers in sequence.
//!
//! ## Use Cases
//!
//! - **Reliability** - If Z3 times out, automatically try CVC5
//! - **Solver shopping** - Some problems are easier for certain solvers
//! - **Development** - Test against multiple solvers without code changes
//!
//! ## How It Works
//!
//! 1. Record all `declare` and `assert` calls
//! 2. On `check()`, try the first solver
//! 3. If it fails (timeout, error, crash), replay on the next solver
//! 4. Return first successful result, or error if all fail
//!
//! ## Example
//!
//! ```rust,no_run
//! use logician::multisolver::MultiSolver;
//! use logician::driver::Config;
//! use logician::term::{Term, Sort};
//! use logician::parser::Response;
//! use std::time::Duration;
//!
//! let z3 = Config {
//!     program: "z3".into(),
//!     args: vec!["-in".into()],
//!     timeout: Duration::from_secs(5),
//!     trace: false,
//! };
//!
//! let cvc5 = Config {
//!     program: "cvc5".into(),
//!     args: vec!["--lang".into(), "smt2".into()],
//!     timeout: Duration::from_secs(5),
//!     trace: false,
//! };
//!
//! let mut ms = MultiSolver::new(vec![z3, cvc5]);
//!
//! ms.declare("x", &Sort::Bool);
//! ms.assert(&Term::Var("x".into(), Sort::Bool));
//!
//! // Tries Z3 first, falls back to CVC5 if needed
//! match ms.check() {
//!     Ok(Response::Sat) => println!("Satisfiable!"),
//!     Ok(Response::Unsat) => println!("Unsatisfiable!"),
//!     Err(e) => println!("All solvers failed: {:?}", e),
//!     _ => {}
//! }
//! ```

use crate::driver::Config;
use crate::parser::Response;
use crate::solver::Solver;
use crate::term::{LogicError, Sort, Term};

/// Multi-solver with automatic fallback across solver backends.
///
/// `MultiSolver` records all declarations and assertions, then replays them
/// on each solver in sequence until one succeeds.
///
/// # Replay Semantics
///
/// When `check()` is called:
/// 1. Launch the first solver
/// 2. Replay all `declare` calls
/// 3. Replay all `assert` calls
/// 4. Call `check-sat`
/// 5. If successful, return the result
/// 6. If failed, move to next solver and repeat
///
/// # Thread Safety
///
/// `MultiSolver` is not thread-safe. Each instance should be used from a single thread.
pub struct MultiSolver {
    /// Solver configurations to try in order
    pub configs: Vec<Config>,
    /// Recorded assertions to replay on fallback
    pub asserts: Vec<Term>,
    /// Recorded declarations to replay on fallback
    pub declares: Vec<(String, Sort)>,
}

impl MultiSolver {
    /// Create a new MultiSolver with a list of solver configs
    pub fn new(configs: Vec<Config>) -> Self {
        MultiSolver {
            configs,
            asserts: Vec::new(),
            declares: Vec::new(),
        }
    }
    
    /// Record an assertion (to be replayed on fallback)
    pub fn assert(&mut self, term: &Term) {
        self.asserts.push(term.clone());
    }
    
    /// Record a declaration (to be replayed on fallback)
    pub fn declare(&mut self, name: &str, sort: &Sort) {
        self.declares.push((name.to_string(), *sort));
    }
    
    /// Check satisfiability, trying solvers in order with fallback
    #[cfg(not(feature = "tokio"))]
    pub fn check(&mut self) -> Result<Response, LogicError> {
        if self.configs.is_empty() {
            return Err(LogicError::Solver("no solver configs provided".into()));
        }
        
        let mut last_error = None;
        let max_attempts = self.configs.len();
        
        for (idx, config) in self.configs.iter().enumerate() {
            // Prevent infinite loop - only try each config once
            if idx >= max_attempts {
                break;
            }
            
            match Solver::new(config.clone()) {
                Ok(mut solver) => {
                    // Replay declares
                    let mut setup_failed = false;
                    for (name, sort) in &self.declares {
                        if let Err(e) = solver.declare(name, sort) {
                            last_error = Some(e);
                            setup_failed = true;
                            break;
                        }
                    }
                    if setup_failed { continue; }
                    
                    // Replay asserts
                    for term in &self.asserts {
                        if let Err(e) = solver.assert(term) {
                            last_error = Some(e);
                            setup_failed = true;
                            break;
                        }
                    }
                    if setup_failed { continue; }
                    
                    // Check
                    match solver.check() {
                        Ok(response) => return Ok(response),
                        Err(e) => {
                            last_error = Some(e);
                            // Fall through to try next solver
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                    // Fall through to try next solver
                }
            }
        }
        
        // All solvers failed
        Err(last_error.unwrap_or_else(|| LogicError::Solver("all solvers failed".into())))
    }
    
    /// Check satisfiability, trying solvers in order with fallback (async version)
    #[cfg(feature = "tokio")]
    pub async fn check(&mut self) -> Result<Response, LogicError> {
        if self.configs.is_empty() {
            return Err(LogicError::Solver("no solver configs provided".into()));
        }
        
        let mut last_error = None;
        let max_attempts = self.configs.len();
        
        for (idx, config) in self.configs.iter().enumerate() {
            // Prevent infinite loop - only try each config once
            if idx >= max_attempts {
                break;
            }
            
            match Solver::new(config.clone()).await {
                Ok(mut solver) => {
                    // Replay declares
                    let mut failed = false;
                    for (name, sort) in &self.declares {
                        if let Err(e) = solver.declare(name, sort).await {
                            last_error = Some(e);
                            failed = true;
                            break;
                        }
                    }
                    if failed { continue; }
                    
                    // Replay asserts
                    for term in &self.asserts {
                        if let Err(e) = solver.assert(term).await {
                            last_error = Some(e);
                            failed = true;
                            break;
                        }
                    }
                    if failed { continue; }
                    
                    // Check
                    match solver.check().await {
                        Ok(response) => return Ok(response),
                        Err(e) => {
                            last_error = Some(e);
                            // Fall through to try next solver
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                    // Fall through to try next solver
                }
            }
        }
        
        // All solvers failed
        Err(last_error.unwrap_or_else(|| LogicError::Solver("all solvers failed".into())))
    }
}