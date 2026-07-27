# Changelog

All notable changes to Logician are recorded in this file.

The format is based on [Keep a Changelog](https://keepachangelog.org/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.0] — 2026-07-27

### Added

- **S-expression tokenizer** (`src/tokenizer.rs`) — proper lexical scanner with position tracking, error recovery, comment handling, support for quoted strings, pipe-delimited symbols, and numeric literals. Unit-tested against 8 edge-case scenarios including unterminated strings, pipe symbols, and comments.
- **Structured response types** — `Response::Eval` and `Response::GetInfo` variants added; `Response` is now `#[non_exhaustive]` with an `SExpr` AST type for extensibility.
- **Push/pop scope tracking** — `Solver::scope_depth()`, validated underflow with `LogicError::ScopeUnderflow`, replayed through `MultiSolver`.
- **Unsat core API** — `Solver::assert_named(term, label)` and `Solver::get_unsat_core()` for both sync and async paths.
- **Cross-solver CI matrix** — GitHub Actions tests Z3 on ubuntu/windows/macos, CVC5 and Yices2 on ubuntu, in both default and tokio feature configurations.
- **Architecture ADR** — `docs/adr/001-architecture.md` documents the decision to use runtime sort validation (not phantom types).
- **Error quality audit** — all `unwrap()` calls in production code replaced with `expect()` including context messages; `Mutex` lock errors given descriptive panic messages.
- **Language links** — README includes Chinese translations (简体中文, 繁體中文) with corresponding README files.

### Changed

- **Tagline** — "Type-safe SMT solver driver" → "Sort-checked SMT solver driver" (honest description of runtime validation approach).
- **Documentation consistency** — all `type-safe` references in source code, docs, and comments replaced with `sort-checked` or `sort-validated`.
- **`kill_tree` crate dependency removed** — replaced with a cross-platform `kill_tree_sync` that uses `taskkill` on Windows and `kill -9` on Unix.
- **README honesty** — removed fake "90%+ coverage" claim, removed "Invariant Superhighway" hype, removed references to nonexistent `.internal/` files.

### Fixed

- **Async build** — `p_driver_construct` gated under `#[cfg(not(feature = "tokio"))]` since sync `launch` doesn't work under tokio.
- **Missing `silent_hang_config`** test helper restored, enabling async watchdog tests to pass.
- **Dependabot vulnerabilities** — `bytes` updated 1.11.0 → 1.12.1, `rand` updated 0.9.2 → 0.9.5.

---

## [0.1.0] — 2026-02-01

### Added

- Initial public release: fluent Term API with Bool/Int sorts, multi-solver fallback, process watchdog, SMT-LIB tracing, async tokio support.
- Basic invariant assertion system (`assert_invariant!` macro).
- Property-based testing with proptest.
