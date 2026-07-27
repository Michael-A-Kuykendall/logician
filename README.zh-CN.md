<p align="center">
  <img src="https://raw.githubusercontent.com/Michael-A-Kuykendall/logician/master/assets/logician-logo.png" alt="Logician Logo" width="300">
</p>

<h1 align="center">Logician</h1>

<p align="center">
  <strong>面向 Rust 的排序检查 SMT 求解器驱动</strong>
</p>

<p align="center">
  <a href="README.zh-CN.md">简体中文</a> • <a href="README.zh-TW.md">繁體中文</a> • <a href="README.md">English</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/logician"><img src="https://img.shields.io/crates/v/logician.svg" alt="crates.io"></a>
  <a href="https://docs.rs/logician"><img src="https://docs.rs/logician/badge.svg" alt="Documentation"></a>
  <a href="https://github.com/Michael-A-Kuykendall/logician/actions"><img src="https://github.com/Michael-A-Kuykendall/logician/workflows/CI/badge.svg" alt="CI Status"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"></a>
  <a href="https://github.com/sponsors/Michael-A-Kuykendall"><img src="https://img.shields.io/badge/❤️-Sponsor-ea4aaa?logo=github" alt="Sponsor"></a>
</p>

<p align="center">
  <a href="#为什么选择-logician">为什么选择 Logician</a> •
  <a href="#快速开始">快速开始</a> •
  <a href="#功能特性">功能特性</a> •
  <a href="#设计理念">设计理念</a> •
  <a href="#赞助商">赞助商</a>
</p>

---

## 为什么选择 Logician？

SMT 求解器功能强大，但将其集成到 Rust 项目中不应需要博士学位。

| 方案 | 配置 | 类型安全 | 多求解器 | 看门狗 |
|------|------|----------|----------|--------|
| **FFI 绑定** | C++ 工具链，平台依赖 | 是 | 手动 | 手动 |
| **字符串构造** | 简单 | 无——祈祷字符串能解析 | 手动 | 手动 |
| **Logician** | `cargo add logician` | **运行时：排序不匹配时 panic** | **内置** | **内置** |

**核心特性：**

- **流畅的 Term API** — 用 Rust 构建公式，而非字符串。排序不匹配立即 panic，附带可操作的诊断信息。
- **多求解器自动切换** — Z3 超时？Logician 自动切换到 CVC5 重试。
- **进程看门狗** — 求解器挂死？立即终止，整个进程树被干净清理。
- **可选异步支持** — 需要时启用 `tokio` 特性。

```rust
// 立即 panic："and 要求另一个参数为 Bool 排序"
let bad = bool_var.and(int_var);

// 正常工作，序列化为有效的 SMT-LIB
let good = x.and(y.or(z));
```

无静默失败。无格式错误的查询到达求解器。无孤儿进程。

---

## 快速开始

```toml
[dependencies]
logician = "0.1"
```

你需要在 PATH 中安装 SMT 求解器（例如 [Z3](https://github.com/Z3Prover/z3)）。

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
        Response::Sat => println!("可满足！"),
        Response::Unsat => println!("不可满足！"),
        Response::Unknown => println!("未知"),
        _ => {}
    }

    Ok(())
}
```

---

## 功能特性

### 排序检查的 Term（运行时，非编译时）

```rust
let a = Term::Var("a".into(), Sort::Bool);
let c = Term::Var("c".into(), Sort::Int);

// 正常工作
let f1 = a.and(b);
let f2 = c.eq(Term::Int(42));

// Panic："and 要求另一个参数为 Bool 排序"
let bad = a.and(c);
```

### 多求解器自动切换

```rust
use logician::multisolver::MultiSolver;

let mut ms = MultiSolver::new(vec![z3_config, cvc5_config]);
ms.declare("x", &Sort::Bool);
ms.assert(&Term::Var("x".into(), Sort::Bool));
match ms.check() {
    Ok(Response::Sat) => println!("找到解"),
    Err(e) => println!("所有求解器均失败：{:?}", e),
}
```

### 进程看门狗

```rust
let config = Config {
    timeout: Duration::from_secs(5),
    // ...
};
```

通过 PID 安全的轮询和 kill 终止求解器进程树。

### SMT-LIB 跟踪

```rust
let config = Config {
    trace: true,
    // ...
};
```

### 异步支持

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

## 设计理念

### 运行时不变量（非编译时类型安全）

Logician 通过运行时不变量（`assert_invariant!`）而非类型级机制来强制排序正确性：

1. **记录**每个排序检查的唯一标签（用于审计）
2. **违反时 panic**（无静默损坏）
3. **审计覆盖率** — `tests/mod.rs::c_invariant_audit` 枚举每个标签，
   如果某个不变量停止被测试，构建失败

这是运行时检查：排序不匹配在构造 term 时 panic，而非编译时。权衡是简单性优先于编译时保证。

### Logician 是什么

- SMT 求解器的子进程驱动（stdin/stdout）
- 带排序检查的 term 构造
- 多求解器编排与自动切换
- 带超时处理的进程生命周期管理

### Logician 不是什么

- 非 FFI 绑定（无需 C++ 编译）
- 非求解器本身（你需要 Z3、CVC5、Yices 等）
- 非定理证明框架
- 不追求高级 SMT-LIB 特性（数组、位向量、量词不在范围内）

---

## 支持的求解器

| 求解器 | 配置 |
|--------|------|
| [Z3](https://github.com/Z3Prover/z3) | `program: "z3", args: ["-in"]` |
| [CVC5](https://cvc5.github.io/) | `program: "cvc5", args: ["--lang", "smt2"]` |
| [Yices 2](https://yices.csl.sri.com/) | `program: "yices-smt2"` |

任何符合 SMT-LIB 2 标准的求解器均可使用。

---

## 测试

```bash
# 单线程（全局不变量状态需要）
cargo test -- --test-threads=1

# 含覆盖率
cargo tarpaulin --out Html
```

**当前：26 个测试（默认）/ 18 个测试（tokio）。**

---

## 赞助商

**如果 Logician 为你节省了时间，请考虑赞助开发。**

| 等级 | 权益 |
|------|------|
| **$5/月** 咖啡英雄 | 衷心感谢 + 赞助徽章 |
| **$25/月** 开发者支持 | 优先支持 + SPONSORS.md 中署名 |
| **$100/月** 企业支持 | README 中显示 Logo + 月度会议 |
| **$500/月** 企业合作伙伴 | 直接支持 + 功能建议权 |

**公司用户**：需要发票？请发邮件至 michaelallenkuykendall@gmail.com

---

## 贡献

Logician 是**开源但非开放贡献**的模式。详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

欢迎通过 GitHub Issues 提交错误报告。安全问题请参阅 [SECURITY.md](SECURITY.md)。

---

## 许可证

MIT — 详见 [LICENSE](LICENSE)。

---

<p align="center">
  用 🦀 构建，作者 <a href="https://github.com/Michael-A-Kuykendall">Michael A. Kuykendall</a>
</p>
