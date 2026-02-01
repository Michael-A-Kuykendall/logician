//! # Invariant Assertion and Tag Tracking System
//!
//! This module provides runtime invariant checking with tag tracking for audit purposes.
//!
//! ## Philosophy
//!
//! Traditional assertions crash on failure but leave no trace of what was checked.
//! Logician's invariant system:
//!
//! 1. **Records** every invariant tag that was evaluated
//! 2. **Panics** on violation with clear diagnostics
//! 3. **Enables auditing** - you can verify which invariants were actually exercised
//!
//! ## Usage
//!
//! ```rust
//! use logician::assert_invariant;
//! use logician::invariant::{clear_invariant_log, get_invariant_tags};
//!
//! // Clear before test
//! clear_invariant_log();
//!
//! // This records "my_check" and passes
//! assert_invariant!(true, "condition must hold", "my_check");
//!
//! // Verify the invariant was checked
//! let tags = get_invariant_tags();
//! assert!(tags.contains("my_check"));
//! ```
//!
//! ## Contract Testing
//!
//! The tag system enables contract tests that verify coverage:
//!
//! ```rust,ignore
//! #[test]
//! fn contract_all_invariants_checked() {
//!     // Run your logic...
//!     let tags = get_invariant_tags();
//!     assert!(tags.contains("term_and_sort_self"));
//!     assert!(tags.contains("term_or_sort_self"));
//!     // etc.
//! }
//! ```

use lazy_static::lazy_static;
use std::collections::HashSet;
use std::sync::Mutex;

lazy_static! {
    /// Global set of invariant tags that have been checked during execution.
    ///
    /// This is thread-safe but requires single-threaded test execution
    /// (`--test-threads=1`) for reliable contract testing.
    pub static ref INVARIANT_TAGS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

/// Clear all recorded invariant tags.
///
/// Call this at the start of each test to ensure a clean slate for contract verification.
///
/// # Example
///
/// ```rust
/// use logician::invariant::clear_invariant_log;
///
/// clear_invariant_log();
/// // Now run code that triggers invariants...
/// ```
pub fn clear_invariant_log() {
    INVARIANT_TAGS.lock().unwrap().clear();
}

/// Get a copy of all recorded invariant tags.
///
/// Returns all tags that have been checked since the last `clear_invariant_log()` call.
/// Use this in contract tests to verify that expected code paths were exercised.
///
/// # Example
///
/// ```rust
/// use logician::invariant::get_invariant_tags;
///
/// let tags = get_invariant_tags();
/// println!("Invariants checked: {:?}", tags);
/// ```
pub fn get_invariant_tags() -> HashSet<String> {
    INVARIANT_TAGS.lock().unwrap().clone()
}

/// Assert an invariant condition, recording the tag and panicking on failure.
///
/// This macro:
/// 1. Records the tag in the global `INVARIANT_TAGS` set
/// 2. Evaluates the condition
/// 3. Panics with a clear message if the condition is false
///
/// # Arguments
///
/// * `$cond` - Boolean expression to evaluate
/// * `$msg` - Human-readable error message if condition fails
/// * `$tag` - Unique identifier for this invariant (used in contract tests)
///
/// # Panics
///
/// Panics with format: `invariant violation [tag]: message`
///
/// # Example
///
/// ```rust
/// use logician::assert_invariant;
///
/// let x = 5;
/// assert_invariant!(x > 0, "x must be positive", "positive_x");
/// ```
#[macro_export]
macro_rules! assert_invariant {
    ($cond:expr, $msg:expr, $tag:expr) => {{
        $crate::invariant::INVARIANT_TAGS
            .lock()
            .unwrap()
            .insert($tag.to_string());
        if !$cond {
            panic!("invariant violation [{}]: {}", $tag, $msg);
        }
    }};
}
