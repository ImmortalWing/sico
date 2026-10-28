# STEP-0550 m22-w2 P2 **批量复算片**：夹具上限 64 fn / 256 KiB → 256 fn / 1 MiB，396 站点首次**全部开测** ⇒ 回绿 **0 / 364 = 0%** ⇒ **D2 = 结构性**（墙仍是 B 路 IR 重扫）

| 项 | 值 |
|---|---|
| 片型 | P2 测量 + 夹具整改：`runner/sico-runner/tests/selfhost_compiler.rs`（上限参数化、新增探针、新增“全部站点已测量”钉、报告序列化修复）、`selfhost/match-component-coverage-v1.json`（396 条 verdict 刷新）、`tools/probe-m22-closure-bounds.py`（预检，入 `tools/`）；`selfhost/*.sico`、`crates/`、`wit/` **零改动** |
| 决策门 | **D2 = 结构性**：相对 P1 前的 364 个非全等站点，回绿 **0 条（0%）**，低于 25% 阈值 ⇒ 残差由 B 路（逐指令 IR 重扫）主导，不是形状覆盖或装配策略 |
| 不翻门 | 门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用 —— 全部**维持**；组件轴仍 **32 / 396** 全等 |
| 证据等级 | `internal-fixture`：本机 2026-10-28，Windows x64，debug 测试构建，`sico_runner` 限额不变（fuel 800e9 / timeout 120 s / 内存 2560 MiB / hostcall 8 GiB） |

## 1. 要回答的问题（D2）

STEP-0549 把随程序规模升维的四个装配折叠改成「分片 `list.append` + 边界一次 `join`」后，A 墙（输出字节自拼装）已有实测线性解法，但 396 站点账本**没有复算**。P2 的问题是：

> 把夹具的**闭包上限**从 64 fn / 262,144 B 放开后，原先被跳过的 219 个站点（全在 `parser.sico`）到底能不能回绿？残差是形状问题，还是同一条 B 墙？

这条问题不先答，继续按形状补丁推进就是在赌 219 个站点里有可回收的量。

## 2. 上限不是随手定的：先做闭包预检

新增 `tools/probe-m22-closure-bounds.py`，逐字复刻固件的 `match_sico_functions` / `match_sico_callees` / `match_sico_closure`，只读冻结 S 测**完整闭包**（账本里 `closure-too-large:65fn/…` 是撞上限**当时**的读数，是下界不是真值）。

| 量 | 实测 |
|---|---|
| 站点 / 组 | 396 / **95** 个 `(path, function)` |
| 旧上限（64 fn / 256 KiB）下被跳过 | **25 组 / 219 站点** |
| 最大真实闭包 | **164 fn / 1,027,015 B**（`parser.sico:scalar_ir`） |
| 新上限（256 fn / 1 MiB）下被跳过 | **0 组 / 0 站点** |

**计划的 128 fn 不够用**：`scalar_ir` 是 164 fn，128 fn 仍会把它跳过。本片取 **256 fn / 1,048,576 B**，覆盖最大闭包并留 21,561 B 余量；不是追认失败后的上调 —— 上限只是夹具自设值，fuel / timeout / 内存 / hostcall 一律未动。

## 3. 方法

- `MATCH_CLOSURE_FUNCTION_LIMIT = 256`、`MATCH_CLOSURE_BYTE_LIMIT = 1_048_576`。
- 上限成为参数：`match_sico_closure` / `match_compare_site` 接收 `(function_limit, byte_limit)`；**冻结账本测试永远用冻结上限**，环境变量 `SICO_M22_MATCH_CLOSURE_FUNCTION_LIMIT` / `SICO_M22_MATCH_CLOSURE_BYTE_LIMIT` 只影响显式运行的探针。
- 新增 `#[ignore]` 探针 `sico_compiler_probes_the_match_site_closure_bound`：按 95 组逐组测量，每测完一组就把报告原子落盘（`actual-probe.json`），中断不丢已测组；探针不写冻结账本、不断言冻结表。
- 测量路径与冻结账本测试完全同一条（`match_compare_site`），差别只有上限参数。

```
cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml \
    --test selfhost_compiler -- --ignored --nocapture --exact \
    sico_compiler_probes_the_match_site_closure_bound
```

- 实测 **6,259.78 s**（约 1 h 44 min）跑完 95 组；原始日志 `target/tmp/step0550/probe.log`。

## 4. 结果

### 4.1 组级直方图（95 组）

| verdict | P2 前 | P2 后 | 变化 |
|---|---|---|---|
| `ir-equal+component-equal` | 29 | **29** | ±0 |
| `ir-equal+component-other:FuelExhausted` | 39 | **63** | **+24**（全部来自原 `closure-too-large`） |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | 2 | **2** | ±0 |
| `closure-too-large`（夹具跳过，未测） | 25 | **0** | −25 |
| `guest-refused:Trap("malformed result: stdout exceeds 8 MiB")` | 0 | **1** | +1（`scalar_ir`） |

### 4.2 站点级直方图（396 站点）

| verdict | P2 前 | P2 后 |
|---|---|---|
| `ir-equal+component-equal` | 32 | **32** |
| `ir-equal+component-refused-guest:unsupported codegen source shape` | 5 | **5** |
| `ir-equal+component-other:FuelExhausted` | 140 | **335** |
| `guest-refused:Trap("malformed result: stdout exceeds 8 MiB")` | 0 | **24** |
| `closure-too-large:*` | 219 | **0** |

逐站转移（396 条逐条比较，不是抽样）：

| 转移 | 站点数 |
|---|---|
| `component-equal → component-equal` | 32 |
| `refused-guest → refused-guest` | 5 |
| `FuelExhausted → FuelExhausted` | 140 |
| `closure-too-large → FuelExhausted` | **195** |
| `closure-too-large → 8 MiB trap` | **24** |

**回绿 = 0 / 364 = 0%。**

### 4.3 三个具名残差

1. **335 站点 / 63 组 = `FuelExhausted`（800e9 主预算）**：与 STEP-0544 的归因一致 —— 成本在 B 路（`cgs_value_slot` 对参数/块/指令的从头扫描 + `cgj_array_item` 对数组的从头遍历），每条指令的每个操作数都要重扫，二次项占 86%–97%。P1 只关 A 墙，**对 B 墙零影响**，所以 195 个新开测站点直接落进同一堵墙。
2. **24 站点 / 1 组 = `guest-refused:Trap("malformed result: stdout exceeds 8 MiB")`**：`scalar_ir` 的 164 函数闭包在 canonical IR 轴上的 stdout 超过 runner 的 8 MiB 结果上限（`runner/sico-runner/src/lib.rs:3599`）。这是**测量传输上限**，不是 fuel、不是形状；本片不擅自上调，单独登记。
3. **5 站点 / 2 组 = `unsupported codegen source shape`**：P2 前既有，未变。

### 4.4 口径副作用：发射词表 artifact 145 → 340 站点，缺口不是 0

`tools/census-m22-emitter-vocabulary.py` 的闭包规则同样钉在固件上限上；上限放开后，**195 个 `ir-equal` 站点进入词表普查**，artifact 由 **145 → 340 站点**重生成（`guest_emitter.source_sha256` 不变，仍是 `6be9bf98…`）：

| additions_needed | 站点数 |
|---|---|
| 0 | 333 |
| 2 | **7** |

- 缺口只有一项：**`sico.list.length[U64]`**（intrinsic 与 helper 同名），出现在 **7 个 `parser.sico:general_while_function_ir` 站点**（MS-318…MS-324）。
- 因此「发射词表缺口 5 → 0（STEP-0542）」的 **0 只对当时的 145 站点口径成立**；340 站点口径下是 **7 个站点 / 1 个名字**。这 7 个站点同时是 `FuelExhausted`，词表缺口暂时不改变它们的 verdict，但 P3 必须记账，不能把 340 口径写成 0。

## 5. D2 判读

- 阈值是「回绿 < 25% ⇒ 结构性」。实测 **0%**，且 195 个新开测站点**全部**落进 `FuelExhausted`、一个都没有变成全等 ⇒ **结构性成立**。
- 结构性指向的唯一主路径是 **B 路修复**：把「值 id → 槽号」的逐次扫描换成每组一次建索引（B2），而不是继续开形状或继续改装配。
- 三个选项里本片选「**继续**」，但把「继续」限定为 B2；「合同变更」与「R3 止损」均不满足触发条件（没有连续零增长实现片的新计数，也没有合同缺口）。
- 8 MiB 残差与 B2 正交：B2 修好后 `scalar_ir` 仍会在 IR stdout 上撞 8 MiB，需要单独决策（先测 IR 文本真实大小，再决定是分子输出还是别的校验口径；**不以上调限额追认**）。

## 6. 固化与修复

- 账本由 `tools/census-m22-match-sites.py --update` 从实测报告合并，396 条 `verdict` / `source_sha256` / `ir_sha256` / `component_sha256` 全量刷新；`--check` 复验 `LEDGER_CURRENT sites=396`。
- 新增常驻钉 `every_match_site_is_measured_on_the_component_axis`：396 条不得再有 `closure-too-large`、不得有空 `source_sha256`、不得有 `pending` —— 把「跳过」从账本口径里永久移除。
- **修掉一个会静默丢证据的序列化缺陷**：原报告写入用 `format!` 直接把 verdict 插进 JSON 字符串，`guest-refused:Trap("malformed result: …")` 里的引号会让整份报告不是合法 JSON，census 会拒绝全部已测组。改为 `serde_json::json!` + `to_string_pretty`。该缺陷由本片的 8 MiB 残差首次触发。
- `tools/probe-m22-closure-bounds.py` 入版本库，默认上限同步为 256 fn / 1 MiB；`--limits 64 262144` 可复现 P2 前的跳过集。
- `tools/census-m22-emitter-vocabulary.py` 的闭包上限同步为 256 fn / 1 MiB，artifact `selfhost/emitter-vocabulary-v1.json` 由 145 → 340 站点重生成；`--check` = `EMITTER_VOCABULARY_CURRENT sites=340`。

## 7. 验证

| 项 | 结果 |
|---|---|
| `cargo fmt`（runner） | exit 0 |
| `cargo clippy --test selfhost_compiler -- -D warnings` | exit 0（3.25 s 增量） |
| `census-m22-match-sites.py --update` / `--check` | `LEDGER_WRITTEN sites=396 pending=0` / `LEDGER_CURRENT sites=396` |
| `census-m22-emitter-vocabulary.py --update` / `--check` | `EMITTER_VOCABULARY_WRITTEN sites=340` / `EMITTER_VOCABULARY_CURRENT sites=340`（新增 7 站点 / 1 个名字缺口） |
| `every_match_site_is_measured_on_the_component_axis` | **ok**（随 ledger 复跑同轮） |
| `sico_compiler_reproduces_the_frozen_match_site_ledger` | **ok**（同轮重测 95 组，**6,052.86 s**；逐站 verdict 与摘要和刷新后的账本一致）；同轮 `every_match_site_is_measured_on_the_component_axis` 也 **ok**（2 passed / 1 ignored） |
| `cargo fmt --all -- --check`（根） | exit 0 |
| `git diff --check` | exit 0 |
| 开发期子集 `tools/validate-m22-s5-dev.ps1 -Set fast` | **无法运行：该脚本不在本 worktree**（`tools/` 只有 `validate-m22-s5.ps1`；STEP-0549 引用的 dev 子集脚本未入库，属 §10.11 A2 既有缺口）⇒ 逐项手工等价执行上表命令 |
| 全轮电池（15 拍） | **未跑**（P2 不是族收口片，按 §10.1.2 分层不跑整轮） |

## 8. 边界（本片**没有**证明的事）

- **没有回绿任何站点**：组件轴仍是 **32 / 396**；本片的价值是**把「跳过 219 个」变成「全部有 verdict」并给出结构性判读**。
- 只覆盖 `U64.checked_add` 头的 396 个 `match` 站点；T2/T3–T20 的其余形状面与 519 个 `match` 总数不在本片口径内。
- 95 组是「按闭包去重后」的测量数；同一函数的多个站点共享一个 verdict，逐站证据来自同一闭包的字节对比，不是逐站独立编译。
- 上限只覆盖冻结 S 的当前最大闭包；S 变更后需重跑 `tools/probe-m22-closure-bounds.py` 复核，不能把 256 fn / 1 MiB 当成永久余量。
- 8 MiB trap 只测到一个组（`scalar_ir`），没有测「刚好低于 / 刚好高于」的边界；该上限来自 runner 结果层，未做任何放宽。
- 本片**未推送**：`codex/m22-w2` 仍领先 origin，推送需 owner 授权。

## 9. 下一步

1. **B2（B 路修复）**：在 `selfhost/codegen.sico` 为函数级「值 id → 槽号」建一次索引（前缀或哈希），把 `cgs_value_slot` 的逐指令重扫降到 O(1)、每个函数总量降到 O(n)；`cgj_array_item` 的重复从头遍历按实测热点决定是否一起修。exit = 至少 195 个新开测站点中的可回收部分回绿 + 既有 32 个全等站点逐字节不变。
2. **8 MiB 残差单独裁决**：先量 `scalar_ir` 的 IR 文本真实大小与构成，再决定子输出/流式校验，或明确登记为门 3 之外的测量限制；不得直接上调结果上限。
3. **5 个 refused-guest 站点**：按 P3 具名残差逐族处理。
4. **`sico.list.length[U64]`**：340 站点口径下的唯一词表缺口（7 个 `general_while_function_ir` 站点），按发射词表的四道闸门（名称 / 元数 / 类型 / 结果槽数）补一小组 helper；它不改变 D2，但必须单独出账。
