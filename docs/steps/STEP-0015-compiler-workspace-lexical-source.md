# STEP-0015: 建立编译器 workspace 并冻结 lexical/source 契约

> - status: complete
> - phase: M1
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

建立首个正式、可构建、依赖锁定的 Sico Rust compiler workspace；在任何 lexer 实现之前，用 RFC 固定 UTF-8 输入、Unicode identifier、newline、trivia、token、span 与输入限额，并为每类规则提供正反例和明确的 accepted/proposed 边界。

## 2. Context and evidence

- [`M1 compiler frontend plan`](../plans/M1-compiler-frontend.md) 要求本步骤先于 source/lexer 实现；
- [`RFC-0005`](../rfc/RFC-0005-labeled-block-syntax-baseline.md) 固定 B labeled-block surface，但明确未定义 Unicode、identifier、文件与 trivia；
- [`RFC-0001`](../rfc/RFC-0001-diagnostics-protocol-v0.md) 已固定 UTF-8 byte 半开 range 和派生的 1-based Unicode scalar line/column；
- 当前 208 个 `.sico` fixture 均为无 BOM UTF-8，实测使用 LF；B corpus 使用 `//` 行注释、ASCII 标点、整数和简单字符串，但没有 Unicode identifier 样本；
- 本机活动工具链为 stable GNU，实际版本在完成验证时记录。

## 3. Scope

包含：workspace 根 manifest、锁文件、七个 M1 前端边界 crate、Rust 工具链/仓库忽略规则、依赖与 parser/tree/CLI 的有界选择、lexical/source RFC、独立正反例与 STEP 验证器、状态与索引更新。

不包含：真实 source/span API、lexer、parser、syntax kind、AST、formatter、诊断 code、CLI 命令或任何 M2 类型/效果/能力/所有权判断。空 crate 只建立依赖边界，不声明功能完成。

## 4. Options and decision

- lossless tree：选择精确锁定的 `rowan 0.16.1`，拒绝在没有收益证据时自建 green tree；
- parser：选择 STEP-0017 实现手写 recursive-descent + event stream，以直接控制 B 具名关闭和 recovery anchor；生成式 parser 不进入 workspace；
- CLI：选择精确锁定、裁剪默认 feature 的 `clap 4.6.1`，但 STEP-0020 前不建立 binary 或退出码；
- identifier：拒绝 ASCII-only 和静默 normalization；接受 Unicode 17.0 XID + 必须已是 NFC，中文名称合法；UAX #39 confusable/script lint 保持 proposed；
- source：接受无 BOM 严格 UTF-8、LF/CRLF、ASCII space/TAB、`//`、整数与受限双引号字符串；block/doc comment、decimal/exponent、raw/multiline string 保持 proposed/not accepted；
- span/limit：接受统一 `SourceId + u32 UTF-8 byte range`、派生 Unicode scalar line/column，以及 16 MiB source、1 MiB line/token、1024-byte identifier、1,000,000 token、100 lexical diagnostic 硬上限。

选择只固定工程依赖和词法边界，没有新增 M2 语义或稳定 E1xxx code。撤销条件和兼容规则见 RFC-0006。

## 5. Plan

1. 创建本 STEP 并把 STATUS 标记为 M1/STEP-0015 in-progress；
2. 盘点 corpus 字节、换行、字符、注释、literal 与标点事实；
3. 编写 RFC-0006、正反例清单与可执行一致性验证；
4. 建立 workspace/crate 边界，锁定 Rust 与第三方依赖；
5. 运行 format、Clippy、tests、workspace metadata、STEP 验证器、M0 回归、链接和 diff 检查；
6. 逐项审查 exit criteria，更新 STATUS/ROADMAP/索引/README，提交并推送。

## 6. Changes

- 根目录新增 Cargo workspace、`Cargo.lock`、stable toolchain policy、target ignore 和 LF attributes；
- 建立 `sico-source/syntax/lexer/parser/format/diagnostics/cli` 七个 crate，只有文档与依赖边界，无行为 API；
- 接受 [`RFC-0006`](../rfc/RFC-0006-lexical-source-contract-v0.md)，新增 7 positive + 14 negative machine-readable contract case；
- 新增 `validate-step-0015.ps1`，检查 case/RFC/limit/span/metadata/lock/corpus 一致性；
- 修复 M0 validator 对未来 STEP 数量和当前 next pointer 的错误耦合，使其只审计 STEP-0001–0014；
- 更新 README、DEVELOPMENT、STATUS、ROADMAP 与审计索引，新增 [`review report`](../reports/compiler-workspace-lexical-source-v0.md)。

## 7. Validation

实际命令：

```text
cargo fmt --all --check
cargo clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
cargo test --offline --locked --workspace --all-targets --all-features
cargo metadata --offline --locked --format-version 1
powershell -File tools/validate-step-0015.ps1
```

环境：rustc 1.97.0、cargo 1.97.0、rustfmt 1.9.0-stable、clippy 0.1.97。结果：format pass；Clippy pass；七个空 crate test target pass（0 behavioral tests）；locked metadata pass；STEP validator pass；M0 全离线验证与 Markdown links pass；`git diff --check` pass。

确定摘要：

```text
STEP_0015_OK crates=7 registry_dependencies=14 cases=21 corpus=208 lf=3979 crlf=0 bare_cr=0 bom=0 unicode=17.0.0
```

## 8. Metrics

- 7 workspace crates；14 个 registry package（含传递依赖）；
- 21 contract cases：7 positive、14 negative，正反两侧均覆盖 utf8/identifier/newline/trivia/token/span/limit；
- 208 个 `.sico`、3979 个 LF、0 个 CRLF、0 个 bare CR、0 个 BOM/invalid UTF-8；
- 18 个仓库 Rust 文件、13 个 Cargo manifest；
- 0 个 lexer/parser behavior test，准确反映尚未实现。

## 9. Risks and follow-ups

- Unicode 数据版本升级可能改变 identifier 集合，必须通过依赖锁与 RFC 兼容规则控制；
- hard limit 只有保守设计值，STEP-0021 必须用 fuzz/performance 复核；
- confusable/script lint 和若干 lexical form 未接受，后续不能由 lexer 临时扩展；
- crate/JSON validator 不是行为实现；STEP-0016 必须让 21 个 case 驱动真实 source/span/lexer tests。

## 10. Audit links

- RFC：[`RFC-0006`](../rfc/RFC-0006-lexical-source-contract-v0.md)；
- report：[`compiler workspace and lexical/source v0 review`](../reports/compiler-workspace-lexical-source-v0.md)；
- commit topic：`feat(frontend): [STEP-0015] establish workspace and lexical contract`；
- next：STEP-0016 source/span + lossless lexer。
