# STEP-0516 m22-w2 S5 **S5-W1 规模可行性测量片**（纯测量）：1 MiB 级字节组装在合格限额内**可行**（线性策略 66 ms / fuel 24.1 M），二次方策略在 256 KiB 处撞 arena 上限

> - status: **measurement complete locally（本机实测绿，未提交未推送）**
> - date: 2026-10-23
> - owners: autonomous-agent
> - depends: [STEP-0514](./STEP-0514-m22-w2-s5-s6-closure-ladder.md)（owner 规划片：闭合阶梯）、[STEP-0515](./STEP-0515-m22-w2-s5-host-call-let-binding.md)（上一执行片）
> - gate: **不推进任何出口门** —— W1 是**纯测量**（M22 计划 §3.3.4），产出是 D6 三档判读
> - numbering: **STEP-0516**（取号前已核对 `docs/steps/`：最大号 0515，**0516 未被占用**；M24 占 STEP-0330–0337、M23 占 0489/0490，均不冲突）

## 0. 为什么这一片现在可以开（入口门槛）

| 入口条件 | 状态 |
|---|---|
| owner 已授权「插入纯测量片并前置」 | ✅ 决策 1（STEP-0514 §4.2） |
| 判读阈值已固定（不许执行会话自行放宽） | ✅ D6：≤6× 可行 / ≤16× 临界 / >16× 不可行（计划 §3.3.4） |
| 探针落点已指定（不进 `selfhost/`） | ✅ D5：放 runner 测试侧独立 fixture，由 Rust 后端单独编译；**不得进入 A/B/C 链** |
| 有界出口测试 | ✅ §3（1 条测量钉，含 3 条结构化断言） |

⇒ 取号合法。**测量已完成**，见 §4。

## 1. 这一片回答的问题（阶梯的决策枢纽）

计划 §3.3.3 把 W1 称为**整个闭合阶梯的决策枢纽**：

> **W1 的三档判读是本阶梯的决策枢纽**：判读为"不可行"即触发止损 T-R3-a，停止 W2/W3 的顺序推进并转入 ADR / RFC 修订流。

原因是量化落差（计划 §3.3.1）：固定点要求 guest 发射 **≈1,024,613 B** 的组件，而 guest 已发射过的最大组件是 **1,025 B** ⇒ **约 1000 倍**。若 1 MB 级发射在合格限额内根本做不到，那么 W2（20 个内建族）、W5（bundle 组件级全等）、W6（ADR-0015 自举闭环）整条链的**实施方式**都要改。

**本片的结论：1 MiB 级发射在合格限额内可行（tier = 可行）**，但**可达性强烈依赖字节组装策略** —— 这是一条 W2/W5 必须遵守的**实施约束**。

## 2. 测量设计

### 2.1 探针落点（D5 遵守情况）

| 要求 | 本片做法 |
|---|---|
| 探针**不进 `selfhost/`**（否则扰动冻结源集 S 与 `A = R(S)` 基线字节） | ✅ 探针源在**测试进程内生成**，组件由 `sico_cli::run(["sico","build","--profile","script-v0",…])` 单独编译 |
| 探针组件**不进 A/B/C 链** | ✅ 只在本片自己的测试里跑 |
| 探针**不进自举形状前沿** | ✅ 与 `selfhost/` 无任何关系 |
| 测量路径与生产路径**分离** | ✅ 独立测试文件 `runner/sico-runner/tests/m22_scale_probe.rs` |
| 测量用**与合格跑相同的限额** | ✅ fuel **800 B**（800,000,000,000）/ timeout **120 s** / 内存上限 **2560 MiB**（runner 默认） |

### 2.2 三种字节组装策略

| 策略 | 形状 | 拷贝量级 |
|---|---|---|
| **S1 — 增量 `sico.text.concat`** | 一个累积串，`n` 次拼接 | **O(n²)**（每次拼接复制整个累积串） |
| **S2 — 分块 `sico.arena.push_chunk`** | `n` 次 push 到 arena chunk 表，末段一次拼装 | **O(n²)**（每次 push 重建 chunk 表） |
| **S3 — 线性对照** | **一次** `sico.text.concat(left, left)`，两个大操作数 | **O(n)** |

S3 的两个大操作数从 **stdin** 进来（不是源字面量）：源字面量达到这个尺寸会触发 `LineTooLong` **源契约**（实测：`source contract error LineTooLong at bytes 453..1049045`），那会测成「源契约」而不是「发射规模」。runner 的通道预算 8 MiB 覆盖本片阶梯。

### 2.3 记什么量、怎么记（D13 口径）

Runtime ABI **没有** fuel 实际耗量读数出口，**也没有** guest 峰值内存读数出口（runner 只用它判 `FuelExhausted` / `MemoryLimit`）。所以本片**不假装读数**，改为测**所需上限**：

- `min_fuel` = 让探针仍能完成的**最小 fuel 限额**（二分）⇒ 是耗量的**上界**，不是读数；
- `min_mem` = 让探针仍能启动的**最小内存上限**（二分）⇒ 同上；
- `wall_ms` = 宿主侧实测墙钟。

失败按类型**分类**（`arena-exhausted` / `fuel-exhausted` / `memory-limit` / `timeout`），因为 arena 上限是注册在 `unreachable` 上的 **Wasm trap**，与 typed 的 `FuelExhausted` **不是一回事**，合并会掩盖真实原因。

## 3. 出口测试（有界）

| # | 钉 | 口径 |
|---|---|---|
| 1 | `the_one_megabyte_emission_scale_is_measured`（`runner/sico-runner/tests/m22_scale_probe.rs`） | 测三条策略 × 各自阶梯；写证据文件 `target/tmp/step0516/scale-measurement.txt`；**并断言 3 条结构化结论**：① S3 达到 **1 MiB** 且 fuel 远低于合格限额、增长曲线 **≤ D6 可行阈值**；② S1 **仍在 arena 上限处失败**且**到不了 1 MiB**；③ 实测内存下限 **== 声明的线性内存**（33,792 页 = 2,214,592,512 B） |

## 4. 实施记录（2026-10-23）

### 4.1 改动清单

| 文件 | 改动 |
|---|---|
| `runner/sico-runner/tests/m22_scale_probe.rs` | **新增**：W1 测量探针 + 3 条结构化断言（唯一新增代码） |
| `selfhost/*.sico` | **零改动**（纯测量） |
| oracle 生产代码 | **零改动** |
| `docs/plans/M22-compiler-self-host.md` / `docs/STATUS.md` / `docs/ROADMAP.md` / `docs/handoff-m22.md` / `docs/steps/README.md` | 同步本片结论 |

### 4.2 实测数据（本机：Windows x64 / GNU 1.98.0 / Wasmtime）

**S3 — 线性策略（唯一能到 1 MiB 的策略）**

| 输出字节 | wall (ms) | 相邻增长 | 最小 fuel 限额 |
|---|---|---|---|
| 4 KiB | 24 | 1.00× | 12,217 |
| 64 KiB | 22 | 0.76× | 754,153 |
| 128 KiB | 28 | 1.27× | 3,015,169 |
| 256 KiB | 29 | 1.04× | 6,029,825 |
| 512 KiB | 37 | 1.28× | 12,059,137 |
| **1 MiB** | **66** | **1.78×** | **24,117,785** |
| 2 MiB | 78 | 1.18× | 48,235,033 |
| **4 MiB** | **133** | **1.71×** | **96,469,529** |

⇒ **增长(256 KiB→1 MiB) = 1.8×–2.3×（多轮观测）⇒ tier = 可行**（D6 阈值 ≤6×）。**墙钟是机器相关的观测值**（同机不同轮次有抖动），因此本片**按区间记录**、且测量钉**不钉墙钟**，只钉结构事实与阈值余量（见 §4.3）。

**S1 / S2 — 二次方策略（撞 arena 上限）**

| 策略 | 最大成功输出 | 最小 fuel 限额（该点） | 首个失败点 |
|---|---|---|---|
| S1 增量 concat | **128 KiB** | 541,471,259 | 256 KiB ⇒ `arena-exhausted` |
| S2 分块 push_chunk | **256 KiB** | 1,085,117,073 | 384 KiB ⇒ `arena-exhausted` |

**同一个 256 KiB 输出，S2 比 S3 贵 180 倍燃料**（1,085,117,073 vs 6,029,825），wall 9 倍（268 ms vs 29 ms）⇒ **撞墙的是策略的二次方行为，不是规模**。

### 4.3 D6 三档判读

| 判据（计划 §3.3.4） | 实测 | 结论 |
|---|---|---|
| 1 MiB 输出在 (arena ≤ 2 GiB, fuel ≤ 800 B, wall ≤ 120 s) 内达成 | ✅ 66–78 ms（多轮观测），fuel 上界 24,117,785（限额的 **0.003%**），内存下限 = 声明值 | **达成** |
| 256 KiB→1 MiB wall 增长 ≤ 6× | ✅ **1.8×–2.3×**（多轮观测，线性策略） | **可行** |

⇒ **判读 = 可行**。**T-R3-a 止损不触发**，W2/W3 的顺序推进**继续**。

**关于墙钟的诚实标注**：wall time 是**机器相关**的观测值（同一台机器不同轮次也有抖动：增长比在 1.82×–2.28× 之间波动，1 MiB 点在 51–78 ms 之间波动）。⇒ 判读所依据的**稳定事实**是：①增长比**始终远低于** 6× 阈值（最大观测 2.28×，留有 2.6× 余量）；②fuel 上界比限额低**三个数量级**（24.1 M vs 800 B）；③**结构事实**（S1/S2 撞 arena、S3 不撞、内存下限 == 声明值）由钉**断言**固定，**不依赖**墙钟数值。测量钉因此**不钉墙钟**，只钉结构与阈值余量。

### 4.4 M5 目标结构折算（纯算术，锚定实测）

| 项 | 值 | 来源 |
|---|---|---|
| `A = R(S)` 组件总量 | **1,024,613 B** | STEP-0514 §2（Rust 后端生产输出） |
| 其中 code 段 / data 段 / type 段 | 1,003,250 / 16,487 / 3,685 B | 同上 |
| hex 文本表示（2 字符/字节） | **2,049,226 B** | 折算 |
| guest 侧需常驻（canonical IR JSON ≈1.3× + hex 缓冲） | **≈3,381,223 B** | 折算 |
| 相对 arena 2 GiB 的余量 | **≈2.14 GB** | 折算 |
| **相对 S3 实测 256 KiB 成功点的倍数** | **12.9×** | 实测 × 折算 |

⇒ 目标规模**只比「线性策略已实测成功的 256 KiB」大 12.9 倍**，而线性策略的实测增长曲线是 1.2–1.8×/档 ⇒ **目标规模本身远在可行区内**。

### 4.5.1 现有 guest 组装路径**就是**实测失败的 S1（把判读变成对现有代码的直接诊断）

guest 当前的 hex 编码路径（`selfhost/codegen.sico`）是**增量拼接**：

```
function cg_leb_hex(value: U64) returns Text:
  ...
  set out = sico.text.concat(out, cg_hex_byte(low))      # S1 形状
```

而 `cg_hex_byte` 自己又返回 `sico.text.concat(cg_hex_digit(high), cg_hex_digit(low))`（**每字节一次拼接**）⇒ 整条 hex 路径是**层层嵌套的增量拼接** = 本片实测的 **S1（O(n²)）** 形状。`cg_section_hex` = `sico.text.concat(id, sico.text.concat(siz, payload))` 同理。

⇒ **这不是「将来可能发生的问题」，而是对现有代码的直接诊断**：按现状，guest 在 **128 KiB** 输出量级就会耗尽 arena（实测），而目标 hex 缓冲是 **2,049,226 B**（≈ 2 MiB，是 128 KiB 的 **16 倍**）⇒ **W2 的 codegen 刀必须先把组装路径改成线性**，否则 W5（bundle 组件级全等）在规模上根本走不到。

### 4.6 自造缺陷 ①：**内存上限低于声明值 ⇒ 必然 `MemoryLimit`**

初版把 W1 的内存上限按计划字面写成 **2 GiB**（`2048 * 1024 * 1024`）⇒ 探针**全部**返回 `MemoryLimit`。**根因**：计划 §3.3.4 的「arena 2 GiB」指的是 **guest arena**（`canonical::ARENA_LIMIT`，编译期常量，活在 guest 线性内存**里面**），而 runner 的 `memory_bytes` 是**线性内存上限**，必须覆盖**声明值**（33,792 页 = **2,214,592,512 B** = 2112 MiB = arena 2048 MiB + 64 MiB 余量）⇒ 上限低于声明 ⇒ wasmtime 无法满足声明的最小值 ⇒ 实例化被拒（**STEP-0497 记录过的失败形态**）。

**修法**：分开两个概念 —— arena 上限用**实测**（guest 自己撞 `unreachable`），runner 内存上限用**声明值下限 + runner 默认余量**，并把「实测内存下限 == 声明值」写成断言（钉 #3）。

### 4.7 自造缺陷 ②：**S3 的大操作数写成源字面量 ⇒ 测成源契约**

S3 初版把两个大操作数写成源里的字符串字面量 ⇒ 编译被拒：`source contract error LineTooLong at bytes 453..1049045`。**修法**：操作数改从 **stdin** 进（`sico.bytes.utf8_decode(input.stdin)`），源里只剩一个固定小函数 ⇒ 测的才是发射规模。

### 4.8 自造缺陷 ③：**`sico.list.empty[Text]()` 不是合法调用目标**

S2 初版用 `sico.list.empty[Text]()` 造空 `List[Text]` ⇒ `E2031 unresolved call target sico.list.empty[Text]`。**根因**：`sico.list.empty` 的 bracket 单态**只声明了 `I64`/`U64`**（`crates/sico-semantics/src/lib.rs`：`"list.empty" => matches!(key, "I64" | "U64")`）。**修法**：空 `List[Text]` 改用 `sico.text.split_lines("")`（语义：`(Text) -> List[Text]`）。

### 4.9 自造缺陷 ④：**`Point` 未 `Clone` / 重复定义 `min_fuel`**

改测量结构时留下的两个编译错（`E0382 borrow of moved value`、`E0428 min_fuel defined multiple times`）⇒ 一次修净。

### 4.10 工具侧观察：**Runtime 没有 fuel 读数出口**（与 D13 一致）

计划 D13 已预登记：「若 Runtime ABI 没有 fuel 实际耗量读数出口 ⇒ 登记为**上界**（限额取值）而非读数」。本片**实测确认**：runner 只在 `store.get_fuel() == 0` 时判 `FuelExhausted`，没有任何把已耗燃料回传给调用方的出口 ⇒ 本片记的 `min_fuel` 就是**上界**，不是读数（**显式标注，不阻塞 GO**）。

### 4.11 自造缺陷 ⑤：**证据文件路径硬编码绝对机器路径 ⇒ CI 上会红**

探针初版把证据文件写到 `E:/github/sico-w2/target/tmp/step0516/...`（本机绝对路径）。**怎么发现的**：本机跑得通，但独立 CI 跑在 Linux + 不同工作目录 ⇒ 该路径不存在 ⇒ 测试会因写文件失败而红。**修法**：用 `CARGO_MANIFEST_DIR`（= `<repo>/runner/sico-runner`）**上溯两级**推导仓库根，再拼 `target/tmp/step0516` ⇒ 跨主机可移植。**这类硬编码在本仓是已知雷区**（历史 STEP 记录过多次同族问题），**测量片也必须守同样的纪律**：任何写盘路径都必须从环境推导，不得写死机器路径。


## 5. 诚实性

### 5.1 本片**只测量，不实现**

- guest 源码**零改动**；oracle 生产代码**零改动**；**不推进任何出口门**。
- 产出是**一个判读**（可行）+ **一条实施约束**（见 §5.2），**不是能力**。
- 证据等级：全部 `internal-fixture`；运行时主张只来自**真实 runner**（本机 Windows x64 GNU + Wasmtime）。
- 这些数字**不是**性能承诺，也**不是** runtime 支持主张 —— 它们是**一台机器**对**一组 fixture** 的观察。

### 5.2 判读**附带一条实施约束**（W2/W5 必须遵守）

**「字节组装策略必须是线性的」** —— 这条约束直接落进 W2 的族实现方式：

- 现有 guest codegen（`selfhost/codegen.sico`）用**增量拼接**（S1 形状）⇒ 在 1 MiB 量级**必然**撞 arena 上限（S1 实测 256 KiB 即耗尽）。
- 计划 D7 的方案优先级把「②组件输出改二进制通道」排在「①流式/分块发射」之前 —— 但本片实测表明：**分块（S2）本身也是二次方的**（chunk 表每次 push 重建）⇒ **只分块不够**，必须做到「总拷贝量线性」。
- ⇒ W2 的 codegen 刀**必须**按线性组装设计（例如：一次成型的大块分配 / 不重建的 chunk 表 / 二进制通道），**不得**沿用现有的增量拼接形状。

### 5.3 其余缺口（承自本线）

- **M2（通用 bytes→hex 编码吞吐）本片未测**：runtime **没有** bytes→hex 内建（`sico.*` 表里无 hex 族）⇒ 该项只能测「guest 自写 hex 循环」，属实现面 ⇒ 留给后续片。
- **M3（大对象共驻峰值）本片未测**：依赖 M2 与 canonical IR JSON 的物化方式。
- **M4（Rust 侧对照）本片未测**：STEP-0514 §2 已给出 `A = R(S)` 的生产输出字节数（1,024,613 B）；host 侧构建墙钟未重测。
- **整文件自举仍未关闭**（guest 不能整体降 `codegen.sico`：`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。
- **M22 整体仍为 NO-GO**（门 1/4/5/7 GO，门 2 PARTIAL，门 3/6 NO-GO）。

## 6. 验证记录（2026-10-23）

| 验证 | 口径 | 结果 |
|---|---|---|
| W1 测量钉 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test m22_scale_probe -- --exact the_one_megabyte_emission_scale_is_measured --test-threads=1`（`SICO_SELFHOST_COMPONENT_CACHE=0`） | ✅ **1 passed / 0 failed**（含 3 条结构化断言） |
| clippy | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --tests -- -D warnings` | ✅ 无警告 |
| rustfmt | `cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` | ✅ exit 0 |
| 证据文件 | `target/tmp/step0516/scale-measurement.txt` | ✅ 30 行实测记录（3 策略 × 阶梯） |
| 本机验证电池 | `target/tmp/step0507/battery.ps1`（11 步） | 见 §6.2 |
| 行尾审计 | `target/tmp/step0507/audit_lines.py` | 见 §6.2 |

### 6.1 固件与前沿结果

| 项 | 结果 |
|---|---|
| W1 测量钉 | **1 passed / 0 failed** |
| S3 线性策略 1 MiB | **66 ms** / fuel 上界 **24,117,785** / 内存下限 **2,214,592,512 B** |
| S3 线性策略 4 MiB | **133 ms** / fuel 上界 **96,469,529** |
| S1 增量策略 | 最大 **128 KiB**，256 KiB ⇒ `arena-exhausted` |
| S2 分块策略 | 最大 **256 KiB**，384 KiB ⇒ `arena-exhausted` |
| **D6 判读（256 KiB→1 MiB 增长 2.28×）** | **可行** |

### 6.2 电池结果（如实记录）

见 §6.4（电池在本片收尾时整轮重跑）。

### 6.3 未做的事

- **只覆盖三个组装策略**，不是穷举；
- **M2/M3/M4 未测**（见 §5.3）；
- **未提交、未推送**；**独立 CI 尚未运行**；
- **不推进任何出口门**，M22 仍为 NO-GO。

### 6.4 本机验证电池（11 步）

本轮电池 **11/11 全绿**（`target/tmp/step0507/battery.ps1`，日志 `target/tmp/step0507/battery.log`，2026-10-23 00:41–00:57）：

| 步 | 秒 | exit | 口径 |
|---|---|---|---|
| `fmt-runner` | 3 | 0 | runner workspace 的 `cargo fmt --all -- --check` |
| `ci-one-key` | 434 | 0 | 一键 CI 全 PASS ⇒ `CI GREEN`（含 runner 全量测试 ⇒ **固件 122 条 + 本片的 W1 测量钉在本步已跑过**） |
| `validate-0245` | 134 | 0 | 内含固件全量 + clippy |
| `validate-0262` | 144 | 0 | `STEP_0262_OK … canary=FORMATTER-CLOSED` |
| `probe-frontend-frontier` | 153 | 0 | 前端前沿 **23/23** ⇒ `FRONTIER_UNCHANGED`（本片未动 guest 前端） |
| `probe-ir-plan-frontier` | 25 | 0 | IR 计划层前沿 **8/8** ⇒ `PLAN_FRONTIER_UNCHANGED` |
| `validate-0124` | 1 | 0 | `STEP_0124_OK milestones=M14-M26` |
| `validate-0131` | 1 | 0 | `validate-step-0131: OK` |
| `validate-0156` | 0 | 0 | `validate-step-0156: OK` + UI renderer corpus **8/8** |
| `git-diff-check` | 0 | 0 | `git diff --check` 无输出 |
| `line-ending-audit` | 0 | 0 | `LINE_ENDING_AUDIT_GREEN`（**23** 个文件，含本片新增的探针与文档） |

⇒ `BATTERY_GREEN 11/11`。

**这一轮电池之前跑过一轮**（同为 11/11 全绿），但那一轮跑的是**可移植性修复之前**的探针（证据文件路径硬编码本机绝对路径，见 §4.11）⇒ 按纪律**整轮重跑**，上表是重跑那一轮的结果。
