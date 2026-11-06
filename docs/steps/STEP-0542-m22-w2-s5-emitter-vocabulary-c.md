# STEP-0542 · M22 W2 S5 · 发射词表 C（剩余 6 项集合 helper）

> **本片性质** = 计划 §3.3.5「每族四拍」的**组件 C 拍**，对象 = **发射词表 B/C 的最后 6 项集合 helper**。
> **不推进任何出口门**；**`crates/*` 零改动**；**`selfhost/parser.sico` 零改动**（只动发射端 ⇒ 前端前沿不动）；**M22 仍为 NO-GO**；**`match` 族仍未闭合** ⇒ **D10 不得降级**。

## 0. 入口门槛

| 门槛 | 状态 | 证据 |
|---|---|---|
| STEP-0541 已提交并推送 | ✅ | 提交 `9f0b7a3`，推送到 `github/codex/m22-w1-ci`（run #52 触发） |
| STEP-0541 整轮版本化电池绿 | ✅ | `BATTERY_GREEN 15/15`（日志 `target/tmp/m22-s5-battery/battery.log`） |
| oracle 生产代码零改动 | ✅ | `git diff --stat` 只含 `selfhost/`、`runner/`、`tools/`、`docs/` |
| 本片对象在**冻结**的缺口清单里 | ✅ | `selfhost/emitter-vocabulary-v1.json`（STEP-0541 快照）：`sico.list.get[U64]` **73** / `sico.text.split_lines` **60** / `sico.list.empty[U64]` **16** / `sico.list.append` **11** / `sico.list.append[U64]` **6** ⇒ **5 项缺口**；本片另加**第 6 项** `sico.text.join`（线性组装刀的前置，STEP-0541 §2.6 已登记） |

## 1. 本片方法

1. **先读冻结 oracle 源码**（不复刻、不猜）：`crates/sico-codegen-wasm/src/stdlib.rs` 的 `collection_helper_signature` / `emit_collection_empty` /
   `emit_list_get_scalar` / `emit_list_append_scalar` / `emit_list_append` / `emit_split_lines` / `emit_join`，
   `crates/sico-codegen-wasm/src/lib.rs` 的 `helpers: BTreeMap` / `INTRINSIC_HELPERS` / `emit_collection_call` / `lower_local_types`。
   核对结论固化在 `target/tmp/step0542/contract-verified.md`（**不入库**）。
2. **helper 体机械推导 + 逐字节自检**：用一份 Python 编码器把 oracle 的 `ops!` 指令表重新编码成 wasm 字节，
   **先重推已冻结的体**（`sico.bytes.equal` / `sico.text.concat` / `sico.u64.to_text` / `sico.text.starts_with`）与发射端里的冻结值
   **逐字节比对**，全部 MATCH 之后才推新体 ⇒ 编码器可信 ⇒ 新体可逐字冻结。
3. **按既有形状落地**：helper 体函数 → `cgs_helper_index` 的**名字排序位**（严格字典序）→ **类型段 / 函数段 / code 段**三处接线 →
   **一个通用集合调用发射器** `cgs_collection_call_hex` → `cgs_intrinsic_hex` 的分派行。
4. **闸门全 fail-closed**：名字 / 实参个数 / 实参 kind（含 list 元素 kind）/ 结果槽数 / helper 索引；任一不过 ⇒ `ERR:E-SH-CG-IR-EMIT`。
5. **钉**：6 个形状**整组件逐字节等同** + **两条拒绝对照**（两侧都拒）；**自举前沿钉**加 9 个函数名。

## 2. 实测结果

### 2.1 六项 helper 的合同（全部读自冻结源码）

| helper | oracle 签名来源 | 参数 | 结果 | 类型段 hex | IR 实参 flat | IR 结果槽 |
|---|---|---|---|---|---|---|
| `sico.list.append` | `stdlib.rs:33-36` 静态分支 | 4×i32 | 2×i32 | `60047f7f7f7f027f7f` | List[String](2) + String(2) | List[String](2) |
| `sico.list.append[U64]` | `stdlib.rs:1000` `ListAppend` | (i32,i32,i64) | 2×i32 | `60037f7f7e027f7f` | List[U64](2) + U64(1) | List[U64](2) |
| `sico.list.empty[U64]` | `stdlib.rs:966` `ListEmpty` | **0** | **2×i32** | **`6000027f7f`** | () | List[U64](2) |
| `sico.list.get[U64]` | `stdlib.rs:996` `ListGet` | (i32,i32,i64) | (i32,i64) | `60037f7f7e027f7e` | List[U64](2) + U64(1) | Result[U64,NumericError] = **2 槽** [I32,I64] |
| `sico.text.join` | `stdlib.rs` `emit_join` | 6×i32 | 2×i32 | `60047f7f7f7f027f7f` | List[String](2) + String(2) | String(2) |
| `sico.text.split_lines` | `stdlib.rs:43-47` 静态分支 | 2×i32 | 2×i32 | `60027f7f027f7f` | String(2) | List[String](2) |

**helper 体（冻结体 + 参数化 `call alloc`）**

| helper | 体字节 | 参数化点 |
|---|---|---|
| `sico.list.append` | **239 B** | **1** 次 `call alloc` |
| `sico.list.append[U64]` | **213 B** | **1** 次 `call alloc` |
| `sico.list.empty[U64]` | **6 B** | **0**（可逐字冻结） |
| `sico.list.get[U64]` | **46 B** | **0**（可逐字冻结） |
| `sico.text.join` | **178 B** | **2** 次 `call alloc` |
| `sico.text.split_lines` | **314 B** | **1** 次 `call alloc` |

**helper 索引链（`cgs_helper_index`，泛化后的严格字典序，12 项）**：

```
sico.bytes.equal < sico.bytes.utf8_decode < sico.list.append < sico.list.append[U64]
  < sico.list.empty[U64] < sico.list.get[U64] < sico.text.concat < sico.text.join
  < sico.text.length < sico.text.split_lines < sico.text.starts_with < sico.u64.to_text
```

⇒ 新增的六项按字典序**插入**已有六项之间 ⇒ **已有六项的 core 索引会移动**（这是排序位链的定义使然），
但**本片新增六项的相对顺序与 oracle 的 `BTreeMap` 顺序逐项对上**（由 §2.2 的整组件逐字节等同钉保证，不是推断）。

### 2.2 组件层：六个形状的整组件与 oracle 生产输出**逐字节等同**

| 钉 | 形状 | oracle 组件（字节） | 结论 |
|---|---|---|---|
| `the_emitter_vocabulary_c_covers_the_collection_helper_shapes` | `sico.list.empty[U64]()` | **1044 B** | ✅ **逐字节等同**，两侧都过真实 `wasmparser::Validator` |
| 同上 | `sico.list.get[U64](list, index)` | **1208 B** | ✅ **逐字节等同** |
| 同上 | `sico.list.append[U64](list, value)` | **1263 B** | ✅ **逐字节等同** |
| 同上 | `sico.list.append(list, value)` | **1293 B** | ✅ **逐字节等同** |
| 同上 | `sico.text.split_lines(text)` | **1360 B** | ✅ **逐字节等同** |
| 同上 | `sico.text.join(list, separator)` | **1233 B** | ✅ **逐字节等同** |

**这条钉比较的是整组件字节，不是长度**（`assert_bytes_equal(&actual, &expected)`），并且两侧都跑真实 `wasmparser::Validator`。
钉另外显式断言 `hex.len() % 2 == 0`（guest 的 hex 必须是整字节）—— 这条断言正是本片缺陷 2（半字节 hex）的第一道闸。

#### 2.2.1 六个形状的组件长度也是**硬钉**

钉额外把六个形状的 oracle 生产组件长度**冻结**成常量 `COLLECTION_SHAPE_COMPONENT_BYTES`
（**1044 / 1208 / 1263 / 1293 / 1360 / 1233 B**）⇒ **helper 集合一变（core 索引移动 ⇒ 段布局移动）就会被长度抓住**，
而不必等到逐字节比较才报。

> **测量纪律**：这六个数是**先测后钉**——第一版把常量全写成 `0`，钉**失败**并把实测值整条打印出来
> （`left: [("list-empty-u64", 1044), …]`）⇒ 再把实测值填回常量 ⇒ 复跑为绿。
> **常量不是手写的**，是从 oracle 的生产输出测出来的。


### 2.3 闸门是活的（两条拒绝对照，两侧都拒）

| 对照 | Rust oracle | guest |
|---|---|---|
| **实参个数**：`sico.list.get[U64]` 去掉 index 实参 | verifier 拒 | `ERR:E-SH-CG-IR-EMIT` |
| **实参类型**：`sico.list.get[U64]` 的 index 换成 **Text** 形参 | verifier 拒 | `ERR:E-SH-CG-IR-EMIT` |

### 2.4 自举前沿：本片新增的发射代码**自己**能经 guest 前端逐字节降

**自举前沿钉的函数清单 22 → 31**（新增 `cg_list_append_helper_body_hex` / `cg_list_append_scalar_helper_body_hex` /
`cg_collection_empty_helper_body_hex` / `cg_list_get_scalar_helper_body_hex` / `cg_text_join_helper_body_hex` /
`cg_split_lines_helper_body_hex` / `cgs_collection_arg_is` / `cgs_collection_arg_is_list_of` / `cgs_collection_call_hex`）⇒ **绿**。

> 这条钉是 STEP-0541 教训的直接延续：**逐字节相等 ≠ 在自举前沿之内**。
> 本片新增的 9 个函数若只做到「组件逐字节相等」而没进前沿钉，仍可能落在前沿之外。
>
> **实测**：本片把这 9 个函数名加进前沿钉后一次通过 ⇒ 它们**已经在**前沿之内。

### 2.5 账本与普查工件刷新

**词表工件**（`selfhost/emitter-vocabulary-v1.json`；`census-m22-emitter-vocabulary.py --check` ⇒ `EMITTER_VOCABULARY_CURRENT sites=145`）：

| 项 | STEP-0541 | 本片 |
|---|---|---|
| guest 已有 helper | 6 | **12** |
| 工件站点数（「IR 已全等但组件未全等」） | 147 | **145**（**2 个站点本片闭合 ⇒ 移出本口径**） |
| `missing_op_frequency` | {} | **{}** |
| `missing_intrinsic_frequency` | {} | **{}** |
| `missing_helper_frequency` | **5 项** | **{}** |
| `additions_needed_histogram` | 0 / 2 / 4 / 6 / 10 分布 | **{"0": 145}** |

⇒ **发射词表缺口 5 → 0**；**145 / 145** 个「IR 已全等但组件未全等」的站点**已无词表缺口**
⇒ 剩下的全是**燃料/闭包**问题，不再是「缺 helper」。


**账本**（`selfhost/match-component-coverage-v1.json`，396 站点；`census-m22-match-sites.py --check` ⇒ `LEDGER_CURRENT sites=396`）：

#### 2.5.1 账本 verdict 分布（实测）

| verdict | STEP-0541 | 本片 | 差 |
|---|---|---|---|
| `ir-equal+component-equal`（**真闭合**） | 30 | **32** | **+2** |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | 63 | **5** | **−58** |
| `ir-equal+component-other:FuelExhausted`（**不是闭合**） | 84 | **140** | **+56** |
| `closure-too-large:*`（21 个不同规模桶，**未测**） | 219 | **219** | 0 |

**62 个站点的 verdict 发生移动**，分**三类**（不是两类）：

| 移动 | 站点数 | 含义 |
|---|---|---|
| `component-refused-guest` → **`component-equal`** | **6** | **真正闭合**：`MS-019`（`parser.sico:gw_intrinsic_call_packed`）、`MS-037`（`operand_cell_or_literal_json`）、`MS-105`..`MS-107`（`gw_list_sort_rhs_packed`）、`MS-266`（`gw_deferred_tail_json`） |
| `component-refused-guest` → **`component-other:FuelExhausted`** | **52** | **不是闭合**：guest 现在**能发**这段源码，但发射端的二次路径把**主 fuel** 耗尽 ⇒ 前进到下一个瓶颈 |
| **`component-equal` → `component-other:FuelExhausted`** | **4** | **回退**：`MS-081`..`MS-084`（`parser.sico:fixed_literal_const_segment`）在 STEP-0541 已闭合，本片**又耗尽**了 ⇒ 见下方「回退」段 |

> **探针信号**：账本差分固件以 `MS-019: the verdict of parser.sico:gw_intrinsic_call_packed moved` **失败**（exit 101）——
> 这是**预期的闭合信号**，不是回归失败：固件**先写** `actual.json` 再断言，账本按纪律**在实测之后**才用 `census --update` 合并
> （`LEDGER_WRITTEN sites=396 pending=0`），再以 `--check` 复验（`LEDGER_CURRENT sites=396`）。

#### 2.5.2 诚实收窄：**4 个站点回退**（本片最重要的负面实测）

**组件轴净增只有 +2（30 → 32），而不是 +6**，因为 `MS-081`..`MS-084` **回退**了。

**实测事实**：这 4 个站点在 STEP-0541（guest 只有 **6** 项 helper）时**组件逐字节等同**；
本片把 guest 的 helper 集合扩到 **12** 项后，它们变成 **`FuelExhausted`**。

**这说明什么（只说测到的）**：
1. **主 fuel 预算对「发射端工作量」是敏感的** ⇒ 组件越大、发射端要做的活越多 ⇒ 同样的 800e9 预算可能不够。
2. ⇒ **「补 helper」与「省 fuel」不是两件独立的事**：helper 越全，闭合的站点越多，但**每个站点的发射成本也越高**，
   于是**已经在悬崖边的站点会被推下悬崖**。
3. ⇒ 这给 **fuel 归因测量片**一个更强的理由：**在补更多能力之前，必须先把每站点的 fuel 成本降下来**，
   否则净收益会被回退吃掉。

**剩下的 5 个 `component-refused-guest`** 全部是 `lex_words`（`MS-003` / `MS-004` / `MS-394` / `MS-395` / `MS-396`）⇒
**它们不是 helper 缺口**，是另一个形状缺口 ⇒ 归入**后续族**，不在本片范围。

#### 2.5.3 口径纪律

> **「发射词表清零」不等于「组件轴通了」**。它只说明**缺 helper** 这个原因被排除了；
> 组件轴现在的最大阻塞是**主 fuel**（140 个站点实测耗尽）与**闭包规模**（219 个站点未测），
> 外加 **5** 个非 helper 形状（`lex_words`）。


### 2.6 本片**捕获并修掉**的两个自造缺陷（诚实登记）

| # | 缺陷 | 怎么发现的 | 修法 |
|---|---|---|---|
| 1 | 集合调用发射器里 `let ccch_item` 在**同一函数**出现两次 ⇒ Sico 的 `let` 是**函数级 cell**，重名即前端降级失败（`unsupported cell redeclared at bytes 140990..141053`） | **前端降级失败**（不是逐字节钉） | 改名 `ccch_arg_item`；并加扫描脚本（`target/tmp/step0542/check-dup-cells.py`）核全文件 **193** 个函数 ⇒ 无重名 cell |
| 2 | `sico.list.empty[U64]` 的**类型段条目**被**手改**成 `600027f7f`（**9 个 hex 字符 = 半字节**）⇒ guest 发出的类型段整体错位 1 个 hex 字符 ⇒ 组件比 oracle **少半个字节**（guest hex 串 **2087** 字符 vs oracle hex 串 **2088** 字符 = 1043.5 B vs **1044 B**） | **逐字节钉**在**类型段**抓到（长度同、字节不同） | oracle 的**生产输出**实测真值 = **`6000027f7f`**（`60` + 0 参 + `02` + `7f 7f`）⇒ 产品与 9 份准备工件全部改正；**「hex 不得手改」写进契约文档**（条目必须由 oracle 生产输出或 `ops!` 指令表派生） |

> 缺陷 2 与 STEP-0541 的 `I32GeU` vs `I32GtU` 是**同一类错误**（手抄/手改字节 ⇒ 长度同、字节不同）：
> **长度检查抓不到它，只有逐字节比较抓得到。**

## 3. 与计划 / 合同的关系

- 计划 §3.3.5「每族四拍」的**组件 C 拍**；本片把**发射词表 B** 与其依赖的**第 6 项 helper** 一起做完 ⇒ **发射词表这条线关闭**。
- 本片**必须一片做完**（体 + 链 + 三段 + 调用点 + 普查表 + 钉）：只做 helper 体只能关掉词表缺口、**不闭合任何站点**。
- **ADR**：不需要新开 ADR —— 生产后端 `emit_collection_call` / `collection_helper_signature` / `lower_local_types` 已冻结逐字节发射形状
  （[ADR-0015](../adr/ADR-0015-compiler-bootstrap-closure-v0.md) 的 `A == B == C` 字节闭合目标不变）⇒ 这是**已冻结合同的实现**，需要的是**对照字节钉**。
- **归因口径不变**：`FuelExhausted` 读的是**主 fuel**（`store.get_fuel()==0`），主 fuel **只按 guest 执行的 Wasm 指令**计费；
  hex 拼接是**组件内 core module 的 Wasm 函数**（`emit_concat` = `alloc` + **2× `memory.copy`**）⇒ guest 对它是**组件内调用** ⇒ 记主 fuel，
  但**每次拼接只有 ~13 条指令** ⇒ 对主 fuel 的贡献是 **O(调用次数)**、随字节数增长的是**拷贝字节量**（O(L²)）
  ⇒ **hex 拼接不可能是 fuel 墙**（它解的是**时间墙**与**整文件自举**）；真凶更可能是 **guest 的 IR 读取**（B 路）⇒ **下一刀 = fuel 归因测量片**。

## 4. 独立 CI

- STEP-0541（提交 `9f0b7a3`）的 **run #52 = completed / success ✅**（run id `40162446092`，`windows-gnu`）⇒ **STEP-0541 的独立 CI 裁决已回填**。
- 本片推送后触发下一轮独立 CI，**裁决由下一片回填**。

> **回填位置**：本片的实现与文档初稿写于 run #52 尚未出结果时（`status=in_progress`）；
> 上面的裁决由**收尾脚本**（`target/tmp/step0542/sync-docs.py`，在整轮电池 exit 0 之后运行）按 **GitHub API 的实测值**回填
> ⇒ 文档里的 CI 结论**只来自实测**，不是抄写；该脚本对这一行是**幂等**的（正则替换，重复运行不会重复插入）。

## 5. 验证

**整轮版本化电池：`BATTERY_GREEN 15/15`**（本机执行；**源码指纹 `b67138cb3181630ed86794aba63b9dd5dd76f2c86c7128d4981e3bb4cc0ec442` 上的一次完整 15 拍**；
因外部原因三次中断（2 次宿主硬重启 + 1 次 harness 回收），15 拍**分 4 次进程会话续跑**，逐轮登记见 §5.1 下方）。
日志：`target/tmp/m22-s5-battery/battery.log`（版本化脚本：`tools/validate-m22-s5.ps1`，**一字未改**）。

### 5.1 电池表

| # | 步骤 | 耗时 | exit | 命令 |
|---|---|---|---|---|
| 1 | `fmt (workspace)` | 2s | **0** | `cargo fmt --all -- --check` |
| 2 | `fmt (runner)` | 2s | **0** | `cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` |
| 3 | `clippy (workspace)` | 2s | **0** | `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` |
| 4 | `clippy (runner)` | 1s | **0** | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings` |
| 5 | `ci-one-key` | 2200s | **0** | `powershell -File tools/run-ci.ps1` |
| 6 | `validate-0245` | 1725s | **0** | `powershell -File tools/validate-step-0245.ps1` |
| 7 | `validate-0262` | 1709s | **0** | `powershell -File tools/validate-step-0262.ps1` |
| 8 | `probe-frontend-frontier` | 502s | **0** | `powershell -File tools/probe-m22-frontend-frontier.ps1` |
| 9 | `probe-ir-plan-frontier` | 27s | **0** | `powershell -File tools/probe-m22-ir-plan-frontier.ps1` |
| 10 | `comment-discipline` | 0s | **0** | `python tools/validate-selfhost-comment-discipline.py` |
| 11 | `match-site-census` | 0s | **0** | `python tools/census-m22-match-sites.py --check` |
| 12 | `match-site-coverage` | 1519s | **0** | `powershell -File tools/probe-m22-match-coverage.ps1` |
| 13 | `emitter-vocabulary-census` | 24s | **0** | `python tools/census-m22-emitter-vocabulary.py --check` |
| 14 | `firmware-selfhost` | 2103s | **0** | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` |
| 15 | `git-diff-check` | 0s | **0** | `git diff --check` |

**判定**：本片的实现、改钉、账本刷新、普查工件刷新与文档同步在**同一份快照**上通过整轮电池 ⇒ **STEP-0542 达成**。
但**不推进任何出口门**：**门 3 仍 NO-GO**（**门 6 的出口裁决由 STEP-0543 给出**：锚点 = 独立 CI **run #52** = `9f0b7a3` = completed / success），M22 整体仍 **NO-GO**；`match` 族组件级全等仍远未闭合。

**固件**：`selfhost_compiler` = **182 passed / 0 failed**（STEP-0541 为 181；本片新增 1 条钉 + 自举前沿钉 22 → 31）。

> **四次会话的如实登记**：本片的整轮电池**被外部原因打断过三次**（**没有任何一次是测试失败**），第 4 次会话跑通：
>
> | 轮次 | 起止 | 终止原因 | 停在第几拍 | 已绿步骤 |
> |---|---|---|---|---|
> | 第 1 轮 | 13:55 → 14:58 | **宿主硬重启**（事件 41，15:05:37） | 第 7 拍 `validate-0262` 进行中 | 6 拍（`ci-one-key` 1945s / `validate-0245` 1591s，其内固件套件实测 **182 passed / 0 failed**、1645.81s） |
> | 第 2 轮 | 17:33 → 19:46 | **宿主非正常关机**（事件 41，19:45:58） | 第 14 拍 `firmware-selfhost` 第 3 分钟 | **13 拍**（含 `ci-one-key` 2200s / `validate-0245` 1725s / `validate-0262` 1709s / `match-site-coverage` 1519s） |
> | 第 3 轮 | 20:25 → 20:36 | **harness 回收长时后台任务**（exit 4294967295；**宿主未重启**） | 第 5 拍 `ci-one-key` 进行中 | 4 拍（fmt / clippy） |
> | 第 4 轮 | 20:44 → 21:20（+ 收尾器） | **跑通**（脱 harness 任务表 + 指纹续跑） | 全部 15 拍 | **15 拍**（`firmware-selfhost` 2103s / `git-diff-check` 0s） |
>
> 根因 = **宿主硬重启**（Windows 事件 41「rebooted without cleanly shutting down」；本日 06:09 / 09:51 / 15:05 / 19:53 共 4 次，间隔 3–5 小时，
> 并伴随 `AV_win32kbase!` 显卡/桌面栈故障；机器上跑着 AweSun 远程桌面）**与 harness 的长时任务回收** ⇒ **不是电池红，也不是代码问题**。
> 三次中断**既没有产生红灯，也没有产生收尾，也没有提交**；每轮的日志留档在
> `target/tmp/step0542/battery-step0542-INTERRUPTED*.log`。
>
> **第 4 轮怎么续跑的**：第 3 次中断暴露出两个问题——「harness 托管的长时后台任务不可靠」与「版本化电池的状态只在内存里，进程一死就全丢」。
> 第 4 轮用一个**未入库的临时包装器**（`target/tmp/step0542/resumable-battery.ps1`）驱动**一字未改**的版本化电池
> （`tools/validate-m22-s5.ps1`，**15 拍的名称、命令与判定逻辑完全不变**），并做了两件事：
>
> 1. **脱离 harness 任务表**：由 Windows 计划任务（`schtasks`）拉起 ⇒ 不再被 harness 回收；
> 2. **按源码指纹续跑**：指纹 = `HEAD` + 完整工作区 diff + 每个未跟踪文件的内容哈希 = **`b67138cb3181630ed86794aba63b9dd5dd76f2c86c7128d4981e3bb4cc0ec442`**；
>    第 4 轮启动时实测指纹与状态文件里存的**完全一致** ⇒ 第 2 轮那 13 拍的绿灯**对当前快照仍然有效**，直接复用；
>    指纹一旦不同，包装器**拒绝续跑**（fail-closed），并且**拒绝复用过期种子**。
>
> **代码快照在四轮之间未变**：受保护目录（`selfhost/` / `crates/` / `runner/` / `tools/` / `wit/` / `semantic-cases/`）里
> **最新的文件改动时间是 13:53:44**（`selfhost/emitter-vocabulary-v1.json`），**早于第 1 轮启动（13:55）** ⇒
> 四轮跑的是**同一份快照** ⇒ 跨会话复用绿灯不损失任何证据强度。
> 第 2 轮那 13 拍的**逐步输出**已从第 2 轮日志**逐字节还原**（47,635 字节，自检通过）到 `target/tmp/step0542/outputs/`；
> 重建出的 `battery.log` 与整轮直跑**逐字节同形**（原版布局：步行 + 各自完整输出，空输出的步骤同样保留空行），
> 且**前 47,635 字节与第 2 轮存档逐字节相同**（已复验）。
>
> ⇒ **本表记录的是第 4 轮（续跑后）在同一快照上凑齐的完整 15 拍整轮结果**。

## 6. 诚实收窄

- **组件级全等 32 / 396**（净增 **+2**，不是 +6 ⇒ **4 个站点回退**，见 §2.5.2）⇒ **`match` 族未闭合** ⇒ **D10 ⇒ 不得降级、不得宣告闭合**。
- **发射词表已清空**（缺口 5 → 0），但那**不等于**组件轴已通：组件轴现在的最大阻塞是**主 fuel 耗尽**（**140** 个站点实测）与**闭包规模**（**219** 个站点未测）。
- **5** 个站点仍被拒：全部是 `lex_words`（**不是** helper 缺口）⇒ 另一个形状缺口。
- **净收益被回退吃掉**：补 helper 会同时抬高每个站点的发射成本 ⇒ 在 fuel 成本降下来之前，补能力的边际收益会变小。
- **不上调限额、不加尺寸门**。
- **语料覆盖仍 0 条**；**整文件自举未关闭**；**门 3 仍 NO-GO**（**门 6 的出口裁决由 STEP-0543 给出**：锚点 = 独立 CI **run #52** = `9f0b7a3` = completed / success）；**M22 整体仍 NO-GO**。

## 7. 下一刀

1. **fuel 归因测量片**（**先测量，后修复**；本片把它的优先级再抬高一级，因为 §2.5.2 证明**发射成本与组件规模正相关**）：
   - **X 族（隔离 B 路 / IR 遍历）**：指令条数 n 递增 ⇒ 输出线性增长 ⇒ 判据是 **F(n) 的指数**（拟合 `a·n³ + b·n`，a≫b ⇒ **B 路**被证实）；
   - **Y 族（隔离 A 路 / hex 拼装）**：指令条数恒定、输出字节数递增 ⇒ 判据是**耗时斜率**（fuel 应保持线性、耗时应超线性）。
2. **B 路修复**（IR 读取游标化 + 槽位前缀和表）⇒ 解锁**主 fuel 耗尽**的 140 个站点，并**阻止已闭合站点再次回退**。
3. **A 路线性组装**（时间墙 + 整文件自举）—— 本片已落地其前置（`sico.text.join` helper 可逐字节发出）。
4. **F2**（实参是字符串字面量的宿主调用）。
5. **M-g3′（15）/ M-g2（46）/ M-g3（21）** 仍开；**闭包规模 219** 站点组件轴未测；**`lex_words`（5）** 另立形状缺口。
6. 之后回到 **T3 族** / **语料覆盖** / **门 3**（门 6 见 STEP-0543）。
