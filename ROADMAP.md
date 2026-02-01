# Logician Roadmap

## Scope Definition

### What Logician Does (v0.1)

- **Term API**: Type-safe construction of Bool and Int terms
- **Sort Enforcement**: Runtime invariants that panic on misuse
- **Solver Communication**: stdin/stdout subprocess driver
- **Multi-Solver Fallback**: Try Z3, then CVC5, etc.
- **Process Watchdog**: Timeout and clean termination
- **Async Support**: Optional tokio feature

### What Logician Will Never Do

These are permanent architectural decisions, not "not yet" features:

- **FFI bindings** — We are a subprocess driver. Period.
- **Be a solver** — You bring Z3/CVC5/Yices; we drive it.
- **Advanced SMT-LIB features** — Arrays, bitvectors, quantifiers, uninterpreted functions are out of scope. This is a *simple* driver for *simple* problems.
- **Theory-specific reasoning** — No LIA/LRA optimization, no theory combination logic.
- **Solver installation** — Install your own solver.
- **GUI/TUI** — This is a library.

### Promises

1. **Free forever** — MIT licensed, no paid tiers, no feature-gating
2. **Stable API** — After 1.0, breaking changes only in major versions
3. **Subprocess only** — No C++ toolchain required, ever
4. **Type-safe terms** — Sort mismatches panic immediately with clear messages
5. **Clean termination** — No orphan solver processes

---

## Version Roadmap

### v0.1.0 — Current (Foundation)

✅ Core Term API (Bool, Int, And, Or, Not, Eq, Ite, Implies)  
✅ Single solver driver with timeout  
✅ Multi-solver fallback  
✅ Runtime invariant system  
✅ Property-based testing (proptest)  
✅ 90%+ test coverage  

### v0.2.0 — Polish (Planned)

- [ ] Real arithmetic (`Sort::Real`) for basic LRA problems
- [ ] Distinct (`Term::distinct(terms)`) — all-different constraint
- [ ] Let bindings (`Term::let_in(name, value, body)`) — expression sharing
- [ ] Push/pop scope validation (track depth, panic on negative)
- [ ] Model parsing improvements (handle more Z3/CVC5 output quirks)

### v0.3.0 — Hardening (Planned)

- [ ] Solver capability detection (query supported logics)
- [ ] Incremental assertion replay optimization
- [ ] Trace file replay tool (`logician-replay trace.smt2`)
- [ ] Better error messages (show offending term in sort errors)

### v1.0.0 — Stable (Target)

- [ ] API freeze
- [ ] MSRV policy (minimum supported Rust version)
- [ ] Long-term support commitment
- [ ] Comprehensive documentation book

---

## Not Planned (Out of Scope Forever)

| Feature | Why Not |
|---------|---------|
| Arrays | Complexity explosion, niche use case |
| Bitvectors | Use a dedicated BV library |
| Quantifiers | Changes the problem class entirely |
| Uninterpreted Functions | Requires theory solver integration |
| Proof production | Different tool (use native solver APIs) |
| Optimization | Use a dedicated optimizer |
| Parallel solving | Subprocess-per-solver is good enough |

---

## Decision Log

### 2026-02-01: No Arrays/BV/Quantifiers

**Decision**: Logician will not support arrays, bitvectors, or quantifiers.

**Rationale**: These features dramatically increase complexity for both implementation and users. The target audience is people with simple SAT/SMT problems who want type safety without FFI pain. Users with complex needs should use FFI bindings directly.

### 2026-02-01: Subprocess Only

**Decision**: Logician will never provide FFI bindings.

**Rationale**: FFI bindings already exist (z3, cvc5 crates). Our value is *avoiding* C++ compilation. If you need FFI performance, use those crates. If you want simplicity, use Logician.

---

## Contributing to the Roadmap

Logician is **open source, not open contribution**. Roadmap decisions are made by the maintainer.

If you have a use case that genuinely needs a feature not listed here, email [michaelallenkuykendall@gmail.com](mailto:michaelallenkuykendall@gmail.com) to discuss. Features that fit the scope *may* be considered.

Features that violate the "Will Never Do" list will not be considered.
