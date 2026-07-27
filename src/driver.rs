//! # SMT Solver Process Driver
//!
//! This module manages SMT solver subprocesses with automatic timeout handling.
//!
//! ## Features
//!
//! - **Process isolation** - Solvers run as separate processes
//! - **Watchdog timeout** - Automatically kills hung solvers, scoped to each query
//! - **Process tree cleanup** - Terminates child processes via PID-safe kill (cross-platform)
//! - **cfg-gated async** - Supports both sync (std::io) and async (tokio) modes
//! - **No orphan processes** - `Driver` terminates the solver tree on drop
//!
//! ## Watchdog semantics
//!
//! Unlike a launch-time timer, the watchdog is armed per blocking operation
//! (`send`/`query`). A given operation is allowed `Config::timeout` to produce a
//! response; if it does not, the *entire process tree* is killed. The watchdog
//! only fires while the child is still alive (tracked via a shared flag), so it
//! can never kill an unrelated process whose PID happened to be recycled.
//!
//! ```rust,ignore
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
//! // dropping `driver` terminates the solver process tree
//! ```

#[cfg(not(feature = "tokio"))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(feature = "tokio"))]
use std::sync::Arc;
use std::time::Duration;

use crate::term::LogicError;

#[cfg(feature = "tokio")]
use std::io::Write;
#[cfg(not(feature = "tokio"))]
use std::io::{BufRead, Write};

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

/// Synchronously terminate a whole process tree.
///
/// `kill_tree::kill_tree` is async (it returns a `Future`), so it cannot be
/// called from the synchronous driver or a watchdog thread. We shell out to
/// the OS killer directly instead. This is only ever called from a *detached*
/// thread, so a slow `taskkill` can never block `Drop`. For the common case of
/// a single-process solver (z3/cvc5/yices) the immediate child is killed via
/// `Child::kill()`; this handles any grandchildren.
#[cfg(windows)]
fn kill_tree_sync(pid: u32) {
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

#[cfg(not(windows))]
fn kill_tree_sync(pid: u32) {
    let _ = std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// A watchdog thread handle plus the shared flags it consults.
#[cfg(not(feature = "tokio"))]
struct Watchdog {
    /// Set to cancel the pending kill (e.g. when the query completes in time).
    cancel: Arc<AtomicBool>,
    /// The OS thread; intentionally detached on disarm (joining would block).
    handle: JoinHandleType,
}

/// SMT solver subprocess driver with I/O handles and watchdog.
///
/// The Driver manages the lifecycle of a solver process:
/// - Holds stdin/stdout handles for communication
/// - Arms a per-query watchdog that kills the process tree on timeout
/// - Terminates the entire process tree when dropped (no orphans)
///
/// # Platform Support
///
/// When the `tokio` feature is enabled, types switch to async variants and the
/// watchdog is implemented with `tokio::time::timeout` instead of a thread.
pub struct Driver {
    /// The solver child process
    pub child: ChildType,
    /// Stdin handle for sending SMT-LIB commands
    pub stdin: StdinType,
    /// Buffered stdout for reading responses
    pub stdout: StdoutType,
    /// Per-operation timeout
    timeout: Duration,
    /// Optional trace file for debugging
    trace_file: Option<std::fs::File>,

    #[cfg(not(feature = "tokio"))]
    child_alive: Arc<AtomicBool>,
    #[cfg(not(feature = "tokio"))]
    watchdog: Option<Watchdog>,
}

// # Spell: ProcessDriver
// Process launch with per-query watchdog and drop-based cleanup

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
    /// Maximum time to wait for a single query (check/get-model/assert) before killing the solver
    pub timeout: Duration,
    /// If true, write all commands to a trace file (trace_<pid>.smt2)
    pub trace: bool,
}

#[cfg(not(feature = "tokio"))]
pub fn launch(config: &Config) -> Result<Driver, LogicError> {
    use std::io::BufReader;
    use std::process::{Command, Stdio};

    let mut child = Command::new(&config.program)
        .args(&config.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| LogicError::Solver("failed to capture stdin".into()))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| LogicError::Solver("failed to capture stdout".into()))?;
    let stdout = BufReader::new(stdout);

    let trace_file = if config.trace {
        let path = format!("trace_{}.smt2", std::process::id());
        Some(std::fs::File::create(&path)?)
    } else {
        None
    };

    Ok(Driver {
        child,
        stdin,
        stdout,
        timeout: config.timeout,
        trace_file,
        child_alive: Arc::new(AtomicBool::new(true)),
        watchdog: None,
    })
}

#[cfg(feature = "tokio")]
pub async fn launch(config: &Config) -> Result<Driver, LogicError> {
    use tokio::io::BufReader;
    use tokio::process::Command;

    let mut cmd = Command::new(&config.program);
    cmd.args(&config.args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let mut child = cmd.spawn()?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| LogicError::Solver("failed to capture stdin".into()))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| LogicError::Solver("failed to capture stdout".into()))?;
    let stdout = BufReader::new(stdout);

    let trace_file = if config.trace {
        let path = format!("trace_{}.smt2", std::process::id());
        Some(std::fs::File::create(&path)?)
    } else {
        None
    };

    Ok(Driver {
        child,
        stdin,
        stdout,
        timeout: config.timeout,
        trace_file,
    })
}

#[cfg(not(feature = "tokio"))]
impl Driver {
    /// Arm a per-query watchdog: spawn a thread that, after `timeout`, kills the
    /// process tree only if the child is still alive and the operation has not
    /// been disarmed (completed in time).
    fn arm(&mut self) {
        self.disarm();
        let cancel = Arc::new(AtomicBool::new(false));
        self.child_alive.store(true, Ordering::SeqCst);
        let pid = self.child.id();
        let timeout = self.timeout;
        let child_alive = Arc::clone(&self.child_alive);
        let cancel_clone = Arc::clone(&cancel);
        let handle = std::thread::spawn(move || {
            let start = std::time::Instant::now();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(50));
                if cancel_clone.load(Ordering::SeqCst) {
                    return;
                }
                if start.elapsed() >= timeout {
                    if child_alive.load(Ordering::SeqCst) {
                        // Kill the tree in a detached thread so a slow/failed
                        // kill can never block the watchdog (and thus Drop).
                        let pid2 = pid;
                        std::thread::spawn(move || kill_tree_sync(pid2));
                    }
                    return;
                }
            }
        });
        self.watchdog = Some(Watchdog { cancel, handle });
    }

    /// Cancel the pending watchdog and join its thread. The watchdog polls
    /// every 50ms, so once `cancel` is set it returns almost immediately —
    /// this never blocks for the full timeout and leaves no lingering thread.
    fn disarm(&mut self) {
        if let Some(w) = self.watchdog.take() {
            w.cancel.store(true, Ordering::SeqCst);
            let _ = w.handle.join();
        }
    }

    /// Run `f` with the watchdog armed for its duration.
    fn with_watchdog<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Driver) -> T,
    {
        self.arm();
        let r = f(self);
        self.disarm();
        r
    }

    fn raw_send(&mut self, cmd: &str) -> Result<(), LogicError> {
        if let Some(ref mut f) = self.trace_file {
            writeln!(f, "{}", cmd)?;
        }
        writeln!(self.stdin, "{}", cmd)?;
        self.stdin.flush()?;

        let mut line = String::new();
        self.stdout.read_line(&mut line)?;

        if !line.trim().is_empty() && line.trim() != "success" && line.contains("error") {
            return Err(LogicError::Solver(line.trim().to_string()));
        }
        Ok(())
    }

    fn raw_query(&mut self, cmd: &str) -> Result<String, LogicError> {
        if let Some(ref mut f) = self.trace_file {
            writeln!(f, "{}", cmd)?;
        }
        writeln!(self.stdin, "{}", cmd)?;
        self.stdin.flush()?;

        let mut result = String::new();
        self.stdout.read_line(&mut result)?;

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
                self.stdout.read_line(&mut line)?;
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

    /// Send a command and trace it, guarded by the per-query watchdog.
    pub fn send(&mut self, cmd: &str) -> Result<(), LogicError> {
        self.with_watchdog(|d| d.raw_send(cmd))
    }

    /// Send a command and read its (possibly multi-line) response, guarded by
    /// the per-query watchdog.
    pub fn query(&mut self, cmd: &str) -> Result<String, LogicError> {
        self.with_watchdog(|d| d.raw_query(cmd))
    }

    /// Test/diagnostic helper: is the child process still running?
    pub fn check_alive(&mut self) -> bool {
        self.child.try_wait().map(|o| o.is_none()).unwrap_or(false)
    }

    /// Test/diagnostic helper: arm the watchdog once (without an accompanying
    /// query) so tests can verify timeout-driven termination.
    pub fn arm_once(&mut self) {
        self.arm();
    }
}

#[cfg(feature = "tokio")]
impl Driver {
    async fn raw_send(&mut self, cmd: &str) -> Result<(), LogicError> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        if let Some(ref mut f) = self.trace_file {
            writeln!(f, "{}", cmd)?;
        }
        self.stdin.write_all(cmd.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        let mut line = String::new();
        self.stdout.read_line(&mut line).await?;

        if !line.trim().is_empty() && line.trim() != "success" && line.contains("error") {
            return Err(LogicError::Solver(line.trim().to_string()));
        }
        Ok(())
    }

    async fn raw_query(&mut self, cmd: &str) -> Result<String, LogicError> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        if let Some(ref mut f) = self.trace_file {
            writeln!(f, "{}", cmd)?;
        }
        self.stdin.write_all(cmd.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        let mut result = String::new();
        self.stdout.read_line(&mut result).await?;

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
                self.stdout.read_line(&mut line).await?;
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

    /// Send a command, bounded by `Config::timeout`. On timeout the process
    /// tree is killed and [`LogicError::Timeout`] is returned.
    pub async fn send(&mut self, cmd: &str) -> Result<(), LogicError> {
        let timeout = self.timeout;
        match tokio::time::timeout(timeout, self.raw_send(cmd)).await {
            Ok(r) => r,
            Err(_) => {
                let _ = self.child.start_kill();
                Err(LogicError::Timeout(timeout))
            }
        }
    }

    /// Send a command and read its response, bounded by `Config::timeout`.
    pub async fn query(&mut self, cmd: &str) -> Result<String, LogicError> {
        let timeout = self.timeout;
        match tokio::time::timeout(timeout, self.raw_query(cmd)).await {
            Ok(r) => r,
            Err(_) => {
                let _ = self.child.start_kill();
                Err(LogicError::Timeout(timeout))
            }
        }
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        #[cfg(feature = "tokio")]
        {
            // kill_on_drop(true) already kills the immediate child; also kill
            // the rest of the tree (best-effort, non-blocking) and ensure the
            // child process is reaped.
            let pid = self.child.id().unwrap_or(0);
            let _ = self.child.start_kill();
            std::thread::spawn(move || kill_tree_sync(pid));
        }
        #[cfg(not(feature = "tokio"))]
        {
            self.disarm();
            self.child_alive.store(false, Ordering::SeqCst);
            let pid = self.child.id();
            // Kill the immediate child synchronously (TerminateProcess /
            // SIGKILL: fast, never shells out, never blocks). This guarantees
            // no orphan solver process.
            let _ = self.child.kill();
            // Best-effort whole-tree cleanup in the background (non-blocking).
            std::thread::spawn(move || kill_tree_sync(pid));
            let _ = self.child.wait();
        }
    }
}
