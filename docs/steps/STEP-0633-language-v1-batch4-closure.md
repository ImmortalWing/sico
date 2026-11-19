# STEP-0633 — language v1 batch 4 收口：全量验证 + 文档同步

状态：closing locally（2026-11-19）；未 commit/push；不翻任何 gate。
计划：[`docs/plans/language-v1-batch4-option-variant-match.md`](../plans/language-v1-batch4-option-variant-match.md)。

## 1. 验证矩阵（判据 (5)）

| 命令 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | **绿** |
| `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` | **exit 0** |
| `cargo test --locked --offline --workspace --no-fail-fast` | 见 §2 |
| `cargo test --locked --offline`（runner/sico-runner 工作区） | 见 §2 |

## 2. 全量测试与先存红清单

**根工作区 `cargo test --locked --offline --workspace --no-fail-fast`**：**467 passed / 6 failed（exit 101）**。6 个失败测试全部属 IND-003 族（同根因：`syntax-candidates/b/indentation/invalid/zero-indent-recovery-declaration.sico` 的 parser 恢复路径）：

| 失败测试 | 所在 |
|---|---|
| `all_thirty_three_invalid_cases_stop_before_ir_support_dispatch` | sico-ir |
| `check_matches_all_58_semantic_oracles` | sico-cli |
| `remaining_b_groups_do_not_receive_unowned_diagnostics` | sico-semantics |
| `all_b_cases_preserve_ast_shape_and_are_idempotent` | sico-format |
| `all_b_canonical_cases_parse_losslessly_without_semantic_rejection` | sico-hir |
| `all_b_sources_lower_deterministically_with_complete_source_maps` | sico-parser/syntax 面 |

**先存证据**：六个均在我方改动 stash 后复跑仍旧红（sico-ir/sico-cli/sico-semantics/sico-format 逐面证实）；STATUS 2026-11-07 条已登记 IND-003 使多个校验器止步。本批触碰的全部 crate（sico-ir 24/25、sico-semantics 5/6、sico-codegen-wasm 全绿、sico-cli 12/13、runner `ui_source` 6/6）绿，唯一红即上表先存项。

**runner/sico-runner 工作区 `cargo test --locked --offline --no-fail-fast`**（前置：`cargo build --bin sico-runner` 产出运行器二进制 + `SICO_RUNNER` 指向它的 **Windows 路径**）：**467 passed / 6 failed（exit 101）**——6 个失败与根工作区**完全同一组 IND-003 族**（`all_thirty_three_invalid_cases…` / `check_matches_all_58_semantic_oracles` / `remaining_b_groups…` / `all_b_canonical…` / `all_b_cases_preserve…` / `all_b_sources_lower…`），本批触碰面 `ui_source` 6/6 绿、全部环境依赖测试（checked_result/csv/stats/modules/sico_test/tetris 链）在运行器二进制就位后转绿。**本批触碰面零失败**。

**阻塞解除链（2026-11-19 实测）**：owner 关闭天擎主服务（`QAXClientSvc` Stopped）后构建速度即刻恢复；随之浮出第二层——D 盘 0 字节（此前崩溃/强杀构建在 runner target 攒 63 GB），`cargo clean` 释放；第三层——`cargo test` 不隐式构建 `src/main.rs` 运行器二进制（需显式 `cargo build --bin sico-runner`）且测试需 `SICO_RUNNER`（Windows 路径）。三层解除后全量通过。

**其它先存红（登记不修）**：
- `validate-step-0489.ps1`（诊断协议对拍）的计数门：map 含 5 条 **syntax-candidates** 路径条目（M22 时代登记），而校验器的 source glob 只覆盖 semantic-cases + implementation-cases ⇒ HEAD 即红（46↔41）；本批新增的 EXP-005（E2028）本身经 `sico check --json` 逐字段实测一致（code/key/arguments/message 对拍 catalog 模板）。
- 过程记录：runner 工作区首轮测试撞 rustc `STATUS_STACK_BUFFER_OVERRUN`（STEP-0627 登记过的 EDR 形态），降并发重试时与本地的 stash 归因复跑并发导致二次编译错混（本次归因发现），最终干净重试以本文件记录为准。

## 3. 判据 (6) 同步清单

- machine support matrix：`tests/language-matrix/application-profile-v0.json` 增 executable 行 `enum-payload-match-option-v0` + refused 行 `user-enum-option-function-boundary`；`tools/validate-step-0131.ps1` **绿**。
- backend-refusal corpus：`diagnostics/implementation-cases/expressions/invalid/host-enum-shape.sico`（EXP-005，E2028）+ manifest/case-map/README 同步；EXP-005 实测 E2028 身份一致。
- `docs/STATUS.md` 顶条（2026-11-19 batch 4 汇总）+ `docs/ROADMAP.md` M23 节更新（独立 batch 轨、不改 M23 裁决）。
- `docs/steps/README.md` 登记 STEP-0629..0632（本片登记 0633）。

## 4. 判据 (7) selfhost 边界

`git diff --stat -- selfhost/parser.sico selfhost/codegen.sico` 的两文件改动**全部早于本批**（会话首个 git status 即已 dirty，STEP-0627/0628 并发工作）；本批全程零写入。派生工件 `emitter-vocabulary-v1.json`（`source_sha256` 钉 codegen-wasm 源哈希）与 `match-component-coverage-v1.json` 因本批对 `sico-codegen-wasm` 的合法改动而被 census 测试自动刷新——如实保留并在此登记。

## 5. 交付物总览（batch 4）

STEP-0629（探针）→ 0630（G1）→ 0631（G2）→ 0632（G3-R1）→ 0633（本片）；
RFC-0056（**accepted**，owner 指令 2026-11-19「RFC-0056 接受」——D5 宿主面 enum 约定随接受冻结为合同）；冻结探针 13/13；e2e `enum-payload-match` PASS；runner `ui_source` 6/6。
