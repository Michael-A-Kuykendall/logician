#!/usr/bin/env bash
set -e

cd "C:/Users/micha/repos/logician"

EPIC=$(bd create "Logician hardening: real invariant audit, no orphans, safe watchdog, genuine type-safety, honest tests" \
  --type epic --priority 0 --silent \
  --description "Remediate gaps from code review: (1) Invariant Superhighway contract test is a hollow stub and must actually verify every invariant tag is exercised (PPT follow-through); (2) fix orphan solver processes with Drop-based cleanup; (3) fix the watchdog so timeout is per-query and PID-reuse-safe; (4) fix the async feature (block_on-in-runtime panic) and async watchdog; (5) make the API genuinely type-safe via phantom Sort types (or correct the misleading type-safe claim); (6) make all tests non-vacuous and docs truthful." \
  --acceptance "- All sub-issues closed and CI green (cargo test with z3 installed)
- No orphan z3 processes after Solver drop
- Watchdog kills only on a real per-query timeout, never an unrelated PID
- API is compile-time sort-safe OR docs are corrected and consistent
- Every test asserts something real; no vacuous pass")

I1=$(bd create "Implement real invariant-coverage contract test (PPT follow-through)" \
  --type task --priority 1 --parent $EPIC --estimate 5 --silent \
  --labels "testing,invariants" \
  --description "Replace the hollow c_coverage_audit/c_global_audit tests with a real contract test that enumerates every assert_invariant! tag in src/ and asserts each is exercised by a representative driver. Maintain EXPECTED_INVARIANT_TAGS; adjust the macro so tags reflect actual evaluation. Remove or make-true fictional doc examples (README invariant audit snippet, invariant.rs doc)." \
  --acceptance $'- A contract test enumerates every assert_invariant! tag currently in src/ via a maintained EXPECTED_INVARIANT_TAGS list\n- Running the exercising driver populates 100% of EXPECTED tags (test fails if any missing)\n- Adding a new assert_invariant! tag without adding it to EXPECTED fails CI (compile/run)\n- No doc example references an invariant tag that is not actually asserted in tests')

I2=$(bd create "Add Drop impl to terminate solver process tree (no orphans)" \
  --type bug --priority 0 --parent $EPIC --estimate 3 --silent \
  --labels "process,bug" \
  --description "Implement Drop for Driver (and ensure Solver drops its driver) that terminates the child process tree via kill_tree, for both sync and tokio Child. Flush/drop stdin. Directly fixes the README promise of no orphan processes." \
  --acceptance $'- Dropping a Solver while z3 is running leaves no z3 process (test: after drop, child.try_wait()==Some)\n- Drop is safe if process already exited (no panic)\n- Async (tokio) Solver variant also cleaned on drop\n- kill_tree used to terminate the entire process tree')

I3=$(bd create "Redesign sync watchdog: per-query timeout with PID-safe kill" \
  --type task --priority 1 --parent $EPIC --estimate 5 --silent \
  --labels "process,watchdog" \
  --description "Replace the launch-time one-shot watchdog with a per-query watchdog: before each blocking query spawn a watchdog thread keyed to an Arc<AtomicBool> done flag + child id; on timeout only kill if child.try_wait()==Ok(None) (still alive) to avoid PID-reuse kills; after the query returns set done and join." \
  --acceptance $'- A single query exceeding config.timeout causes the solver to be killed\n- A query completing within timeout is NOT killed\n- Watchdog only kills when child.try_wait()==Ok(None) to avoid PID-reuse kills of unrelated processes\n- e_driver_watchdog asserts the process was actually terminated (try_wait==Some after timeout), not merely status.is_ok()')

I6=$(bd create "Fix async feature: eliminate block_on-in-runtime panic, proper async launch" \
  --type bug --priority 1 --parent $EPIC --estimate 5 --silent \
  --labels "async,bug" \
  --description "The tokio launch uses Handle::try_current().block_on(...) called from inside Solver::new().await, which panics under a running runtime. Replace with a natively async launch (tokio::process::Command::spawn via async fn) so the async API works inside tokio::main." \
  --acceptance $'- Solver::new(config).await inside a tokio::main runtime does NOT panic (no block_on-in-runtime)\n- Async launch uses tokio::process natively; no std runtime block_on\n- Existing async paths compile and run under a multi-threaded tokio runtime')

I4=$(bd create "Redesign async watchdog via tokio::time::timeout" \
  --type task --priority 2 --parent $EPIC --estimate 5 --silent \
  --labels "async,watchdog" \
  --description "For the tokio feature, wrap each query in tokio::time::timeout(timeout, ...); on elapse kill the child process tree (kill_tree on pid). Builds on the async-launch fix (I6)." \
  --acceptance $'- Async check exceeding timeout is aborted and process killed via tokio::time::timeout + kill_tree\n- Within-timeout async check succeeds\n- No orphan process after async timeout abort')

I5a=$(bd create "Type-safe Term<S>: Sort trait + Bool/Int markers + phantom-typed Term" \
  --type task --priority 0 --parent $EPIC --estimate 5 --silent \
  --labels "api,type-safety" \
  --description "Introduce a Sort trait with Bool and Int zero-sized marker types. Make Term carry a phantom sort parameter Term<S: Sort> so sort is known at compile time. Display and the parser are unaffected (parser yields a runtime Sort enum separately at the boundary)." \
  --acceptance $'- Sort is a trait; Bool and Int are zero-sized marker types\n- Term<S: Sort> carries phantom sort; sort() is compile-time, infallible\n- Display and parser unaffected (parser yields runtime Sort enum separately)')

I5b=$(bd create "Type-safe fluent builders: and/or/not/implies/eq/ite with Sort-checked signatures" \
  --type task --priority 1 --parent $EPIC --estimate 5 --silent \
  --labels "api,type-safety" \
  --description "Give the fluent builders typed signatures: and/or/not/implies require Term<Bool> and return Term<Bool>; eq requires matching sort Term<S>,Term<S> and returns Term<Bool>; ite requires cond Term<Bool> and same-sort branches and returns Term<S>. Sort mismatches become compile errors, not panics." \
  --acceptance $'- and/or/not/implies require Term<Bool> and return Term<Bool>\n- eq requires matching sort (Term<S>, Term<S>) and returns Term<Bool>\n- ite requires cond Term<Bool> and same-sort branches, returns Term<S>\n- Mismatched-sort combinations are compile errors, not panics')

I5c=$(bd create "Wire type-safe Term through solver/multisolver/parser/tests/docs" \
  --type task --priority 1 --parent $EPIC --estimate 5 --silent \
  --labels "api,type-safety" \
  --description "Update Solver.assert to take &Term<Bool>, and propagate the typed Term through multisolver, the SMT-LIB boundary, all tests, and README/doc examples. Remove the now-redundant sort-mismatch runtime invariants (compile-time enforced instead)." \
  --acceptance $'- solver.assert takes &Term<Bool>; declare/check/get_model updated\n- multisolver records typed terms and replays correctly\n- All existing functional tests pass; README/doc examples compile\n- Sort-mismatch runtime invariants removed (now compile-time)')

I7=$(bd create "Make all tests non-vacuous and docs truthful" \
  --type task --priority 2 --parent $EPIC --estimate 5 --silent \
  --labels "testing" \
  --description "Turn no-op tests into real ones (p_driver_construct, p_incremental_consistency must exercise behavior, not define unused nested fns). Integration tests requiring z3 must assert meaningful results; when z3 absent they should be ignore rather than silently vacuous. Remove fictional doc examples (README invariant-audit snippet, copilot-instructions 90.72 percent figure) or make them true." \
  --acceptance $'- p_driver_construct and p_incremental_consistency exercise real behavior (no no-op nested fns)\n- Integration tests needing z3 assert meaningful results; when z3 absent they are ignore, not silently vacuous\n- Fictional doc examples (README invariant audit, copilot-instructions coverage percent) removed or made true\n- cargo test passes in CI with z3 installed')

echo "EPIC=$EPIC I1=$I1 I2=$I2 I3=$I3 I6=$I6 I4=$I4 I5a=$I5a I5b=$I5b I5c=$I5c I7=$I7"

bd dep add $I1  $I5c
bd dep add $I3  $I2
bd dep add $I6  $I2
bd dep add $I4  $I6
bd dep add $I5b $I5a
bd dep add $I5c $I5b
bd dep add $I7  $I1
bd dep add $I7  $I2
bd dep add $I7  $I3
bd dep add $I7  $I4
bd dep add $I7  $I5c
bd dep add $I7  $I6

bd sync --flush-only
echo "=== GRAPH (layering) ==="
bd graph $EPIC --box 2>&1 | head -50
