//! # SMT Solver Process Driver
//!
//! This module manages SMT solver subprocesses with automatic timeout handling.
//!
//! ## Features
//!
//! - **Process isolation** - Solvers run as separate processes
//! - **Watchdog timeout** - Automatically kills hung solvers
//! - **Process tree cleanup** - Uses `kill_tree` to terminate child processes
//! - **cfg-gated async** - Supports both sync (std::io) and async (tokio) modes
//!
//! ## Example
//!
//! ```rust,no_run
//! use logician::driver::{Config, launch};
//! use std::time::Duration;
//!
//! let config = Config {
//!     program: "z3".into(),
//!     args: vec!["-in".into()],
//!     timeout: Duration::from_secs(30),
//!     trace: false,
//! };
//!
//! let driver = launch(&config).expect("failed to launch Z3");
//! // driver.stdin and driver.stdout are ready for communication
//! ```

use std::time::Duration;
use crate::term::LogicError;

#[cfg(not(feature = "tokio"))]
pub type ChildType = std::process::Child;

#[cfg(feature = "tokio")]
pub type ChildType = tokio::process::Child;

#[cfg(not(feature = "tokio"))]
pub type StdinType = std::process::ChildStdin;

#[cfg(feature = "tokio")]
pub type StdinType = tokio::process::ChildStdin;

#[cfg(not(feature = "tokio"))]
pub type StdoutType = std::io::BufReader<std::process::ChildStdout>;

#[cfg(feature = "tokio")]
pub type StdoutType = tokio::io::BufReader<tokio::process::ChildStdout>;

#[cfg(not(feature = "tokio"))]
pub type JoinHandleType = std::thread::JoinHandle<()>;

#[cfg(feature = "tokio")]
pub type JoinHandleType = tokio::task::JoinHandle<()>;

/// SMT solver subprocess driver with I/O handles and watchdog.
///
/// The Driver manages the lifecycle of a solver process:
/// - Holds stdin/stdout handles for communication
/// - Runs a watchdog thread/task that kills the process on timeout
/// - Automatically cleans up the entire process tree
///
/// # Fields
///
/// - `child` - The subprocess handle
/// - `stdin` - Write handle for sending commands
/// - `stdout` - Buffered read handle for responses
/// - `watchdog_handle` - Background thread/task for timeout
///
/// # Platform Support
///
/// When the `tokio` feature is enabled, types switch to async variants.
pub struct Driver {
    /// The solver child process
    pub child: ChildType,
    /// Stdin handle for sending SMT-LIB commands
    pub stdin: StdinType,
    /// Buffered stdout for reading responses
    pub stdout: StdoutType,
    /// Watchdog that kills process tree on timeout
    pub watchdog_handle: Option<JoinHandleType>,
}

// # Spell: ProcessDriver
// Process launch with watchdog

/// Configuration for launching an SMT solver process.
///
/// # Example
///
/// ```rust
/// use logician::driver::Config;
/// use std::time::Duration;
///
/// // Z3 configuration
/// let z3 = Config {
///     program: "z3".into(),
///     args: vec!["-in".into()],
///     timeout: Duration::from_secs(30),
///     trace: false,
/// };
///
/// // CVC5 configuration
/// let cvc5 = Config {
///     program: "cvc5".into(),
///     args: vec!["--lang".into(), "smt2".into()],
///     timeout: Duration::from_secs(30),
///     trace: false,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct Config {
    /// Path or name of the solver executable (e.g., "z3", "/usr/bin/cvc5")
    pub program: String,
    /// Command-line arguments for the solver
    pub args: Vec<String>,
    /// Maximum time to wait before killing the solver
    pub timeout: Duration,
    /// If true, write all commands to a trace file (trace_<pid>.smt2)
    pub trace: bool,
}

/// Launch a solver process with watchdog timeout.
///
/// This function:
/// 1. Spawns the solver as a subprocess with piped stdin/stdout
/// 2. Starts a background watchdog that kills the process tree after timeout
/// 3. Returns a [`Driver`] with handles for communication
///
/// # Arguments
///
/// * `config` - Solver configuration including program path, args, and timeout
///
/// # Returns
///
/// A [`Driver`] with communication handles, or [`LogicError`] on failure.
///
/// # Errors
///
/// - [`LogicError::Io`] - Failed to spawn process
/// - [`LogicError::Solver`] - Failed to capture stdin/stdout
///
/// # Example
///
/// ```rust,no_run
/// use logician::driver::{Config, launch};
/// use std::time::Duration;
///
/// let config = Config {
///     program: "z3".into(),
///     args: vec!["-in".into()],
///     timeout: Duration::from_secs(5),
///     trace: false,
/// };
///
/// let driver = launch(&config)?;
/// # Ok::<(), logician::term::LogicError>(())
/// ```
#[cfg(not(feature = "tokio"))]
pub fn launch(config: &Config) -> Result<Driver, LogicError> {
    use std::process::{Command, Stdio};
    use std::io::BufReader;
    
    let mut child = Command::new(&config.program)
        .args(&config.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    
    let stdin = child.stdin.take().ok_or_else(|| {
        LogicError::Solver("failed to capture stdin".into())
    })?;
    
    let stdout = child.stdout.take().ok_or_else(|| {
        LogicError::Solver("failed to capture stdout".into())
    })?;
    let stdout = BufReader::new(stdout);
    
    // Spawn watchdog thread
    let pid = child.id();
    let timeout = config.timeout;
    let watchdog_handle = std::thread::spawn(move || {
        std::thread::sleep(timeout);
        // Kill process tree on timeout - ignore result, best effort
        drop(kill_tree::kill_tree(pid));
    });
    
    Ok(Driver {
        child,
        stdin,
        stdout,
        watchdog_handle: Some(watchdog_handle),
    })
}

#[cfg(feature = "tokio")]
pub fn launch(config: &Config) -> Result<Driver, LogicError> {
    use tokio::process::Command;
    use tokio::io::BufReader;
    
    // Note: This is a sync wrapper; actual async launch would be async fn
    let rt = tokio::runtime::Handle::try_current()
        .map_err(|_| LogicError::Solver("no tokio runtime".into()))?;
    
    let mut child = rt.block_on(async {
        Command::new(&config.program)
            .args(&config.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
    })?;
    
    let stdin = child.stdin.take().ok_or_else(|| {
        LogicError::Solver("failed to capture stdin".into())
    })?;
    
    let stdout = child.stdout.take().ok_or_else(|| {
        LogicError::Solver("failed to capture stdout".into())
    })?;
    let stdout = BufReader::new(stdout);
    
    // Spawn watchdog task
    let pid = child.id().unwrap_or(0);
    let timeout = config.timeout;
    let watchdog_handle = rt.spawn(async move {
        tokio::time::sleep(timeout).await;
        // Kill process tree on timeout - ignore result, best effort
        drop(kill_tree::kill_tree(pid));
    });
    
    Ok(Driver {
        child,
        stdin,
        stdout,
        watchdog_handle: Some(watchdog_handle),
    })
}
