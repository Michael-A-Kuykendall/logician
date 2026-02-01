# Security Policy

## Supported Versions

We actively support the following versions of Logician with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take the security of Logician seriously. If you discover a security vulnerability, please follow these guidelines:

### 🔒 Private Disclosure Process

**DO NOT** create a public GitHub issue for security vulnerabilities.

Instead, please report security issues privately:

1. **Direct Email**
   - Send details to: michaelallenkuykendall@gmail.com
   - Include "SECURITY: Logician" in the subject line

### 📝 What to Include

Please provide the following information in your report:

- **Description**: Clear description of the vulnerability
- **Impact**: What could an attacker accomplish?
- **Reproduction**: Step-by-step instructions to reproduce the issue
- **Environment**:
  - Logician version
  - Operating system
  - Rust version
  - SMT solver used (if applicable)
- **Proof of Concept**: Code or logs demonstrating the issue
- **Suggested Fix**: If you have ideas for remediation

### ⏱️ Response Timeline

We aim to respond to security reports according to the following timeline:

- **Initial Response**: Within 48 hours of report
- **Triage**: Within 7 days - confirm/deny vulnerability
- **Resolution**: Within 30 days for critical issues, 90 days for others
- **Disclosure**: Public disclosure after fix is released

### ⚠️ Vulnerability Severity Guidelines

#### Critical
- Arbitrary code execution via malformed solver output
- Memory corruption in parser
- Process escape from watchdog

#### High
- Denial of service via crafted input
- Resource exhaustion attacks
- Unintended process tree leaks

#### Medium
- Information disclosure
- Panic in safe Rust code
- Resource leaks

#### Low
- Issues requiring local access
- Minor information leaks
- Performance degradation

### 🏆 Recognition

We believe in recognizing security researchers who help keep Logician secure:

- Public recognition in security acknowledgments
- Credit in release notes

*Note: We currently do not offer monetary bug bounties, but we deeply appreciate responsible disclosure.*

## Security Features

Logician includes several built-in security features:

- **Memory Safety**: Built with safe Rust
- **Process Isolation**: SMT solvers run as isolated subprocesses
- **Watchdog Protection**: Automatic timeout and process tree termination via `kill_tree`
- **No Unsafe Code**: Pure safe Rust in all paths

## Contact

For non-security related issues, please use:
- GitHub Issues: https://github.com/Michael-A-Kuykendall/logician/issues

---

*This security policy is effective as of February 2026 and may be updated periodically.*
