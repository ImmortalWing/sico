# STEP-0499 m22-w2 S5 槽位计划：guest 局部声明计划与 Rust oracle 实际发射逐字节相同

> - status: implementation complete locally（未提交快照；待 owner 授权后由独立 CI 取裁决）
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0496](./STEP-0496-m22-w2-s5-ir-ingestion-plan.md)（IR 摄取 + `run` 发射计划）、[STEP-0488](./STEP-0488-m22-w2-s5-frontend-record-variant-gap.md) §4 第 5 项（IR 扫描器 + 布局/槽位计划 + op 发射 + Script 返回发射）
> - gate: 门 3（`A == B == C` + `.sapp` 链）的 S5 波次 = **guest IR 摄取 + run 体发射** —— **本片是发射侧的第一刀（槽位/局部声明计划）**
> - numbering: M22 线顺延为 **STEP-0499**（本线已占用 STEP-0491..0497；**并行 M23 会话在 `36f0ee9` 提交并占用了 STEP-0498**（`STEP-0498-m23-item3-corpus-freeze-and-d3-record.md`），故本片让位重编号为 STEP-0499 —— 已核对 `docs/steps/` 无 0499 冲突；STEP 编号是跨并行 worktree 的共享序列，已提交的取号优先，未入库的让位）

## 1. 本片目标与结论

**目标**：让 guest 从**摄取的 IR** 推出 `run` 函数的**局部声明计划**（槽位分配 + 局部声明分组），并与 Rust backend **实际发射**的 local declaration section **逐字节相同**。这是 [STEP-0496](./STEP-0496-m22-w2-s5-ir-ingestion-plan.md) §10 第 1 项（槽位/布局计划）。

**结论（实测）**：已闭合。

- guest 新增 `cgs_run_function` / `cgs_flat_hex` / `cgs_record_hex` / `cgs_local_hex`（`selfhost/codegen.sico`）+ 新驱动模式 **`--emit-locals-hex`**（`selfhost/compiler.sico`）；
- guest 推出的局部声明字节 = `017f017f017f017f017e017f017f017f017f017e017f017f017f017f017f017e017f017f017f017f027f`（84 个 hex 字符 = 21 组 = 42 字节），**与 Rust backend 实际发射的组件里的 local declaration section 逐字节相同**；
- **对照方式是"读 oracle 的生产输出"**，不是复刻：测试侧用 `wasmparser` 打开 `compile_script_program` 真正产出的组件，取 core module 导出 `run` 的函数体，读它的局部声明分组再编码 ⇒ 比 STEP-0496 的"测试文件内复刻计划生成器"更强；
- **自举形状前沿未破**：全源 census（`sico_guest_arena_accommodates_full_parser_selfhost_input`）仍绿，guest 能降低自己新增的源码；
- 两个前沿探针无回归：前端 **18/18**、IR 计划层 **8/8**。

**门 3 仍为 NO-GO**：**run 体字节发射**（把计划变成 run 体的指令序列）、37 语料 / formatter / 全源 canary 阶梯、ADR-0015 `A == B == C` + `.sapp` 链全部未做。

## 2. 为什么先钉"局部声明计划"，而不是直接发射 run 体

run 体的每一条指令都写着**槽位索引**（`local.get <slot>` / `local.set <slot>`）。槽位错一位，字节就全错，而且错法与"指令顺序错了"无法区分。于是把发射拆成两刀：

```
摄取 IR ──> 槽位/局部声明计划 ──(下一片)──> run 体字节
   │              ▲
   └──── Rust oracle 实际发射的字节对拍 ────┘
```

本片只做中间层，而且**中间层本身就是可对拍的字节**（局部声明段），不是文字描述 —— 这样下一片发射 run 体时，一旦对不上，能立刻区分"槽位算错了"还是"指令发射写错了"。

## 3. 槽位算法（Rust oracle 真值，`LocalLayout::plan`）

`crates/sico-codegen-wasm/src/lib.rs` 的 `LocalLayout::plan`（`lib.rs:1812-1940`）：

1. **参数**按声明顺序取扁平槽（形参**不**进局部声明段）；
2. **指令结果**按 IR 顺序取槽，每个结果占 `flat` 个槽，每个槽各成一组 `(1, valtype)`；
3. **函数局部 cell** 紧随其后（Script profile 的参考形态没有 cell）；
4. **dispatcher** 一个 i32 槽；
5. **scratch**（仅 Script profile）`(return_area, copy_destination)`，**合成一组 `(2, i32)`**。

扁平规则（`canonical.rs::flat_ir_types`）：

| IR 类型 | 扁平 |
|---|---|
| `bool` | 1 × i32 |
| `i64` / `u64` | 1 × i64 |
| `string` / `bytes` | 2 × i32（ptr,len） |
| `list(string\|i64\|u64)` | 2 × i32 |
| `named(N)`（N 是模块自己声明的 record） | 该 record 各字段扁平顺序拼接 |
| `named(ScriptErrorCode)` | 1 × i32 |
| `result(ok,error)` | 1 × i32（tag）+ ok 扁平 + error 扁平 |

参考形态（Script profile echo）的实际数字：4 个参数槽（不进声明段）+ 19 个指令槽 + dispatcher 23 + scratch 24/25 ⇒ **21 组局部声明**。

## 4. guest 侧实现（`selfhost/codegen.sico`）

| 函数 | 职责 |
|---|---|
| `cgs_run_function` | 在 IR 的 `functions` 里找 `run`（找不到 ⇒ `ERR:E-SH-CG-IR-PLAN`） |
| `cgs_flat_hex` | 某个 IR 类型的局部声明字节；不可扁平 ⇒ `""`（**不近似**） |
| `cgs_record_hex` | 模块自己声明的 record 的扁平字节（字段按声明顺序）；字段有不可扁平者 ⇒ `""` |
| `cgs_local_hex` | `run` 的完整局部声明字节：指令槽（IR 顺序）+ dispatcher `017f` + scratch `027f` |

**关键实现取舍**：计划**直接承载为局部声明字节**（`01 7f` = 一个 i32 槽，`01 7e` = 一个 i64 槽，`02 7f` = scratch 对），而不是"槽位类型列表"再二次编码 —— 少一层可出错的转换，且对拍直接是字节。

**两个 typed 身份**（ADR-0016 决策 6，越界形状 fail-closed、禁止静默回退）：

| 身份 | 触发 |
|---|---|
| `ERR:E-SH-CG-IR-PLAN` | IR 里没有 `run`、缺 `blocks`/`instructions`/`ty` |
| `ERR:E-SH-CG-IR-FLAT` | 某个值落在扁平子集之外（`unit`/`int`/`float64`/`map`/`set`、非可扁平字段的 record、`list` 的元素类型不匹配等），或 `run` 声明了局部 cell |

## 5. 对照方式：读 oracle 的生产输出（不是复刻）

`runner/sico-runner/tests/selfhost_compiler.rs` 新增：

- `oracle_run_locals_hex(source)`：`sico_codegen_wasm::compile_script_program` **真正产出**组件 → `wasmparser` 打开 → 找 core module 导出 `run` 的函数索引 → 取该函数体的局部声明分组 → 重新编码成与 guest 相同的 hex；
- 正钉 `sico_compiler_emits_the_run_local_declaration_plan_byte_exactly`：guest 的 `--emit-locals-hex` 输出 == oracle 实际发射的字节；并钉住 21 组 / 84 hex 字符 / 结尾 `017f027f`（dispatcher 后紧跟 scratch 对）。

**这一步比 STEP-0496 强**：STEP-0496 的 oracle 侧是**测试文件内复刻**计划生成器（因为 `LocalLayout` 是私有的）；本片直接读**生产输出**，所以对拍的是"guest 的槽位分配 == backend 真的发射出来的槽位分配"，而不是"guest 的模型 == 我另写的一个模型"。

## 6. 顺带修正一处**规范与实现矛盾**（同范围内修复）

`docs/development/script-component-emission-v0.md` §3 原写："`ScriptError.code` is an enum and therefore not flattenable, so `ScriptError` always uses the WIT layout"。**与实现不符**：`flat_ir_types` 把 `ScriptErrorCode` 映射成 1 个 i32 槽，所以 `ScriptError` **是可扁平的**，用的是**模块自己声明的布局**（flat `[i32,i32,i32]`，字段字节偏移 0 与 8），而 WIT 布局（偏移 0 与 4）是 §6 **结果区**的布局。两者不可混为一谈。已按纪律在同一范围内改正，并写明字段类型落在 Script v0 边界形状之外的 record 保持"整体不注册 + typed 拒绝"。

## 7. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 局部声明逐字节等同（生产输出对拍） | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler sico_compiler_emits_the_run_local_declaration_plan_byte_exactly` | **ok** |
| guest 直跑取证（不经测试框架） | `sico build --profile script-v0` 出组件 → `sico-runner --fuel 800000000000 <component> -- --emit-locals-hex`（stdin 投 Script-profile echo fixture） | exit 0，stdout 84 hex = `017f017f017f017f017e017f017f017f017f017e017f017f017f017f017e017f017f017f017f017f027f`（21 组） |
| 固件套件 | `... --test selfhost_compiler --test selfhost_parser --test selfhost_checker --test selfhost_local_bounds` | **96 + 2 + 9 + 1 = 108 passed / 0 failed** |
| 全源 census（自举形状前沿） | 同上（`sico_guest_arena_accommodates_full_parser_selfhost_input`） | **ok** |
| 前端前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | **18/18** `FRONTIER_UNCHANGED` exit 0 |
| IR 计划层前沿探针 | `tools/probe-m22-ir-plan-frontier.ps1` | **8/8** `PLAN_FRONTIER_UNCHANGED` exit 0 |
| **完整验证电池**（5 步全绿） | 0245 M22 回归 / 0262 M22 checker / 前端前沿探针 / IR 计划层前沿探针 / `tools/run-ci.ps1` 一键 CI | **BATTERY_GREEN**（exit 0 / 0 / 0 / 0 / 0；一键 CI **11/11** 全 PASS，`test (workspace)` 1:40、`test (runner)` 3:24） |
| 行尾审计 | 11 个改动/新增文件逐字节扫描 | **纯 LF、零 CRLF、零 BOM** |

## 7.1 顺带暴露的一个 **CI 覆盖缺口**（实测；**owner 拍板后本片已修**）

本片第一次跑验证电池时，**同一棵树上**出现了互相矛盾的两个结果：

| 步骤 | 结果 |
|---|---|
| `tools/run-ci.ps1` 的 `fmt` 步骤 | **PASS** |
| `tools/validate-step-0245.ps1` 的 **runner fmt** 步骤 | **FAIL**（`runner/sico-runner/tests/selfhost_compiler.rs` 两处 rustfmt 排版不合规） |

**根因**：`runner/sico-runner` 是**独立 Cargo 工作区**，不在根 `Cargo.toml` 的 `members` 里（根只含 28 个 `crates/*` 与 `pilots/*`）。因此根命令 `cargo fmt --all -- --check` **不覆盖** runner 工作区；而 `tools/run-ci.ps1` 的 11 步里**没有** `fmt (runner)` 步骤，唯一的 runner fmt 检查在 `tools/validate-step-0245.ps1` 里，而 **0245 不在 run-ci 的 11 步内**，CI 工作流 `.github/workflows/m22-w1-ci.yml` 又只调用 `tools/run-ci.ps1`。

⇒ **结论（可复现）**：`runner/sico-runner/**` 的 rustfmt 违规**能同时通过本机一键 CI 与独立 CI**。（`clippy (runner)` 步骤在，但 clippy 不检查排版。）本片的违规已按 rustfmt 输出对齐修好（`cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` 现 exit 0），**且缺口本身已在本片修复**（见下）。

**owner 拍板后本片的修法**：给 `run-ci.ps1` 补第 12 步 `fmt (runner)`（与既有 `clippy (runner)` 步骤同形）——

```powershell
Invoke-Step 'fmt (runner)' { & $cargo fmt --manifest-path .\runner\sico-runner\Cargo.toml --all -- --check; $LASTEXITCODE }
```

**随之必须同步的表述**（一步之内层步数从 11 变 12）：**刚被裁决为 GO 的 M23 出口审计（STEP-0500）**以"独立 CI 的 `windows-gnu` job **11/11** 内层步骤全 PASS"作为裁决证据之一。两处按**不篡改历史**的方式同步：① [STEP-0500](./STEP-0500-m23-exit-verdict-independent-ci-green.md) §2 追加「步数口径说明」——那两轮 run 的 11/11 是**历史事实**、裁决证据不变，自本片起后续 run 为 **12/12**；② [M23 出口审计](../reports/m23-exit-audit-2026-10-21.md) §2 追加同义的英文 *Step-count note*。**没有**把历史 run 的 11/11 改写成 12/12。

**本片的修法为什么必须落在本片**：缺口是本片在**实现过程中**实测暴露的（同一棵树上两个工具给出互相矛盾的结果），而且它**直接让本片自己的一次验证红了**；把它留给后续片，等于明知 CI 仍会放过同类违规却继续推进。

## 8. 诚实性

- **本片只做槽位/局部声明计划**，**没有**发射任何 run 体字节；STEP-0487 的 `--emit-component-hex` 仍是**冻结的最小 echo 骨架**（run 体是硬编码 hex 常量），未接到计划上。
- **收窄显式记录**：本片的 oracle 对拍只覆盖 **Script profile 的参考形态**（`compile_script_program` 只接受 Script v0 入口签名，其它形态 oracle 根本不发射组件，故无法对拍）；guest 侧不支持的形状一律按 typed 身份拒绝，未做静默近似。`list[record]`（backend 在 record 全为 i64 时接受 2 × i32）本片**未**实现 ⇒ 走 `ERR:E-SH-CG-IR-FLAT`。
- **与 oracle 的精确分歧（`named` 类型，guest 更窄的一侧）**：oracle 的 `flat_ir_types` 对 `named(N)` 的判定顺序是 ① **模块自己声明的 record**（`user_records`）→ ② 内建 `ScriptInput` / `ScriptOutput` / `ScriptError` / `HttpResponse` / `Http2Response` 的已注册布局 → ③ `ScriptErrorCode` / `InputStream` / `OutputStream` ⇒ 1 × i32 → ④ 其它 ⇒ 拒绝。guest 只实现 ① 与 ③（顺序与 oracle 一致），**② 一律 typed 拒绝**（`ERR:E-SH-CG-IR-FLAT`）。即：**模块未声明**这些内建 record 时，oracle 会扁平而 guest 拒绝 —— guest 在这一侧**更窄**，是 fail-closed 方向，不构成静默近似。
- **判定顺序要点（已与 oracle 逐字对齐）**：`register_user_records` 对**字段不可扁平**的 record **不注册**（`flattenable = false` ⇒ 不入 `user_records`），因此这种 record 会继续落到内建名分支 —— guest 的"先查模块声明的 record、查不到再判 `ScriptErrorCode`"次序正好复现该行为。**若把这两个次序写反，就会把一个不可扁平的自声明 record 误判成 1 × i32**。
- `run` 声明**局部 cell** 时本片 typed 拒绝（`ERR:E-SH-CG-IR-FLAT`），未实现 cell 槽位。
- **首次一键 CI 的红点归属（诚实记录，含完整实测）**：本片跑 `tools/run-ci.ps1` 时 `test (runner)` 多次红，红的都是 `prepared_program_reruns_are_isolated_and_survive_a_trap` 的**环境敏感预算断言**（warm median 超 20 ms 预算），**不是**功能失败，也**不是**本片引入的代码回归 —— 归因证据有三条：① **产物逐位相同**：`runner/sico-runner/src`、`runner/sico-runner/tests/runner.rs`、`runner/sico-runner/Cargo.toml`、`Cargo.lock` 相对 `HEAD` **零改动**（`git diff HEAD --` 为空），失败的测试二进制编译自与 HEAD **完全相同**的源码；本片在 runner 工作区只动了 `tests/selfhost_compiler.rs`，那是**另一个测试二进制**；② **该断言本来就登记为环境敏感**：`tests/runner.rs` 的注释写明"STEP-0454/0484 把 guest 线性内存上限抬到 2 GiB 后，参考主机实测 warm median 从 6.5 ms（128 MiB 天花板）升到 17.4 ms，预算常量仍是 20 ms"，且它是 **M19 计划 §3.1 登记的两个环境敏感预算断言之一**（"重载下失败、单独跑通过"）；③ **本机实测分布**（预算 20,000 us）：

  | 场景 | 实测中位数（us） | 结果 |
  |---|---|---|
  | 单独跑该用例 ×5 | 17,635 / 16,229 / 17,959 / 17,295 / 16,824 | 5/5 通过（中位数 ≈ 17.3 ms，与注释里的参考值 17.4 ms 吻合） |
  | 整套件序列，**跳过**本片新测试 ×3 | 16,589 / 16,509 /（一次未取样） | 3/3 通过 |
  | 整套件序列，**包含**本片新测试 ×7 | 18,185 / 19,897 / 22,528 / 20,878 / 21,251 / 23,702 /（一次通过） | 3 通过 / 4 红 |

  **诚实读数**：跳过与包含两组的**中位数存在约 2–3 ms 的稳定落差**（skip ≈ 16.5 ms，include ≈ 18–19.9 ms）——本片新增的固件钉**确实多了一次 2 GiB guest 实例**（`--emit-locals-hex` 模式必须实跑 guest 才能拿到输出），它把整机页池推到"2 GiB 清零变慢"的一侧；**红点还需要额外的主机负载**才会越过 20 ms 预算线（本机物理内存 15.61 GB、当时仅约 3 GB 空闲；2 GiB 线性内存预留 ≈ 2 GiB 清零 ≈ 17 ms，正落在预算线的 15% 余量内）。**本片没有改预算常量，也没有改该断言**（`tests/runner.rs` 零改动）。
  **裁决路径**：该断言的权威裁决是**独立 CI**（独立 runner、干净页池、串行执行）；历史两轮（`39524849306` / `39525367456`）在独立 runner 上均通过（`11/11`）。若独立 CI 因本片新增的那一次 guest 实例而红，则应按"预算余量已低于本机噪声"重新基线化该常量（需 owner 拍板），而不是删掉本片的字节对拍钉。
- 计划文本（STEP-0496 的五类行格式）**未改动**；本片是**独立**的字节产物，不复用也不修改那套文本格式。
- 本片**没有**触碰 37 语料阶梯、`.sapp` 链或 ADR-0015。门 3 仍为 NO-GO。

## 9. 下一片：run 体字节发射

1. **指令发射**：把 STEP-0496 的计划（`op` 行）与本片的槽位计划合成 run 体字节 —— `project`（逐槽 copy）/ `construct`（连续槽段 copy）/ `const_i64` / `variant`（tag + 归零 + 拷贝）+ `return` 终止符 + dispatcher 外壳。
2. **Script 返回发射**：§6 的结果区降低（`alloc` → 写 tag → 按字段拷贝 → `memory.copy` → 返回 `return_area`）。
3. 然后才是 **37 语料 → formatter → 全源 → ADR-0015 `A == B == C` + `.sapp`** 的 canary 阶梯。

> 提醒（承接 [handoff-m22.md](../handoff-m22.md) §4/§5 与 STEP-0496 §6）：guest 前端的**形状前沿**与**语法/语义纪律**共同构成"新代码必须能被 guest 自己降低"的全部已知约束；本片新增的 `cgs_*` 四个函数刻意保持**顶层浅嵌套**、`if` 链 + 单层 `while` 的形状，未触碰既有前沿。
