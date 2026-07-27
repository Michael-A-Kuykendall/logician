<p align="center">
  <img src="https://raw.githubusercontent.com/Michael-A-Kuykendall/logician/master/assets/logician-logo.png" alt="Logician Logo" width="300">
</p>

<h1 align="center">Logician</h1>

<p align="center">
  <strong>面向 Rust 的排序檢查 SMT 求解器驅動</strong>
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
  <a href="#為什麼選擇-logician">為什麼選擇 Logician</a> •
  <a href="#快速開始">快速開始</a> •
  <a href="#功能特性">功能特性</a> •
  <a href="#設計理念">設計理念</a> •
  <a href="#贊助商">贊助商</a>
</p>

---

## 為什麼選擇 Logician？

SMT 求解器功能強大，但將其整合到 Rust 專案中不應需要博士學位。

| 方案 | 配置 | 型別安全 | 多求解器 | 看門狗 |
|------|------|----------|----------|--------|
| **FFI 綁定** | C++ 工具鏈，平臺依賴 | 是 | 手動 | 手動 |
| **字串建構** | 簡單 | 無——祈禱字串能解析 | 手動 | 手動 |
| **Logician** | `cargo add logician` | **執行時：排序不匹配時 panic** | **內建** | **內建** |

**核心特性：**

- **流暢的 Term API** — 用 Rust 建構公式，而非字串。排序不匹配立即 panic，附帶可操作的診斷資訊。
- **多求解器自動切換** — Z3 超時？Logician 自動切換到 CVC5 重試。
- **程序看門狗** — 求解器掛死？立即終止，整個程序樹被乾淨清理。
- **可選非同步支援** — 需要時啟用 `tokio` 特性。

```rust
// 立即 panic："and 要求另一個參數為 Bool 排序"
let bad = bool_var.and(int_var);

// 正常工作，序列化為有效的 SMT-LIB
let good = x.and(y.or(z));
```

無靜默失敗。無格式錯誤的查詢到達求解器。無孤兒程序。

---

## 快速開始

```toml
[dependencies]
logician = "0.1"
```

你需要在 PATH 中安裝 SMT 求解器（例如 [Z3](https://github.com/Z3Prover/z3)）。

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
        Response::Sat => println!("可滿足！"),
        Response::Unsat => println!("不可滿足！"),
        Response::Unknown => println!("未知"),
        _ => {}
    }

    Ok(())
}
```

---

## 功能特性

### 排序檢查的 Term（執行時，非編譯時）

```rust
let a = Term::Var("a".into(), Sort::Bool);
let c = Term::Var("c".into(), Sort::Int);

// 正常工作
let f1 = a.and(b);
let f2 = c.eq(Term::Int(42));

// Panic："and 要求另一個參數為 Bool 排序"
let bad = a.and(c);
```

### 多求解器自動切換

```rust
use logician::multisolver::MultiSolver;

let mut ms = MultiSolver::new(vec![z3_config, cvc5_config]);
ms.declare("x", &Sort::Bool);
ms.assert(&Term::Var("x".into(), Sort::Bool));
match ms.check() {
    Ok(Response::Sat) => println!("找到解"),
    Err(e) => println!("所有求解器均失敗：{:?}", e),
}
```

### 程序看門狗

```rust
let config = Config {
    timeout: Duration::from_secs(5),
    // ...
};
```

通過 PID 安全的輪詢和 kill 終止求解器程序樹。

### SMT-LIB 跟蹤

```rust
let config = Config {
    trace: true,
    // ...
};
```

### 非同步支援

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

## 設計理念

### 執行時不變量（非編譯時型別安全）

Logician 通過執行時不變量（`assert_invariant!`）而非型別層級機制來強制排序正確性：

1. **記錄**每個排序檢查的唯一標籤（用於審計）
2. **違反時 panic**（無靜默損壞）
3. **審計覆蓋率** — `tests/mod.rs::c_invariant_audit` 枚舉每個標籤，
   如果某個不變量停止被測試，建構失敗

這是執行時檢查：排序不匹配在建構 term 時 panic，而非編譯時。權衡是簡單性優先於編譯時保證。

### Logician 是什麼

- SMT 求解器的子程序驅動（stdin/stdout）
- 帶排序檢查的 term 建構
- 多求解器編排與自動切換
- 帶超時處理的程序生命週期管理

### Logician 不是什麼

- 非 FFI 綁定（無需 C++ 編譯）
- 非求解器本身（你需要 Z3、CVC5、Yices 等）
- 非定理證明框架
- 不追求高級 SMT-LIB 特性（陣列、位元向量、量詞不在範圍內）

---

## 支援的求解器

| 求解器 | 配置 |
|--------|------|
| [Z3](https://github.com/Z3Prover/z3) | `program: "z3", args: ["-in"]` |
| [CVC5](https://cvc5.github.io/) | `program: "cvc5", args: ["--lang", "smt2"]` |
| [Yices 2](https://yices.csl.sri.com/) | `program: "yices-smt2"` |

任何符合 SMT-LIB 2 標準的求解器均可使用。

---

## 測試

```bash
# 單執行緒（全域不變量狀態需要）
cargo test -- --test-threads=1

# 含覆蓋率
cargo tarpaulin --out Html
```

**當前：26 個測試（預設）/ 18 個測試（tokio）。**

---

## 贊助商

**如果 Logician 為你節省了時間，請考慮贊助開發。**

| 等級 | 權益 |
|------|------|
| **$5/月** 咖啡英雄 | 衷心感謝 + 贊助徽章 |
| **$25/月** 開發者支援 | 優先支援 + SPONSORS.md 中署名 |
| **$100/月** 企業支援 | README 中顯示 Logo + 月度會議 |
| **$500/月** 企業合作夥伴 | 直接支援 + 功能建議權 |

**公司用戶**：需要發票？請發郵件至 michaelallenkuykendall@gmail.com

---

## 貢獻

Logician 是**開源但非開放貢獻**的模式。詳見 [CONTRIBUTING.md](CONTRIBUTING.md)。

歡迎通過 GitHub Issues 提交錯誤報告。安全問題請參閱 [SECURITY.md](SECURITY.md)。

---

## 許可證

MIT — 詳見 [LICENSE](LICENSE)。

---

<p align="center">
  用 🦀 建構，作者 <a href="https://github.com/Michael-A-Kuykendall">Michael A. Kuykendall</a>
</p>
