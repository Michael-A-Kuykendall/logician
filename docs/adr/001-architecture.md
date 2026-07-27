# ADR 001: Sort Validation Strategy

**Status:** Accepted  
**Date:** 2026-07-27  
**Driver:** Honest architectural audit

## Context

Logician needs to validate that SMT terms are well-sorted (e.g., you can't AND a Bool with an Int). The question is *how* this validation should be enforced in the Rust API.

Two approaches exist:

- **A) Phantom-type encoding** — `Term<S: Sort>` where `S` is a zero-sized marker type (`Bool`, `Int`). Sort mismatches are compile-time type errors. The public API becomes `Term<Bool>`, `Term<Int>`, etc. Sort checking happens at compile time for zero runtime cost.

- **B) Runtime sort validation** — `Term` holds a `Sort` enum at runtime. Sort mismatches are detected at construction time via `assert_invariant!` and panic immediately with clear diagnostics. The invariant system tracks coverage for testing.

## Decision

**We choose B: Runtime sort validation.**

## Rationale

1. **API stability** — Approach A is a breaking change to every public API: `Term` becomes `Term<S>`, all function signatures change, `Solver::assert`, `parser::Response`, `multisolver::MultiSolver`, and every test and doc example must be rewritten. This crate is pre-1.0 but already has users; breaking changes should be deliberate and rare.

2. **Complexity cost** — Phantom types create a two-tier sort system: compile-time markers (`Bool`, `Int`) for the builder API and a runtime `Sort` enum for the parser boundary (where sorts arrive as strings). This duality is confusing and leaks into model parsing, error messages, and serialization.

3. **Marginal benefit** — The core value of Logician is subprocess SMT driving, not type-level correctness. Sort validation matters, but runtime panics at term construction time are caught immediately in testing and CI. The invariant audit system already ensures every sort check is exercised in tests.

4. **Simplicity** — A single `Sort` enum with runtime validation is straightforward to explain, implement, and debug. Error messages are clear: `assertion failed: "and requires Bool sort for other"`. Users see the problem and fix it.

## Consequences

- Sort mismatches will always be runtime panics, never compile-time errors.
- The invariant system (`assert_invariant!`) is a permanent part of the crate.
- Documentation must consistently describe this as "runtime sort validation" — never imply compile-time guarantees.
- The phantom-type approach remains a possible future direction for a hypothetical 2.0, but is not planned.

## Related

- `src/term.rs` — Sort enum and runtime validation
- `src/invariant.rs` — Assertion tracking and coverage auditing
- `tests/mod.rs::c_coverage_audit` — Contract test verifying every invariant is exercised
