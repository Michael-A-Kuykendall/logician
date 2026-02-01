//! # Logician
//!
//! A type-safe SMT solver driver for Rust.
//!
//! Logician provides a fluent API for building logical formulas and communicating
//! with SMT (Satisfiability Modulo Theories) solvers like Z3 and CVC5. Unlike
//! string-based libraries, Logician enforces sort constraints at runtime through
//! invariant checking, preventing malformed queries before they reach the solver.
//!
//! ## Quick Example
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
//! let mut solver = Solver::new(config).unwrap();
//! solver.declare("x", &Sort::Bool).unwrap();
//!
//! let x = Term::Var("x".into(), Sort::Bool);
//! solver.assert(&x).unwrap();
//!
//! match solver.check().unwrap() {
//!     Response::Sat => println!("Satisfiable!"),
//!     Response::Unsat => println!("Unsatisfiable!"),
//!     _ => {}
//! }
//! ```
//!
//! ## Modules
//!
//! - [`term`] - Type-safe Term AST with sort inference and fluent builders
//! - [`solver`] - Stateful SMT solver session management
//! - [`parser`] - S-expression parser for solver output
//! - [`driver`] - Process management with watchdog timeout
//! - [`multisolver`] - Multi-solver fallback orchestration
//! - [`invariant`] - Runtime assertion tracking system
//!
//! ## Features
//!
//! - **`tokio`** - Enable async API (default: synchronous std::io)

pub mod invariant;
pub mod term;
pub mod driver;
pub mod parser;
pub mod solver;
pub mod multisolver;
