# STEP-0487 m22-w2 S5 codegen 骨架（Component 逐字节发射）

> - status: implementation complete locally（未提交快照；本片补齐 STEP-0486 波次合同的首刀记录）
> - date: 2026-10-20
> - owners: autonomous-agent
> - depends: [STEP-0486](./STEP-0486-m22-w2-s5-codegen-wave.md)（S5 波次合同）、RFC-0011（byte oracle）、ADR-0015/0016、M22 计划 §4 门 3
> - gate: 门 3 的第一刀 = 自举编译器开始真实产出 Component 字节（此前只产 IR JSON）

## 1. 定位

STEP-0486 冻结了 S5 波次合同：门 3 的实质欠账 = 自举编译器只产 IR JSON、不产 Component 字节。本片是合同顺序里的**骨架刀**：先把固定的组件骨架真实发射出来，并让 guest 产出与 Rust backend **逐字节相同**的 Component，作为后续 IR 驱动段落的字节对拍基座。

## 2. 交付物

| 项 | 位置 | 说明 |
|---|---|---|
| guest 发射模块 | `selfhost/codegen.sico`（新文件，11,176 B） | 固定段具名 builder + 段尺寸真 LEB128 实算 + run 体槽位 |
| 驱动模式 | `selfhost/compiler.sico` | `--emit-component-hex`：输入 = 源字节；输出 = Component 的 hex |
| 固件钉 | `runner/sico-runner/tests/selfhost_compiler.rs::sico_codegen_guest_emits_component_bytes_byte_exactly` | guest 输出 vs `sico_codegen_wasm::compile_script_program`（Rust 唯一 byte oracle），逐字节全等 + `wasmparser` 校验 |

## 3. 字节布局（文档化，11 个 component 段 + 核心模块段）

Component（981 B）= `0061736d0d000100` + 段 7（script types 实例类型）+ 段 10（types-only import）+ 段 6（alias 类型）+ 段 7（run 函数类型）+ 段 1（core module，530 B）+ 段 2（core instance）+ 段 6（alias core：memory / cabi_realloc / cabi_post_run / run）+ 段 8（canonical lift）+ 段 11（export run）。

Core module（530 B）= `0061736d01000000` + 段 1（type ×4：run / alloc / realloc / post_run）+ 段 3（function ×4）+ 段 5（memory）+ 段 6（global ×3：arena bump 指针 / perm 区基址 / scratch 模式旗）+ 段 7（export ×4）+ 段 10（code：run 223 B + alloc 84 B + realloc 72 B + post 32 B）。

## 4. 诚实性声明

- 骨架只覆盖**一个冻结源形**（最小 echo fixture：ScriptInput / ScriptOutput / ScriptErrorCode / ScriptError + `run` 回显 stdin）。其它任何源形一律返回空串，驱动 fail-closed 为 typed 拒绝 `unsupported codegen source shape`（ADR-0016 决策 6：无静默回退）。
- run 体目前是**冻结槽位**，不是 IR 驱动 lowering；段尺寸（LEB128）是真算的，固定段按段具名，便于后续逐段替换。
- 无 Rust compile hostcall：全部字节在 guest 内产出，Rust 只作 oracle。

## 5. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 固件钉 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` | **82 passed / 0 failed**（含本片新钉；128.26 s） |
| census 不回归 | `target/tmp/run_census.sh`（census-0501） | 149/149 翻绿、0 DIFF、HIST 空（0484 基线未动） |
| 电池 | `tools/validate-step-0262.ps1` | `STEP_0262_OK`，exit 0 |
| M22 checker/驱动语料 | `tools/validate-step-0245.ps1` | exit 0 |
| formatter canary | `tools/report-m22-canary.ps1` | 30/30 覆盖、`full_source_exit: 0`、FORMATTER-CLOSED |
| 规划合同 | `tools/validate-step-0124.ps1 -SelfTest` | `STEP_0124_OK` |
| 空白纪律 | `git diff --check` | 净 |

## 6. 后续

STEP-0486 波次顺序的下一刀 = **IR 驱动 run 体**（消费 guest 已证的 canonical IR JSON，取代冻结槽位）。该刀的前置依赖与已定位的阻塞见 [STEP-0488](./STEP-0488-m22-w2-s5-frontend-record-variant-gap.md)。
