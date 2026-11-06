# STEP-0561 m22-w2 **W4 拒绝语料 verdict parity（前端 150 项）**

| 项 | 值 |
|---|---|
| 片型 | **测量 + 工具片**：新增版本化探针 `tools/probe-m22-corpus-parity.ps1`（D11），对冻结拒绝语料的 **150 个 `rust_check=refused` 前端项**逐一运行 guest 编译器（`sico run selfhost/compiler.sico`，源码 stdin），比对 **verdict parity**（guest 是否同样拒绝）。结果见 §3 |
| exit | **146/150 `match`、4 `mismatch`**（1 `lowered` = 真实声音洞（`missing-field`/`E2010`）+ 3 `failed:` = 燃料墙）；探针 exit 1（`mismatch`>0）。构建期子集验证：12 项 + 2 项全 `match`，JSON 冻结工件格式已修复 |
| 不翻门 | 门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用 —— 全部**维持**（无变化） |
| 证据等级 | `internal-fixture`：本机 Windows x64，debug 测试构建，`sico_runner` 限额不变（fuel 800e9 / timeout 120 s / 内存 2560 MiB） |

> **Addendum（2026-11-06，评审对齐 STEP-0564）**：本片登记的 1 个 `lowered` 声音洞（`missing-field`/`E2010`）**已由 STEP-0558 修复**（缺字段 ⇒ `ERR:E-SH-IR-CONSTRUCT-FIELD`）；STEP-0564 在含该修复的树上全量重测 150 项 = **exact 147 / zero 3（燃料墙）/ lowered 0**，`missing-field.sico` 判为 `refused:ERR:E-SH-IR-CONSTRUCT-FIELD`。本片 146/150 的读数是**修复前时点**口径，保留原文；当前权威读数以 STEP-0564 与工件 `selfhost/corpus-parity-v0.json` 为准。

## 1. 要回答的问题（W4 拒绝语料 verdict parity 的前端半边）

W4（闭合阶梯，STEP-0514 §3.3.13）要求 **178 条拒绝三元组（verdict, id, span）一致**。本片回答其**verdict 半边**的**前端（check）轴**：

> 对 Rust 前端在 check 阶段拒绝的每一条语料（`rust_check=refused`），guest 前端是否**也拒绝**？

verdict parity = guest 不出现「Rust 拒绝、guest 接受」的**越权接受**（soundness 洞）。id/span 精确 parity 是**另一轴**，见 §4。

## 2. 方法（D11 探针）

- **冻结输入**：`selfhost/corpus-v0.json`（schema `sico.m22.corpus.v0`，215 项；**150** 项 `rust_check=refused`，65 项 `accepted`）。冻结 `source_sha256` 逐项钉死源文本。**150 项拒绝的构成（实测核对）**：**116** 项 `LEXICAL` + **34** 项 `E-code`（**29 个唯一码**：`E2001`×5、`E3002`×2、其余 27 个唯一码各 1）。
- **探针**：`tools/probe-m22-corpus-parity.ps1`（只读 w.r.t. 源与冻结工件；非 `-Json` 时写证据工件 `selfhost/corpus-parity-v0.json`，schema `sico.m22.corpus-parity.v0`）。对每条 `rust_check=refused` 项：
  1. 读源（`[System.IO.File]::ReadAllText`，CRLF→LF）；
  2. `$source | & sico.exe run selfhost/compiler.sico`（`SICO_RUNNER` 指向 `runner/sico-runner/target/debug/sico-runner.exe`，`2>&1 | Out-String`）；
  3. 分类：含 `ERR:` → `refused:<code>`；含 `"schema":"sico.ir.v0"` → `lowered`；其余 → `failed:<snippet>`；
  4. parity：`refused` → `match`；`lowered` / `failed` → `mismatch`（归因见 §4.1）。
- **工件**：每条记录 `{ path, rust_diagnostic_ids, guest_verdict, parity }`；`checked_count` / `mismatch_count` 汇总。确定性序列化（手工递归 JSON，键序冻结，单元素数组保持数组）。

## 3. 结果

探针 `tools/probe-m22-corpus-parity.ps1`（非 `-Json`）全量运行 150 项（2026-11-05 本机，`sico_runner` 限额不变），写入 `selfhost/corpus-parity-v0.json`（schema `sico.m22.corpus-parity.v0`）。汇总：

| 汇总 | 值 |
|---|---|
| `checked_count` | **150** |
| `mismatch_count` | **4**（146 `match`） |

按 Rust 拒绝类别分组（150 = 116 LEXICAL + 34 E-code）：

| Rust 拒绝类别 | 项数 | `match` | `lowered` | `failed:` |
|---|---|---|---|---|
| LEXICAL | 116 | 114 | 0 | 2 |
| E-code（34 项，29 个唯一码） | 34 | 32 | 1 | 1 |
| **合计** | **150** | **146** | **1** | **3** |

每条记录的完整字段在 `selfhost/corpus-parity-v0.json`（`path` / `rust_diagnostic_ids` / `guest_verdict` / `parity`）。4 条 `mismatch` 明细：

| path | rust | guest_verdict | 归因 |
|---|---|---|---|
| `syntax-candidates/b/nominal-invariants/invalid/missing-field.sico` | `E2010` | `lowered` | **真实声音洞**（guest 接受了 Rust 拒绝的） |
| `semantic-cases/future-task/invalid/scope-nesting-limit.sico` | `LEXICAL` | `failed:…FuelExhausted…exit:125` | 燃料墙（与 verdict 正交） |
| `syntax-candidates/b/future-task/invalid/scope-nesting-limit.sico` | `E5105` | `failed:…FuelExhausted…exit:125` | 燃料墙（与 verdict 正交） |
| `syntax-candidates/c/future-task/invalid/scope-nesting-limit.sico` | `LEXICAL` | `failed:…FuelExhausted…exit:125` | 燃料墙（与 verdict 正交） |

## 4. 判读与诚实收窄

### 4.1 verdict parity（本片结论）

- **146/150 项 `match`**（LEXICAL 114/116、E-code 32/34）⇒ guest 前端对绝大部分 Rust 前端在 check 轴拒绝的语料**同样拒绝**。**但 1 项 `lowered`**（下述）打破「无越权接受」的强结论 ⇒ 前端 check 轴 verdict parity **不成立**，W4 verdict 半边**未通过**。
- **1 项 `lowered` = 真实声音洞**：`syntax-candidates/b/nominal-invariants/invalid/missing-field.sico`（rust `E2010`）——guest 前端把 Rust oracle 拒绝的**缺字段 record 构造**降为了 canonical IR（**越权接受**）。这是 W2/W3 前端能力缺口/缺陷的**具名证据**，须逐条进入前端修复账本（`missing-field` 形状的 record 构造未强制字段齐全）；本片**如实登记为缺口**，**不得**改写语料规避。
- **3 项 `failed:`（fuel 耗尽，`exit:125`）**：三个 `scope-nesting-limit.sico` 变体（2 项 `LEXICAL` + 1 项 `E5105`）——深度嵌套使 guest 编译超出 `sico_runner` 燃料限额。这是**与 verdict parity 正交的单独归因**（§4.3 燃料墙，owner 决策叉口），**不读作**「guest 接受」或「guest 拒绝」，不贡献声音洞计数。

### 4.2 本片能关闭什么（不关闭什么）

本片**只回答** §1 的问题（前端 check 轴 verdict parity）。以下各项**均未**由本片关闭：

- **精确 (id, span) parity**：guest 诊断身份是 `E-SH-IR-*`（**44 个唯一码，全部由 `selfhost/parser.sico` 发射**，如 `E-SH-IR-FUNCTION-COUNT` 来自 `parser.sico:22150`），与 Rust oracle 的 `LEXICAL` / `E-code`（34 项拒绝 = **29 个唯一码** `E2001`..`E8010`）**命名空间不相交**（触发位置、span 来源均异）。**verdict parity 是 W4 当前可达的切片**；id/span 精确 parity 需要先定义 guest↔Rust 诊断映射契约，留待后续判断，**不**在本片断言。
- **28 条 `SCRIPT_ABI` 三元组**：这 28 项在 `script-build-corpus-v0.json` 中是 **check-accepted、build-refused**——guest 前端本就拒绝它们（能力缺口，W2/W3 账），**够不到 ABI 检查**。因此在本片把 SCRIPT_ABI 纳入前端 verdict parity 会**测错对象**（测前端能力缺口而非 ABI 语义）。**裁决**：SCRIPT_ABI 的 build 轴校验**过早**，待 W2/W3 前端能力闭合后再以「build 轴」单独出账；本片明确**排除**。

### 4.3 诚实收窄（与既有账并列）

| 通道 | 状态 | 归属 |
|---|---|---|
| W4 verdict parity（前端 150 项） | **本片** | STEP-0561 |
| W4 精确 (id, span) parity | 未达（诊断映射契约未定义） | 后续判断 |
| W4 SCRIPT_ABI build 轴 | 过早（前端能力缺口挡路） | 待 W2/W3 |
| 燃料墙：331 `FuelExhausted` + 24 个 8 MiB trap | **owner 决策叉口** | 上调限额 / 收窄口径均属 owner；唯一自主解为结构预解析（大改、收益不确定） |
| M22 整体 | **NO-GO 维持**（门 1/5/6 GO、门 2 PARTIAL、门 3/6 NO-GO、门 4 未恢复） | 不变 |

### 4.4 结论

本片给 W4 提供**前端轴 verdict parity** 的诚实证据与工件（`selfhost/corpus-parity-v0.json`），并明确列出 W4 不可达与已排除的通道。**结果：verdict 半边未通过**（1 个真实声音洞：`missing-field`/`E2010`），**不是 150/150**。该声音洞连同既有的 4 个 `rust_check=accepted` 而 guest 拒绝的前端能力缺口（W2/W3 账），构成 guest 前端与 Rust oracle 的**双向不一致**，须进入前端修复账本逐条处置。**M22 仍为 NO-GO**，不因本片任何结果改变。
