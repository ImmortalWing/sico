# STEP-0322: M22 S5 扩面——checked_sub codegen 族

> - status: implementation complete locally / independent CI pending（GitHub 本机不可达，快照推送 gitcode `origin`）
> - phase: M22 22-D S5（`--emit-core-hex` 缝第八族；Flash 普查点名的最小扩面候选）
> - completed: 2026-09-30
> - evidence: internal-fixture，Windows x64 真实 runner（`1.98.0-x86_64-pc-windows-gnu`）

## Objective / Changes

- `selfhost/compiler.sico` 新增 `checked_sub_core_hex`（与 `checked_add_core_hex`
  同构的 17 词整函数族）：I64/U64 双模板（体 89/76 字节，核心 opcode `0x7d`
  与 sub 溢出检查操作数序差异均按 Rust codegen oracle 实测 hex 钉死），
  canonical 源校验 `.checked_sub(a, b)`，接入 `declared_core_hex` 链尾。
- 测试：`sico_compiler_encodes_declared_scalar_core_wasm_families_byte_exactly`
  字节全等清单 +`checked-sub-i64`/`checked-sub-u64`；typed 拒绝清单的
  `checked-sub` 项由 `checked-mul` 顶替（拒绝覆盖保持 7 项，为下一族预留钉扎）。
- 语料：`freeze-step-0315-corpus.ps1` 扩为 16 字节全等 + 7 拒绝并重生成
  `codegen-corpus-v0.json`（schema 不变，step 字段 `STEP-0315+0322`）；
  `validate-step-0315.ps1` 的条目数检查由 `==14` 改为 `>=14` 下限（历史底线
  单调保持）。

## Measured results（本机真实 runner）

- 2 个新 fixture 对 `sico_codegen_wasm::compile` oracle 字节全等并通过
  wasmparser 校验；`selfhost_compiler` 套件 55/55 绿。
- S5 缝族数 7 → **8**；语料 16 字节全等 + 7 typed 拒绝。

## Honest residuals

- `checked_mul`/`checked_div` 仍 typed 拒绝（`unsupported codegen source shape`
  + 新钉扎 `checked-mul`）；S5 总缺口（37/178 artifact 差分）不变。
- M22 NO-GO；降低前沿不变。
