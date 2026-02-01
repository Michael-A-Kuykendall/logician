# Contributing to Logician

## Open Source, Not Open Contribution

Logician is **open source** but **not open contribution**.

- The code is freely available under the MIT license
- You can fork, modify, use, and learn from it without restriction
- **Pull requests are not accepted by default**
- All architectural, roadmap, and merge decisions are made by the project maintainer

This model keeps the project coherent, maintains clear ownership, and ensures consistent quality. It's the same approach used by SQLite and many infrastructure projects.

## How to Contribute

If you believe you can contribute meaningfully to Logician:

1. **Email the maintainer first**: [michaelallenkuykendall@gmail.com](mailto:michaelallenkuykendall@gmail.com)
2. Describe your background and proposed contribution
3. If there is alignment, a scoped collaboration may be discussed privately
4. Only after discussion will PRs be considered

**Unsolicited PRs will be closed without merge.** This isn't personal, it's how this project operates.

## What We Welcome (via email first)

- Bug reports with detailed reproduction steps (Issues are fine)
- Security vulnerability reports (please email directly)
- Documentation improvements (discuss first)
- Solver-specific bug fixes (discuss first)

## What We Handle Internally

- New features and architectural changes
- API design decisions
- Dependency updates
- Performance optimizations
- Multi-solver compatibility work

## Bug Reports

Bug reports via GitHub Issues are welcome! Please include:

- Operating system and version
- Rust version and logician version
- SMT solver(s) used (Z3, CVC5, etc.) and version
- Minimal reproduction case
- Expected vs actual behavior

## Code Style (for reference)

If a contribution is discussed and approved:

- Rust 2021 edition with `cargo fmt` and `cargo clippy`
- Comprehensive error handling using `Result<T, LogicError>`
- All public APIs must have documentation with examples
- All behavior must be covered by property-based and contract tests

## Logician Philosophy

Any accepted work must align with:

- **Type Safety First**: Compile-time sort enforcement via the Term API
- **Runtime Invariants**: All assumptions tracked and auditable
- **Production Ready**: Comprehensive error handling and testing
- **Free Forever**: No features that could lead to paid tiers

## Why This Model?

Building reliable SMT solver infrastructure requires tight architectural control. This ensures:

- Consistent API design across all solver backends
- No ownership disputes or governance overhead
- Quality control without committee delays
- Clear direction for the project's future

The code is open. The governance is centralized. This is intentional.

## Recognition

Helpful bug reports and community feedback are acknowledged in release notes.
If email collaboration leads to merged work, attribution will be given appropriately.

---

**Maintainer**: Michael A. Kuykendall
