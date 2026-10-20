# STEP-0280: M22 W1 C/D — E7002 结构检查与零缩进差分

> - status: implementation and local W1 canaries complete; W1/M22 exit still NO-GO pending independent CI adjudication
> - phase: M22 compiler self-host, W1 consolidation (STEP-0277 C/D)
> - date: 2026-09-24
> - evidence class: internal-fixture, Windows x64 GNU real runner

## 1. Scope and decision

完成 STEP-0277 的 W1 C/D。C 将 `compiler_semantics.sico` 中依赖 `loaded`/`model` 字面名字的 E7002 指纹替换为记录字段类型、函数形参和 `payload.text` 访问的结构检查；`payload.revision == ...` 的保护范围按显式 `if`/`end if` 块深度计算。D 先用零缩进语料对照 Rust oracle，发现零缩进 `return` 在 Rust 报 E7002、自举 guest 却放行；随后让语义扫描依据函数起止标记识别正文，不再用原始行是否等于 trim 后行决定是否处理正文。`compiler_parser.sico` 的两个缩进代理对这批零缩进语料没有造成差分，本片未改该文件。

增量用例放在 `selfhost/corpus-w1/`，共 5 个：改名后未保护 E7002、改名后已保护接受、保护块结束后 E7002、零缩进未保护 E7002、零缩进有效函数正文接受。测试冻结每个源的 canonical SHA256，先调用 Rust CLI `check --json` 核对诊断，再让真实 runner 执行自举 checker 并比较结果。原 215 项 `corpus-v0.json` 是历史冻结基线；增量用例不改变它及其历史 215 项校验器的分母。

C/D 在同一 STEP 交付：两者共用 `compiler_semantics.sico`，且 D 的反例直接命中 C 的新检查。此合并偏离 STEP-0277 的逐片 CI 顺序，故本地通过不替代 CI 裁决。

## 2. Executable evidence

- `sico check selfhost/checker.sico`：通过；新 checker 的 Script Component 构建并由 Windows GNU runner 实际加载。开发中曾遇到初版守卫实现生成无效 Wasm（runner exit 127）；改为显式块深度后，最终产物实际执行并通过新用例。
- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test selfhost_checker`：9/9，通过；包含原 215 项差分（116 lexical / 34 semantic identity / 65 accepted / 0 unsupported）与 W1 5 项 Rust/guest 对照。
- `validate-step-0221.ps1`：21/21，通过，真实 runner 的 compiler frontend canary 未回退。
- `validate-step-0261.ps1` 首跑：compiler 21/21、parser 2/2、local-bounds 1/1 通过；formatter 前缀冻结 IR 比对失败。逐字段比较确认 17 个函数 IR 完全相同，唯一差分是 STEP-0276 引入的 `records` 表（`ScriptError` / `ScriptInput` / `ScriptOutput`）尚未写入旧快照；已精确重钉。复跑后历史 `call_left` 拒绝断言因真实 runner exit 0 再次失败：它已由 STEP-0262 实现。校验器据已完成的后续 STEP 更新为检查 `call_left` 成功及当前 typed 前沿；最终完整复跑 `STEP_0261_OK`。
- `validate-step-0262.ps1` 的 22 函数快照同样只缺 STEP-0276 `records` 表；精确重钉该表并把前沿从过时的 `UNRESOLVED` 改为实测 `GWPACK-OTHER` 后，完整复跑 `STEP_0262_OK`。`report-m22-canary.ps1` 独立实测 22/30（73.3%），`nearest_match` / `ERR:E-SH-IR-GWPACK-OTHER`，整份 formatter exit 122；本 STEP 不主张覆盖率增长。
- runner `cargo fmt --check` 初跑发现三处既有 M22 runner 测试格式差分（`record_types.rs`、`selfhost_compiler.rs`、`selfhost_local_bounds.rs`）及本片测试一处格式差分。四处均作纯格式修复；root/runner `cargo fmt --all -- --check` 复跑通过，不改变测试断言或能力合同。
- `validate-step-0245.ps1` 初跑在 bundle 盘点处发现 RFC-0047 后增的 12 个 record fixture 不在历史 215 项内；改为逐项列明冻结后语料后，该单测通过。复跑又发现 `record_types.rs` 将 `RunOutcome` 误作 `Output` 直接取 stdout，修正 variant 解构后 record suite 12/12、runner all-target clippy 通过。最终 `validate-step-0245.ps1` 完整通过（Rust semantic 6+11、bootstrap bundle 4、checker 9、compiler 21、clippy）。
- runner `cargo clippy --locked --offline --test selfhost_checker -- -D warnings`、`validate-step-0221.ps1`（21/21）、`validate-step-0124.ps1` 与 `git diff --check` 均通过。

## 3. Gate accounting

W1 C/D 的新增差分在本机 runner 通过；215 项旧分区本机已重跑，STEP-0221/0245/0261/0262 及当前 canary 报告均绿。STEP-0278/0280 的独立 CI 裁决仍未取得，因此 W1 不宣告关闭。M22 仍 NO-GO，S3/S4/S5 和自举闭环没有因本 STEP 提升；R3 四步计数从 W2 S3/S4 实现开始，本 STEP 不计。下一项有界工作是取得 W1 CI 结果，再重钉 W2 records-surface canary 与拒绝前沿。
