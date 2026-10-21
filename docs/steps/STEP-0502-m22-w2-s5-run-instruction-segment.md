# STEP-0502 m22-w2 S5 run 体指令段：guest 发射的指令段与 Rust backend 实际发射逐字节相同

> - status: implementation complete locally（未提交快照；待 owner 授权后由独立 CI 取裁决）
> - date: 2026-10-21
> - owners: autonomous-agent
> - depends: [STEP-0501](./STEP-0501-m22-w2-s5-run-body-truth.md)（run 体发射真值与逐字节规则表）、[STEP-0499](./STEP-0499-m22-w2-s5-slot-plan.md)（槽位/局部声明计划）、[STEP-0496](./STEP-0496-m22-w2-s5-ir-ingestion-plan.md)（IR 摄取）
> - gate: 门 3（`A == B == C` + `.sapp` 链）的 S5 波次 = **guest IR 摄取 + run 体发射** —— **本片是发射侧的第二刀（指令段）**
> - numbering: M22 线顺延为 **STEP-0502**（本线已占用 STEP-0491..0499、STEP-0501；并行 M23 会话占用 STEP-0489/0490/0498/0500；已核对 `docs/steps/` 无 0502 冲突）

## 1. 本片目标与结论

**目标**：让 guest 从**摄取的 IR** 推出 `run` 体的**指令段**（dispatcher 外壳 + 块分派头 + 块内指令），并与 Rust backend **实际发射**的指令段 **逐字节相同**。这是 [STEP-0501](./STEP-0501-m22-w2-s5-run-body-truth.md) §6 预切计划里的第 1 刀。

**结论（实测）**：已闭合。

- guest 新增 **28 个函数**（`selfhost/codegen.sico`，全部为 `cgs_*` 新件）+ 新驱动模式 **`--emit-run-instr-hex`**（`selfhost/compiler.sico`）；
- guest 推出的指令段 = **182 hex（91 B）**，**与 Rust backend 实际发射的指令段逐字节相同**（新固件钉 `sico_compiler_emits_the_run_instruction_segment_byte_exactly`，对照读的是 oracle 的**生产输出**）；
- **fail-closed**：子集外的形状一律 typed 拒绝（新固件钉 `sico_compiler_refuses_the_run_instruction_segment_outside_the_subset`）；
- **自举形状前沿未破**：全源 census 仍绿，guest 能降低自己新增的源码。

**门 3 仍为 NO-GO**：本片**只发射指令段**，**Script 结果区返回 + dispatcher 尾声**尚未发射（STEP-0503/0504）；37 语料 / formatter / 全源 canary 阶梯、ADR-0015 `A == B == C` + `.sapp` 链全部未做。

## 2. 为什么"指令段"能独立成刀

STEP-0501 把参考形态的 `run` 体切成两段：**指令段 182 hex（91 B）** 与 **终止符（Script 结果区返回）+ dispatcher 尾声 178 hex（89 B）**。切分标记 `41201001`（`i32.const 32` = 结果区大小 + `call 1` = 分配器）在参考形态里**唯一命中**。

指令段这一刀**不含任何组件级事实**：它只需要槽位（STEP-0499 已钉）、IR 形状（STEP-0496 已钉）与 opcode 编码。三样都已就位 ⇒ 这一刀可以在 STEP-0503 的组件级事实（分配器索引、结果区大小、冻结的 WIT 字段偏移）之前独立完成与对拍。

## 3. 逐字节规则：从真值表落到 guest

| 段 | 规则 | guest 函数 |
|---|---|---|
| dispatcher 外壳 | `i32.const <entry block id>` + `local.set <dispatcher slot>` + `block` + `loop` | `cgs_run_instr_hex` |
| 块分派头 | `local.get <dispatcher slot>` + `i32.const <block id>` + `i32.eq` + `if` | `cgs_run_instr_hex` |
| `project` | 逐槽 `local.get <src>` + `local.set <dst>`；src = 基值槽 + 字段在 record 扁平布局里的槽偏移 | `cgs_project_hex` |
| `const_i64` / `const_u64` | `i64.const <v>` + `local.set <slot>` | `cgs_const_hex` |
| `construct` | 按 op 的 `fields` 顺序把各字段值的源槽**连续**拷进结果槽 | `cgs_construct_hex` |
| `variant` | `i32.const <tag>` + `local.set <tag slot>`；载荷拷进激活臂；**未激活臂清零**（`error` 臂右对齐） | `cgs_variant_hex` |

**槽位与局部声明段互证**：本片推出的槽位（形参 0–3、结果 4–22、dispatcher 23）与 STEP-0499 实测的局部声明段 `017f…027f` 是同一套编号；指令段的每条 `local.get`/`local.set` 都落在这个编号里。

**两个新的编码助手**（guest 侧新增）：
- `cgs_sleb_hex`：**有符号** LEB128。与无符号形式只在"末字节第 6 位为 1"时不同 ⇒ `64` 编码为 `c000` 而不是 `40`。`i32.const` / `i64.const` 用它，`local.get` / `local.set` 仍用既有的无符号 `cg_leb_hex`。
- `cgs_decimal_u64` / `cgs_decimal_ok`：十进制解析，**至多 10 位**（⇒ 不溢出、落在 i64 正半区）。

## 4. guest 侧实现（`selfhost/codegen.sico`，28 个新函数）

| 组 | 函数 | 职责 |
|---|---|---|
| 算术 | `cgs_add` / `cgs_sub` / `cgs_mul` | checked 运算的饱和包装（溢出/下溢时给出确定的退化值） |
| 十进制 | `cgs_is_digit_byte` / `cgs_decimal_digit` / `cgs_decimal_ok` / `cgs_decimal_bad` / `cgs_decimal_u64` | 块号、入口块号、`const_i64` 常量的解析与校验 |
| 编码 | `cgs_sleb_hex` / `cgs_local_get_hex` / `cgs_local_set_hex` / `cgs_i32_const_hex` / `cgs_i64_const_hex` | 指令字节 |
| 布局 | `cgs_type_slots` / `cgs_named_text` / `cgs_flat_slot_kind` / `cgs_zero_hex` | 类型的扁平槽数、命名类型的记录名、扁平槽的 i32/i64、槽清零 |
| 槽位 | `cgs_value_slot` / `cgs_value_ty` / `cgs_dispatcher_slot` | 值的 1 基槽号、值的类型偏移、dispatcher 槽 |
| 记录 | `cgs_record_field_offset` / `cgs_record_field_slots` | 字段在记录扁平布局里的槽偏移与槽宽 |
| 发射 | `cgs_project_hex` / `cgs_construct_hex` / `cgs_variant_hex` / `cgs_const_hex` / `cgs_op_hex` / `cgs_run_instr_hex` | 逐 op 的字节与整段组装 |

**驱动模式**：`--emit-run-instr-hex`（`selfhost/compiler.sico`），与 `--emit-locals-hex` 同规：读 canonical IR ⇒ 调 `codegen.cgs_run_instr_hex` ⇒ 以 hex 文本输出；typed 身份以 `ScriptError(InvalidInput, "<identity>")` 抛出。

**两个 typed 身份**：

| 身份 | 触发 |
|---|---|
| `ERR:E-SH-CG-IR-PLAN` | IR 里没有 `run`、缺 `blocks` / `instructions` |
| `ERR:E-SH-CG-IR-EMIT` | op 不在支持子集内、形状越界（槽宽不一致、字段/记录缺失、常量超出可解析范围等） |

## 5. 踩到并修正的 **guest 前端约束**（四条真约束 + 两条自造缺陷）

本片第一次把新代码放进 `selfhost/codegen.sico`，立刻踩到 guest 自举降级机器的形状/语法前沿。**每一条都是实测的**，不是推测：

| # | 现象 | 根因 | 修法 |
|---|---|---|---|
| 1 | `sico build` 报 `unsupported atomic expression at bytes 66804..66810`（范围正好是 `record`） | **`record` 是 Sico 保留关键字**（`crates/sico-lexer/src/lib.rs:384`），lowerer 遇关键字 token 一律拒绝；我把它用作形参名 | 形参改名 `record_name`（与 STEP-0496 记录的 `from` 同类约束） |
| 2 | `unsupported cell redeclared`（两处） | **同函数内 `let` 无块作用域**：① `if`/`else` 两个分支里各自 `let` 同名；② 两个 `while` 循环体里各自 `let` 同名 | 模式固定为**声明一次（`let`）+ 分支里 `set`**；同一函数内所有 `let` 名必须唯一 |
| 3 | `E2001 expected U64, found Bool`（三处） | `U64.equal(a, b)` 的**返回值是 Bool**，不是 U64 ⇒ 不能写 `U64.equal(<Bool>, false)` | 取否定写成助手函数（`cgs_decimal_bad`）；布尔标志一律用 U64 承载 |
| 4 | `E2001 expected U64, found Bool`（第四处） | 同上，但形态是 `let flag = U64.equal(...)` 之后又 `U64.equal(flag, 1)` —— 双重包装 | 直接用布尔变量作条件（`if csh_sign:`） |

另外两条是**本片自造的缺陷**，被字节对拍当场抓出，值得留档：

| # | 现象 | 根因 | 修法 |
|---|---|---|---|
| 5 | `cgs_dispatcher_slot` 恒返回 0 | 我用"空 id 当哨兵"的技巧想一次拿到槽位总数，但 `cgs_value_slot` 的"找不到"返回也是 0 —— **"找不到"与"总数"不能共用同一个 0 哨兵** | 改为显式的 `cgs_dispatcher_slot` 走查（参数 + 指令结果求和） |
| 6 | 指令段在 `variant` 处整体 typed 拒绝 | 我的"结果槽宽 ≠ 期望槽宽"标志位**写反了**（相等时置 1，却按"不相等"读取） | 改为"默认不相等，相等时清零"；并把这条写进纪律：**布尔标志必须用中性名字 + 明确默认值** |

第 1、2 条已写入 [handoff-m22.md](../handoff-m22.md) §5；第 5、6 条是本线的内部教训，写在这里。

## 6. 证据

| 证据 | 命令 | 结果 |
|---|---|---|
| 指令段逐字节等同（生产输出对拍） | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler sico_compiler_emits_the_run_instruction_segment_byte_exactly` | **ok** |
| 子集外形状 fail-closed | 同上，`sico_compiler_refuses_the_run_instruction_segment_outside_the_subset` | **ok**（`invalid-input` / `ERR:E-SH-CG-IR-PLAN`） |
| guest 直跑取证（不经测试框架） | `sico build --profile script-v0` 出组件 → `sico-runner --fuel 800000000000 <component> -- --emit-run-instr-hex`（stdin 投 Script-profile echo fixture） | exit 0，stdout **182 hex**，与 oracle 指令段逐字节相同 |
| 固件套件 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler --test selfhost_parser --test selfhost_checker --test selfhost_local_bounds` | **9 + 99 + 1 + 2 = 111 passed / 0 failed**（selfhost_compiler 由 97 增至 99，+2 即本片的两枚新钉） |
| 全源 census（自举形状前沿） | 同上（`sico_guest_arena_accommodates_full_parser_selfhost_input`） | **ok** |
| 前端前沿探针 | `tools/probe-m22-frontend-frontier.ps1` | **18/18** `FRONTIER_UNCHANGED` |
| IR 计划层前沿探针 | `tools/probe-m22-ir-plan-frontier.ps1` | **8/8** `PLAN_FRONTIER_UNCHANGED` |
| 一键 CI | `tools/run-ci.ps1` | **12/12** 全 PASS（空闲重跑；首轮红点见下） |
| 行尾审计 | 10 个改动文件逐字节扫描 | **纯 LF、零 CRLF、零 BOM**；`git diff --check` exit 0 |

### 6.1 首轮一键 CI 红点的归因（已登记的环境敏感断言）

首轮一键 CI 红在 `prepared_program_reruns_are_isolated_and_survive_a_trap` 的 warm median 预算断言
（实测 **22.1591 ms** vs 预算 **20 ms**）。这不是本片引入的回归，三条证据齐备：

1. **失败二进制的源码与 `HEAD` 逐字节相同**：`git diff --quiet HEAD -- runner/sico-runner/tests/runner.rs` 返回 **0**
   （未改动）。本片在 runner 侧只改了**另一个**测试二进制 `selfhost_compiler.rs`（+53 行，纯 Rust，不跑 guest），
   `cargo test --test runner` 根本不编译它。
2. **该断言已在仓库里被登记为环境敏感**：`runner/sico-runner/tests/runner.rs:1460-1470` 的注释写明
   "fails under heavy parallel load, passes solo"；[STEP-0499](./STEP-0499-m22-w2-s5-slot-plan.md) §8 与
   [STEP-0501](./STEP-0501-m22-w2-s5-run-body-truth.md) §7 各记录过一次同样的红点。
3. **完整实测分布**：单独重跑该断言 **9 次**，warm median = 16111 / 16904 / 16819 / 17998 / 16794 / 17274 /
   16839 / 18335 / 16111 µs，**9/9 通过、0 次超预算**（中位 16904 µs，距预算线尚有余量）；
   紧接着的空闲一键 CI **12/12 全绿**（`test (runner)` 2:56 PASS）。

**处置**：**不改预算常量、不改该断言**（与 STEP-0499/STEP-0501 的处置一致）。权威裁决仍交独立 CI。

## 7. 诚实性

- **本片只发射指令段**。`run` 体的 **Script 结果区返回 + dispatcher 尾声**（178 hex）**尚未**由 guest 发射 ⇒ **整段 run 体还没有逐字节等同**，更不用说整个组件。STEP-0503 / STEP-0504 才是那两刀。
- **支持子集**（超出即 typed 拒绝 `ERR:E-SH-CG-IR-EMIT`）：
  - `project`：**基必须是形参**且其类型是**已声明 record**（字段槽偏移取自记录声明）；
  - `construct`：构造名必须是已声明 record，且**字段值槽宽之和 == 声明类型的槽宽**（不等即拒绝）；
  - `variant`：只支持 `ok` / `error` 两个臂，tag 取**冻结的 canonical ABI 判别值**（ok=0 / error=1），不是按声明顺序；
  - `const_i64` / `const_u64`：常量必须是**非负十进制、至多 10 位**（⇒ 解析不溢出且落在 i64 正半区）；**负值与更大值一律 typed 拒绝**。
- **槽位计数用"声明类型"，oracle 用"推断布局"**。二者在参考形态上相等；本片在 `project` / `construct` 上**显式校验两者相等**，不等即 typed 拒绝（fail-closed，不静默取一侧）。
- **`variant` 的 `error` 臂右对齐**已按源码实现，但参考形态**不触发** ⇒ 该分支只有源码依据，没有真值对拍（承接 STEP-0501 的同一条诚实收窄）。
- **参考形态之外无法对拍**：oracle 的 `compile_script_program` 只接受 Script v0 入口签名，其它形态根本不发射组件。
- **guest 的 `let` 无块作用域**这条硬约束意味着本片新增的 28 个函数里，每个函数的局部名必须**全局唯一**（同函数内）；这是本片代码量最大的隐性成本。
- 本片**没有**触碰 37 语料阶梯、`.sapp` 链或 ADR-0015。门 3 仍为 NO-GO。

## 8. 下一片

1. **STEP-0503 = Script 结果区返回**：需要五项**冻结**的组件级事实（分配器索引 1、结果区 32 B、载荷起始偏移 8、`ScriptOutput` 字段 @0/8/16、`ScriptError` @0/4）；本片的指令段可与之拼接成完整的 `run` 体。
2. **STEP-0504 = 用派生 run 体替换 STEP-0487 的冻结 run 体 hex**：参考形态的**整个组件**逐字节等同（参考形态无 data 段 ⇒ run 体是唯一的 IR 相关部分）。
3. 然后才是 **37 语料 → formatter → 全源 → ADR-0015 `A == B == C` + `.sapp`** 的 canary 阶梯。
4. （**可选**）仿 `tools/probe-m22-ir-plan-frontier.ps1` 加一个 `--emit-run-instr-hex` 的**前沿探针**：本片的两枚固件钉已经覆盖了"成功路径"与"IR 里没有 `run`"这一条拒绝路径，但**探针**能把"子集内可发射 / 子集外 typed 拒绝"的边界整张表钉下来。本片**没有**做这一项（owner 拍板的范围是"发射指令段 + 对拍 182 hex"），登记为后续可做。

> 提醒（承接 [handoff-m22.md](../handoff-m22.md) §4/§5 与 STEP-0496 §6）：guest 前端的**形状前沿**与**语法/语义纪律**共同构成"新代码必须能被 guest 自己降低"的全部已知约束；本片新增的 28 个函数刻意保持**顶层浅嵌套**（`while` 体 → `if`/`else` → 单层 `while`），未触碰既有前沿。
