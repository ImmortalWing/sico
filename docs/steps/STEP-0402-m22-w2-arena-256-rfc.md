# STEP-0402 — M22: RFC-0029 预算表正式修订——guest 线性内存 128→256 MiB，0365 trap 类瓦解，census 107→110/130、0 DIFF

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，RFC 预算表修订片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**RFC-0029 预算表修订**（0351 模式同步面全量）：
- `crates/sico-codegen-wasm/src/canonical.rs`：`ARENA_LIMIT` 128→256 MiB、
  `MEMORY_PAGES` 2048→4096；
- `crates/sico-runtime/src/canonical.rs`：`ARENA_LIMIT` 128→256 MiB；
- `crates/sico-runtime/src/canonical/tests.rs`：Arena 构造页数 2048→4096（×5）；
- `crates/sico-runtime/src/lib.rs`：`HostLimits` 默认 memory 128→256 MiB
  （宿主授权同步）；
- `runner/sico-runner/src/lib.rs`：`RunnerLimits` 默认 memory 128→512 MiB
  （runner 配套，覆盖 256 arena + 余量）；
- `docs/rfc/RFC-0029-script-profile-v0.md`：STEP-0402 amendment 段
  （含 STEP-0401 根因链登记：15 户 trap = arena 耗尽非值损坏，0365
  诊断更正）。

**依据**：STEP-0401 内存矩阵实验（256MiB arena 下 gw_compare trap→
STATEMENT typed 拒绝）+ 函数索引映射（trap 在 canonical 胶水层）。

## 2. 前沿更新

- census **107/130 → 110/130、0 DIFF**；**NON-DOMAIN 17→5**（0365 trap
  类瓦解，12 户走出至 typed 门）；**checked_arithmetic_set_match_ir
  （0365 登记户本体）/ gw_compare_packed / chain_condition_packed 三户
  翻绿**；CALL-SHAPE 3→12（级联推进）；无回归。
- 下一刀主线：CALL-SHAPE 12 户（机器族操作数形续清）+ GWPACK-OTHER 3 户
  + 残余 NON-DOMAIN 5 户（疑似 arena 更深需求或真缺陷，逐户重定）。

## 3. 事故与登记

- 0351 模式复用要点全数照办：RFC amendment 段、宿主授权、runner 配套、
  测试页数同步；canonical ABI 使用常量自动跟随，无需手工改 glue。

## 4. 验证与预算

- census 终值：flipped=110 diffs=0 total=130；HIST CALL-SHAPE 12 /
  GWPACK-OTHER 3 / NON-DOMAIN 5；探针副本按纪律删除。
- 固件：无新测试（本片为零测试面常量修订，81 项既有固件原位保持 =
  行为不回归的见证；census 15 户 typed 门可见为正向证据）。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （81/81 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
