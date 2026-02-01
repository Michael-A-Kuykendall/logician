# Logician Development Instructions

This file provides context for AI assistants working on the Logician codebase.

## Project Overview

Logician is a **type-safe SMT solver driver** for Rust. It provides:
- Fluent Term API with compile-time sort safety via runtime invariants
- Multi-solver fallback (try Z3, then CVC5, etc.)
- Process watchdog with automatic timeout and cleanup
- cfg-gated async support (tokio feature)

---

## Architecture: Sorcery Doctrine

This project follows the **Sorcery** design doctrine where every behavior is locked to tests.

### Glyph Notation Summary

| Symbol | Meaning |
|:------:|---------|
| `#` | Spell Name - atomic unit of capability |
| `^` | Intent - the "why" |
| `@` | Component - function, struct, module |
| `:` | Contract - Input → Output |
| `>` | Dependency - implementation order |
| `$` | Obligation - requirements and proofs |
| `~` | Assumption - runtime truths assumed |
| `?` | Open Question - blocks casting |

### Obligation Types

- `$ require: fn name` - The artifact must exist
- `$ forbid: concept` - Negative space architecture
- `$ prove: behavior -> test: name` - Every claim maps to a test

### Spell File Location

The complete spell definitions are in `.internal/logician.spell`.

---

## Testing Strategy: PPT + Invariants

We use **Predictive Property-Based Testing** with runtime invariant tracking.

### Test Naming Convention

| Prefix | Type | Description |
|--------|------|-------------|
| `e_` | Exploration/Edge | Specific edge cases and panic tests |
| `p_` | Property | Property-based tests using proptest |
| `c_` | Contract | Permanent must-pass contract tests |

### Invariant System

```rust
use logician::assert_invariant;
use logician::invariant::{clear_invariant_log, get_invariant_tags};

// In code - records tag and enforces condition
assert_invariant!(x.sort() == Sort::Bool, "requires Bool", "term_and_sort_self");

// In tests - verify invariants were exercised
clear_invariant_log();
// ... run code ...
let tags = get_invariant_tags();
assert!(tags.contains("term_and_sort_self"));
```

### Running Tests

```bash
# Must use single thread due to global invariant state
cargo test -- --test-threads=1

# With coverage
cargo tarpaulin --skip-clean --ignore-tests -- --test-threads=1
```

**Current coverage: 90.72%** (342/377 lines)

---

## Module Structure

```
src/
├── lib.rs         # Crate root with module declarations
├── term.rs        # Term AST, Sort, LogicError, builders, serialization
├── solver.rs      # Stateful Solver session (sync/async)
├── parser.rs      # S-expression parser for solver output
├── driver.rs      # Process launch with watchdog
├── multisolver.rs # Multi-solver fallback orchestration
└── invariant.rs   # Runtime invariant tracking

tests/
└── mod.rs         # All tests organized by spell proof obligations

.internal/
├── logician.spell # Complete Sorcery spell definitions
└── ppt_invariant_guide.md # PPT testing methodology
```

---

## Key Design Decisions

### 1. Runtime Invariants vs Type-Level Constraints

We use runtime invariants (`assert_invariant!`) rather than complex type-level machinery because:
- Clearer error messages
- Auditable coverage via tag tracking
- Simpler code

### 2. And2/Or2 Wrapper Types

The `And2` and `Or2` types enforce minimum 2 arguments at the type level:
```rust
pub struct And2(pub Box<Term>, pub Box<Term>);
// Additional terms go in SmallVec alongside
Term::And(And2(t1, t2), rest)
```

### 3. cfg-gated Async

Async support is optional via the `tokio` feature:
```rust
#[cfg(not(feature = "tokio"))]
pub fn check(&mut self) -> Result<Response, LogicError> { ... }

#[cfg(feature = "tokio")]
pub async fn check(&mut self) -> Result<Response, LogicError> { ... }
```

### 4. No Regex in Parser

Per spell requirements (`$ forbid: regex`), the parser is hand-written recursive descent.

---

## Making Changes

1. **Check the spell first** - Any change should align with `.internal/logician.spell`
2. **Add invariants** - New logic paths need `assert_invariant!` with unique tags
3. **Add tests** - Every `$ prove:` requires a corresponding test
4. **Update documentation** - All public items need doc comments with examples
5. **Run full test suite** - `cargo test -- --test-threads=1`
6. **Check coverage** - `cargo tarpaulin --skip-clean --ignore-tests -- --test-threads=1`

---

## Open Source Model

This project is **open source but not open contribution**:
- MIT licensed
- Bug reports welcome via GitHub Issues
- PRs not accepted by default
- Contact maintainer before proposing changes

---

## Scope & Roadmap

See `ROADMAP.md` for the complete scope definition.

### In Scope (v0.1)
- Bool/Int terms with sort enforcement
- Solver subprocess management
- Multi-solver fallback
- Process watchdog

### Out of Scope (Forever)
- FFI bindings (subprocess only)
- Arrays, bitvectors, quantifiers
- Being a solver (you bring Z3/CVC5)
- Theory-specific reasoning

### Planned (v0.2+)
- Real arithmetic (Sort::Real)
- Distinct constraints
- Let bindings
- Improved model parsing

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| thiserror | Error derive macro |
| lazy_static | Global invariant tag storage |
| kill_tree | Process tree termination |
| smallvec | Inline storage for And/Or extra terms |
| tokio (optional) | Async runtime |

### Dev Dependencies

| Crate | Purpose |
|-------|---------|
| proptest | Property-based testing |
| criterion | Benchmarking |
| trybuild | Compile-fail tests |

---

## Supported Solvers

| Solver | Launch Args |
|--------|-------------|
| Z3 | `program: "z3", args: ["-in"]` |
| CVC5 | `program: "cvc5", args: ["--lang", "smt2"]` |
| Yices 2 | `program: "yices-smt2"` |

---

## Contact

**Maintainer**: Michael A. Kuykendall <michaelallenkuykendall@gmail.com>
