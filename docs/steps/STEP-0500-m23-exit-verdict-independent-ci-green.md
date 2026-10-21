# STEP-0500 m23 出口裁决：独立 CI 绿 → M23 整体 GO

> - status: complete（M23 出口审计裁决 = **GO**；七门全部有证据，2026-09-30 审计的三项残余全部闭合）
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0498](./STEP-0498-m23-item3-corpus-freeze-and-d3-record.md)（item 3 闭合与 D3 决定记录）、[STEP-0489](./STEP-0489-m23-diagnostics-catalog-full-extension.md)（诊断协议全量扩展）、[M23 计划 §3 / §8](../plans/M23-language-v1-batch-3.md)、[RFC-0046](../rfc/RFC-0046-language-v1-batch2-for-loops-error-propagation-list-elements-v0.md)（D3）、[2026-10-21 出口审计](../reports/m23-exit-audit-2026-10-21.md)、[CI 归因报告](../reports/ci-arena-ceiling-attribution-2026-10-21.md)
> - gate: M23 计划 §3 七门（逐门证据见出口审计 §2 与 §6）+ 2026-09-30 出口审计 §4 的三项残余；**本片闭合残余 #2（独立 CI 裁决）**

## 1. 定位

STEP-0498 闭合了 item 3 并出具了 2026-10-21 出口审计，但**裁决留空**：出口审计
的门 6 要求"独立 CI 裁决"这一腿有绿证据。此前 GitHub 恢复可达后跑出的两轮
（`39454624507` / `39457030463`）都红在同一点，根因经本会话确定性归因为 **M22
轨**的常量错配（codegen 声明客人内存下界 `MEMORY_PAGES` = 33,792 页 vs 应用层
宿主上限 2 GiB = 32,768 页），M22 轨已用 STEP-0497 修复。本片取修复后的独立
CI 结论，补齐出口审计裁决。

## 2. 独立 CI 证据

| run | SHA | 结论 | 内层步骤 |
|---|---|---|---|
| `39524849306` | `7e5ad3c`（M22 修复提交） | **success** | 11/11 PASS |
| `39525367456` | `4e14b9d`（最终树：含 M23 全部提交 + 冻结语料 + 审计文档） | **success** | 11/11 PASS |

`39525367456` 的 `windows-gnu` job 内层逐步结果（`tools/run-ci.ps1`）：

```
PASS: fmt (00:00:01)              PASS: clippy (workspace) (00:00:36)
PASS: clippy (runner) (00:02:19)   PASS: build (runner debug) (00:05:54)
PASS: test (workspace) (00:03:36)  PASS: test (runner) (00:08:14)
PASS: module boundaries (00:00:14) PASS: planning contract (M14-M25) (00:00:00)
PASS: application-profile matrix (00:00:26)
PASS: cross-host matrix + UI corpus (00:00:00)
PASS: whitespace (git diff --check) (00:00:01)
```

> **步数口径说明（STEP-0499 追加，2026-10-21）**：上面两轮 run 的 **11/11** 是**历史事实**——那两轮执行时 `tools/run-ci.ps1` 就是 11 个内层步骤，本裁决所依据的证据不变。自 **STEP-0499** 起，`tools/run-ci.ps1` 增加了第 12 步 **`fmt (runner)`**（`runner/sico-runner` 是独立 Cargo 工作区，不在根 `Cargo.toml` 的 `members` 里，根 `cargo fmt --all` 不覆盖它；此前只有不在 CI 步骤内的 `tools/validate-step-0245.ps1` 检查它 ⇒ runner 工作区的 rustfmt 违规能同时通过本机一键 CI 与独立 CI）。因此**自本片起，独立 CI 的内层步数为 12**；引用本裁决时，历史 run 记 11/11、后续 run 记 12/12，两者都指各自当时的真实形状。

runner 存在 `windows-2025` + MSYS2 MINGW64 的 GNU 工具链，180 分钟上限；
`SICO_TEST_WASMTIME` 由 `tools/run-ci.ps1` 注入，故 `sico-app-cli` 的 guest
启动缝**在本轮被真实执行**（此前红点所在处）。

## 3. 残余闭合

| # | 残余（2026-09-30 审计 §4） | 状态 | 闭合方式 |
|---|---|---|---|
| 1 | item 3 的 owner-review AI 语料 | **闭合** | STEP-0498：100 次计费调用（USD 0.027196）+ owner 复审 + 语料冻结入库 + RFC-0046 §3 D3 决定记录（"D3 stands"，不改语法） |
| 2 | STEP-0328/0329 独立 CI 裁决 | **闭合** | 本片：修复后两轮独立 CI 全绿（11/11 内层步骤） |
| 3 | `diagnostics/catalog.json` / semantic-case-map 全量协议扩展 | **闭合** | STEP-0489：8 码登记 + 编译器同形对齐 + 实现态语料 + 真实编译器逐字节对拍 |

## 4. 裁决

**M23（language v1 batch 3）整体 GO。** 逐门证据见
[`m23-exit-audit-2026-10-21.md`](../reports/m23-exit-audit-2026-10-21.md) §2 与
§6。裁决覆盖 M23 自身表面：RFC-0048 中缀比较与 `&&`、RFC-0049 let 绑定
`if`/`match`、item 3 裸字面量政策（D3 stands，无语法改动）、item 4 `text.chars`
收口、诊断协议全量扩展。

**裁决不覆盖**：M22 的任何门（M22 维持自己的 NO-GO 裁决）；平台广度（CI 证据
只覆盖 Windows x64 GNU）；AI 噪音主张（本里程碑无此主张）。item 3 的 AI 腿仍是
**候选语法 A0 下的文本级测量**（A0 是 M1 保留对照语法、无可执行编译器），与
selfhost 腿**不可比**，且实为 11 份不同程序而非 100 个独立样本——这些都写在
RFC-0046 §3 的 D3 记录里。被收窄的待决问题（能否给 `I64`/`U64` 一个免显式
构造子的上下文推断宽度字面量）记录为**本片不作决定**；任何未来的 typed-literal
RFC 仍须带完整 RFC-0033 证据与其自身实测消费者。

## 5. 本机复核（同树）

```
cargo fmt --all -- --check                                                       → exit 0
cargo clippy --locked --offline --workspace --all-targets --all-features -D warnings → exit 0
cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml -D warnings → exit 0
cargo test --locked --offline --workspace --all-targets --all-features            → exit 0（无失败套件）
runner: cargo test --manifest-path runner/sico-runner/Cargo.toml --test language_batch3 → 14/14
validate-step-0328 / 0329 / 0489 / 0131 / 0156 / module-boundaries /
  semantic-cases / diagnostics / step-0124（规划合同）                               → 全部 exit 0
```

冻结语料零改动（`semantic-cases` / `syntax-candidates` A·B·C / `syntax-mutations` /
`selfhost/corpus-v0` / `corpus-w1`）；本会话对产品代码的改动为零。

## 6. 证据

- 出口审计：[`m23-exit-audit-2026-10-21.md`](../reports/m23-exit-audit-2026-10-21.md)
- item 3 证据：[`ai-eval-item3-live-model-2026-10-21.md`](../reports/ai-eval-item3-live-model-2026-10-21.md)
- 冻结语料：[`ai-eval/runs/item3-2026-10-21/`](../../ai-eval/runs/item3-2026-10-21/)
- CI 红点归因：[`ci-arena-ceiling-attribution-2026-10-21.md`](../reports/ci-arena-ceiling-attribution-2026-10-21.md)
- D3 决定记录：[RFC-0046 §3](../rfc/RFC-0046-language-v1-batch2-for-loops-error-propagation-list-elements-v0.md)
