# Report: Compiler workspace and lexical/source v0 review

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0015
> - environment: Windows, rustc 1.97.0, cargo 1.97.0, rustfmt 1.9.0-stable, clippy 0.1.97

## 1. Question

在不实现 lexer/parser 或偷渡 M2 语义的前提下，仓库能否建立可构建、依赖锁定的正式 Rust 前端 workspace，并让 UTF-8、Unicode identifier、newline、trivia、token、span 与输入限额具有可执行审查入口？

## 2. Method

1. 扫描全部 208 个 `.sico` 的原始 bytes，严格解码 UTF-8，并统计 BOM/LF/CRLF/bare CR；
2. 扫描 54 个 B canonical case 的 identifier、comment、literal 与 punctuation 表层；
3. 对 identifier、tree、parser、CLI、BOM/newline/comment/literal 候选做有界比较；
4. 把接受规则与未接受提案写入 RFC-0006，把 7 positive/14 negative oracle 写入 JSON；
5. 建立七 crate workspace 和精确 registry dependency，生成并提交 `Cargo.lock`；
6. 用独立 PowerShell validator 交叉检查 RFC、case、span、limits、workspace metadata、锁定版本和 corpus；
7. 运行 format、Clippy、test、M0 离线回归、Markdown links 与 Git whitespace 检查。

初次依赖获取允许访问 crates.io；锁文件生成后，STEP validator、Clippy 和 tests 均使用 `--offline --locked`，避免验证阶段解析出不同依赖。

## 3. Reproduction

```powershell
$env:RUSTUP_TOOLCHAIN = 'stable'
$cargo = "$HOME\.cargo\bin\cargo.exe"

& $cargo fmt --all --check
& $cargo clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
& $cargo test --offline --locked --workspace --all-targets --all-features
& $cargo metadata --offline --locked --format-version 1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-step-0015.ps1 -CargoPath $cargo
powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-m0-exit.ps1
git diff --check
```

## 4. Raw evidence

权威输入是入库的 [`contract-v0.json`](../../tests/lexical/contract-v0.json)、[`Cargo.lock`](../../Cargo.lock)、七个 crate manifest 和现有 `.sico` corpus。命令输出不以生成日志重复入库；validator 的确定摘要用于复现比对。

## 5. Results

| Evidence | Result | Strength |
|---|---:|---|
| workspace crates | 7 | verified by locked Cargo metadata |
| registry packages including transitive | 14 | verified by locked Cargo metadata |
| contract cases | 21 = 7 positive + 14 negative | verified structurally |
| `.sico` files | 208 | measured |
| existing corpus newlines | LF 3979; CRLF 0; bare CR 0 | measured |
| existing corpus BOM/invalid UTF-8 | 0 / 0 | verified |
| Rust format | pass | verified |
| Clippy `-D warnings` | pass | verified |
| workspace tests | 7 empty crate targets pass; 0 behavioral tests | verified, not lexer evidence |
| STEP validator | `STEP_0015_OK` | verified |
| actual source/lexer/parser behavior | not implemented | explicit boundary |

The validator summary is:

```text
STEP_0015_OK crates=7 registry_dependencies=14 cases=21 corpus=208 lf=3979 crlf=0 bare_cr=0 bom=0 unicode=17.0.0
```

## 6. Interpretation

STEP-0015 的工程和规范入口成立：workspace 真实构建，依赖可由锁文件离线解析，lexical/source 每个要求都有正反 oracle 和 accepted/proposed 状态。该证据不证明 tokenization、span implementation、line index 或 diagnostic behavior；这些只有在 STEP-0016 用同一 case 驱动真实代码后才可声明。

`validate-m0-exit.ps1` 原先把“仓库永远只有 14 个 STEP”和“当前 next 永远是 STEP-0015”写死；新增 STEP 后会误报 M0 回归。本步骤把它收窄为只验证 STEP-0001–0014 及 M1 phase，使 M0 审计保持可重跑而不依赖未来进度指针。

## 7. Limitations

- contract JSON 目前由结构 validator 验证，不由 lexer 消费；
- corpus 没有 Unicode identifier、CRLF 或 invalid bytes，相关 case 是规范 oracle，不是历史兼容测量；
- Unicode confusable/script lint、block/doc comments、decimal/exponent、raw/multiline string 均未接受；
- 0 behavioral tests 是空 crate 边界的真实结果，不能作为 M1 lexer/parser 完成数据；
- 16 MiB/1 MiB/1,000,000 等限额尚无性能/fuzz 基线，STEP-0021 必须复审。

## 8. Decision impact

RFC-0006 accepted，解除 STEP-0016 的 lexical/source 设计前置条件。M1 parser 继续只面向 RFC-0005 B；M2 语义、WIT name mapping、正式 E1xxx code 和 CLI exit behavior 均未被本步骤决定。

## 9. Links

- [`STEP-0015`](../steps/STEP-0015-compiler-workspace-lexical-source.md)
- [`RFC-0006`](../rfc/RFC-0006-lexical-source-contract-v0.md)
- [`M1 plan`](../plans/M1-compiler-frontend.md)
- [`contract cases`](../../tests/lexical/contract-v0.json)
