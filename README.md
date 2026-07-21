<p align="center">
  <img src="https://raw.githubusercontent.com/Michael-A-Kuykendall/logician/master/assets/logician-logo.png" alt="Logician Logo" width="300">
</p>

<h1 align="center">Logician</h1>

<p align="center">
  <strong>Type-safe SMT solver driver for Rust</strong>
</p>

<p align="center">
  <a href="https://crates.io/crates/logician"><img src="https://img.shields.io/crates/v/logician.svg" alt="crates.io"></a>
  <a href="https://docs.rs/logician"><img src="https://docs.rs/logician/badge.svg" alt="Documentation"></a>
  <a href="https://github.com/Michael-A-Kuykendall/logician/actions"><img src="https://github.com/Michael-A-Kuykendall/logician/workflows/CI/badge.svg" alt="CI Status"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"></a>
  <a href="https://github.com/sponsors/Michael-A-Kuykendall"><img src="https://img.shields.io/badge/❤️-Sponsor-ea4aaa?logo=github" alt="Sponsor"></a>
</p>

<p align="center">
  <a href="#why-logician">Why Logician</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#features">Features</a> •
  <a href="#philosophy">Philosophy</a> •
  <a href="#sponsors">Sponsors</a>
</p>

---

**Logician is free forever.** No paid tiers. No enterprise upsells. No asterisks.

### 💝 Support Logician

🚀 **If Logician helps you, consider [sponsoring](https://github.com/sponsors/Michael-A-Kuykendall) — 100% of support goes to keeping it free forever.**

- **$5/month**: Coffee Hero ☕ — Eternal gratitude + name in [SPONSORS.md](SPONSORS.md)
- **$25/month**: Developer Supporter 🐛 — Priority bug response + roadmap influence
- **$100/month**: Corporate Backer 🏢 — Logo in README + release-note recognition
- **$500/month**: Enterprise Partner 🚀 — Prominent logo + monthly office hours + roadmap input

[**🎯 Become a Sponsor**](https://github.com/sponsors/Michael-A-Kuykendall) | See our amazing [sponsors](SPONSORS.md) 🙏

**Thank you to our sponsors:** [ZephyrCloudIO](https://github.com/ZephyrCloudIO) (Corporate Backer) · alistairheath (Coffee Hero)

---

## Why Logician?

SMT solvers are powerful. Getting them into your Rust project shouldn't require a PhD.

| Approach | Setup | Type Safety | Multi-Solver | Watchdog |
|----------|-------|-------------|--------------|----------|
| **FFI Bindings** | C++ toolchain, platform pain | Yes | Manual | Manual |
| **String Builders** | Easy | None—pray your strings parse | Manual | Manual |
| **Logician** | `cargo add logician` | **Yes, with clear panics** | **Built-in** | **Built-in** |

**What you get:**

- **Fluent Term API** — Build formulas in Rust, not strings. Sort mismatches panic immediately with actionable diagnostics.
- **Multi-solver fallback** — Z3 timed out? Logician automatically retries on CVC5.
- **Process watchdog** — Hung solver? Dead. Entire process tree terminated cleanly.
- **Optional async** — Enable `tokio` feature when you need it.

```rust
// This panics immediately: "and requires Bool sort for other"
let bad = bool_var.and(int_var);

// This works and serializes to valid SMT-LIB
let good = x.and(y.or(z));
```

No silent failures. No malformed queries reaching the solver. No orphan processes.

---

## Quick Start

```toml
[dependencies]
logician = "0.1"
```

You need an SMT solver in your PATH (e.g., [Z3](https://github.com/Z3Prover/z3)).

```rust
use logician::driver::Config;
use logician::solver::Solver;
use logician::parser::Response;
use logician::term::{Term, Sort};
use std::time::Duration;

fn main() -> Result<(), logician::term::LogicError> {
    let config = Config {
        program: "z3".into(),
        args: vec!["-in".into()],
        timeout: Duration::from_secs(30),
        trace: false,
    };
    
    let mut solver = Solver::new(config)?;
    
    solver.declare("x", &Sort::Bool)?;
    solver.declare("y", &Sort::Bool)?;
    
    let x = Term::Var("x".into(), Sort::Bool);
    let y = Term::Var("y".into(), Sort::Bool);
    let formula = x.and(y.not());
    
    solver.assert(&formula)?;
    
    match solver.check()? {
        Response::Sat => println!("Satisfiable!"),
        Response::Unsat => println!("Unsatisfiable!"),
        Response::Unknown => println!("Unknown"),
        _ => {}
    }
    
    Ok(())
}
```

---

## Features

### Type-Safe Terms

```rust
let a = Term::Var("a".into(), Sort::Bool);
let c = Term::Var("c".into(), Sort::Int);

// Works
let f1 = a.and(b);
let f2 = c.eq(Term::Int(42));

// Panics: "and requires Bool sort for other"
let bad = a.and(c);
```

### Multi-Solver Fallback

```rust
use logician::multisolver::MultiSolver;

let mut ms = MultiSolver::new(vec![z3_config, cvc5_config]);

ms.declare("x", &Sort::Bool);
ms.assert(&Term::Var("x".into(), Sort::Bool));

// Tries Z3 first; if it fails, replays everything on CVC5
match ms.check() {
    Ok(Response::Sat) => println!("Found solution"),
    Err(e) => println!("All solvers failed: {:?}", e),
}
```

### Process Watchdog

```rust
let config = Config {
    timeout: Duration::from_secs(5),  // Kill after 5 seconds
    // ...
};
```

Uses `kill_tree` to terminate the solver **and all child processes**.

### SMT-LIB Tracing

```rust
let config = Config {
    trace: true,  // Writes to trace_<pid>.smt2
    // ...
};
```

### Async Support

```toml
[dependencies]
logician = { version = "0.1", features = ["tokio"] }
```

```rust
let mut solver = Solver::new(config).await?;
solver.assert(&formula).await?;
let result = solver.check().await?;
```

---

## Philosophy

### The Invariant Superhighway

Logician doesn't just check for errors—it enforces architectural guarantees.

Every critical code path has runtime invariants that:

1. **Record** what was checked (for auditing)
2. **Panic immediately** on violations (no silent corruption)
3. **Enable contract testing** (verify the guards are watching)

```rust
// In code
assert_invariant!(term.sort() == Sort::Bool, "and requires Bool", "term_and_sort");

// In tests
let tags = get_invariant_tags();
assert!(tags.contains("term_and_sort"));
```

This is **Predictive Property-Based Testing (PPT)**—the same methodology used in high-reliability systems.

### What Logician Is

- A driver for SMT solvers via subprocess (stdin/stdout)
- Type-safe term construction with sort enforcement
- Multi-solver orchestration with automatic fallback
- Process lifecycle management with timeout handling

### What Logician Is Not

- Not FFI bindings (no C++ compilation required)
- Not a solver itself (you bring Z3, CVC5, Yices, etc.)
- Not a theorem prover framework
- Not pursuing advanced SMT-LIB features (arrays, bitvectors, quantifiers are not in scope)

See [ROADMAP.md](ROADMAP.md) for the full scope definition and planned features.

---

## Supported Solvers

| Solver | Configuration |
|--------|---------------|
| [Z3](https://github.com/Z3Prover/z3) | `program: "z3", args: ["-in"]` |
| [CVC5](https://cvc5.github.io/) | `program: "cvc5", args: ["--lang", "smt2"]` |
| [Yices 2](https://yices.csl.sri.com/) | `program: "yices-smt2"` |

Any SMT-LIB 2 compliant solver should work.

---

## Testing

```bash
# Single-threaded (required for global invariant state)
cargo test -- --test-threads=1

# With coverage
cargo tarpaulin --out Html
```

**Current: 24 tests, 90%+ coverage.**

---

## Sponsors

**If Logician saves you time, consider sponsoring development.**

<a href="https://github.com/sponsors/Michael-A-Kuykendall"><img src="https://img.shields.io/badge/❤️_Sponsor_on_GitHub-ea4aaa?style=for-the-badge&logo=github" alt="Sponsor on GitHub"></a>

| Tier | What You Get |
|------|--------------|
| **$5/month** Coffee Hero | My eternal gratitude + sponsor badge |
| **$25/month** Developer | Priority support + name in SPONSORS.md |
| **$100/month** Corporate | Logo on README + monthly office hours |
| **$500/month** Enterprise | Direct support + feature input |

**Companies**: Need invoicing? Email [michaelallenkuykendall@gmail.com](mailto:michaelallenkuykendall@gmail.com)

See [SPONSORS.md](SPONSORS.md) for the current sponsor list.

---

## Contributing

Logician is **open source but not open contribution**. See [CONTRIBUTING.md](CONTRIBUTING.md).

Bug reports via GitHub Issues are welcome. For security issues, see [SECURITY.md](SECURITY.md).

---

## License

MIT — see [LICENSE](LICENSE).

---

<p align="center">
  Built with 🦀 by <a href="https://github.com/Michael-A-Kuykendall">Michael A. Kuykendall</a>
</p>
