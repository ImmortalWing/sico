# STEP-0531 m22-w2 S5 **W2 族 T2 的 T2-g2 真值测量片**：嵌套调用实参的**目标 IR** 量出来了

> - status: **本机实测绿（随本片提交并推送；独立 CI 裁决待下一批回填）**
> - date: 2026-10-24
> - owners: autonomous-agent
> - depends: [STEP-0529](./STEP-0529-m22-w2-s5-w2-t2-frontend-knife.md)（**T2-g1 前端刀 F**）、[STEP-0530](./STEP-0530-m22-w2-s5-w2-t2-codegen-knife.md)（**T2-c1 codegen 刀 C**）、[STEP-0525](./STEP-0525-m22-w2-s5-w2-t1-call-arg-nested-call.md)（**T1 族 G-f2 刀**：嵌套调用实参的既有先例）、[STEP-0528](./STEP-0528-m22-w2-s5-w2-t2-text-length-truth.md)（本族拍 1 的方法学先例）
> - gate: **不推进任何出口门** —— 本片是 **T2-g2 的真值测量片（truth slice）**，**guest 生产代码零改动**、**oracle 生产代码零改动** ⇒ **不包装为特性支持**
> - numbering: **STEP-0531**（取号前已核对 `docs/steps/`：最大号 0530；*0531 未被占用*；M24 轨 STEP-0330–0337、M23 轨 0489/0490，均不冲突）
> - 产物: [支持面登记表 §3.2](../reports/m22-support-surface-t2-text-length.md#32-t2-g2-的实测真值step-0531实现未做)

## 0. 入口门槛

| 入口条件 | 状态 |
|---|---|
| T2 族的**其余两条缺口已闭合**（T2-g1 = STEP-0529、T2-c1 = STEP-0530）⇒ 只剩 T2-g2 | ✅ |
| 上一刀**只登记缺口、不预支能力**：STEP-0530 §7.3 写明「下一刀 = T2-g2 刀：让 guest 前端认**实参本身是嵌套调用**的 `sico.text.length`」 | ✅ |
| 出口测试有界 | ✅ §2：**2 条新钉**（边界拒绝钉 + **目标 IR 钉**）+ 前沿探针 51 → **53** 条用例 |
| **本片零实现改动** | ✅ §4.1（`selfhost/*` 与 `crates/*` **零改动**） |

⇒ **取号合法**。*实现已完成并本地实测绿*，见 §5。

## 1. 这一片回答的问题

STEP-0530 之后，T2 族**只剩一条缺口**：**T2-g2 = 实参本身是嵌套调用**。S 内 **2** 个调用点（`compiler.sico:57`、`compiler.sico:361`）落在这个形状上。

**但「形状」这两个字底下压着三个没量出来的东西**：

1. **目标 IR 到底长什么样？**（oracle 把嵌套调用降成几条指令？值 id 怎么排？）
2. **guest 侧的拒绝身份是什么？**（是语义拒绝，还是前端层的形状缺口？）
3. **下一刀该改哪条路由？**（T1 族已经实现了嵌套调用实参——**能不能照搬**？）

**本片的结论（先说结果）**：

- **目标 IR 已量清并钉住**：**嵌套调用是 1 条指令**；它的**结果 id 先于**外层调用的结果 id（**后序分配**）；外层 `sico.text.length` 把那个 id 当作**唯一实参**。4 个形状的完整指令序列进了固件。
- **拒绝身份 = `ERR:E-SH-IR-EXPRESSION`**，5 个形状**全部一致**；而 oracle **全部能降下来** ⇒ 这是**前端层**的形状缺口，**不是**语义拒绝 ⇒ **T2-g2 仍未闭合**。
- **T1 族的做法**不能照搬**：T1 族的嵌套调用实参实现（`straight_text_concat_packed` 的递归 `:9200-9230` 与 `stc_call_operand_packed` `:9642`）**只服务 `sico.text.concat` 的直线路由**；T2 的两个 S 站点分别落在**通用宿主调用路由**与**固定算子实参路径**上 ⇒ 下一刀要动的是**另外两条路**（§3.3）。

## 2. 出口测试（钉）

| # | 钉（`runner/sico-runner/tests/selfhost_compiler.rs`） | 断言 | 状态 |
|---|---|---|---|
| 1 | `the_t2_g2_nested_call_shapes_stay_typed_refused`（**新**） | **5 个嵌套实参形状**（嵌套内建 / 嵌套用户 / **S 站点 ①** / **S 站点 ②** / S 站点 ① 的 `let` 绑定形）**全部** typed 拒绝 `ERR:E-SH-IR-EXPRESSION`，而 **oracle 全部能降** ⇒ 是前端层缺口、不是语义拒绝 | ✅ 新增 |
| 2 | `the_t2_g2_target_ir_is_the_nested_call_shape`（**新**） | **4 个形状的 oracle canonical IR 指令序列**逐条钉住（结果 id + 操作文本，用固件既有的 `plan_op_text` 拼写）⇒ **下一刀的目标被钉成可验证的数字** | ✅ 新增 |

⇒ 固件从 **161** 增至 **163** 条；前沿探针从 **51** 增至 **53** 条用例（**+2 条**：把 **T2-g2 的两个 S 站点**钉进探针，均 `refused:ERR:E-SH-IR-EXPRESSION`）。

## 3. 实施设计

### 3.1 测量方法（**口径可复现**）

- **oracle IR**：`cargo test -p sico-ir --test dump_oracle_ir`，`ORACLE_SOURCE` / `ORACLE_OUT` 指向 fixture；脚本 `target/tmp/step0531/measure_t2g2.py`（**4 个形状**）。
- **guest 判定**：runner + guest 组件走 `--emit-ir-split`（前端层判词），脚本 `target/tmp/step0531/guest_verdict.py`（**5 个形状**）。
- **CLI 口径复核**：前沿探针走 CLI ⇒ 另用 `target/tmp/step0531/cli_verdict.py` 复核两个 S 站点的判词与探针一致（**CLI 与 runner 口径一致**）。
- **固件**：目标 IR 用**固件既有的 `plan_op_text`** 拼写（`call 2 args=[0]` / `intrinsic sico.text.concat args=[0,1]` / `const_string "x"`）——**不另造一套拼写**。

### 3.2 目标规则（**实测**）

```
嵌套调用 = 1 条指令
嵌套调用的结果 id  <  外层调用的结果 id      （后序分配）
外层 sico.text.length 的唯一实参 = 嵌套调用的结果 id
```

### 3.3 下一刀要动的两条路由（**实测定位**）

| S 站点 | 形状 | 落点 |
|---|---|---|
| `compiler.sico:57` | 嵌套**内建**调用（`sico.bytes.utf8_decode(src)`） | **通用宿主调用路由** `gw_intrinsic_call_packed`（`selfhost/parser.sico:940`）的实参分支——现为「形参 / 局部 cell / 否则 `ERR:E-SH-IR-CALL-ARGUMENT`」**三路** |
| `compiler.sico:361` | 嵌套**用户**调用（`positive_sleb_hex(initial)`） | **固定算子实参路径**（`U64.checked_div` 的实参）——**不在**宿主调用路由里 |

> **这解释了为什么 T1 族的既有实现不能直接复用**：T1 族把「嵌套内建调用」做成了 `straight_text_concat_packed` 的**自身递归**（`:9200-9230`），把「嵌套用户调用」做成了 `stc_call_operand_packed`（`:9642`）——两者都**只服务 `sico.text.concat` 的直线路由**。T2 的 S 站点在**通用路由**与**固定算子实参路径**上 ⇒ 下一刀**不是**改 `straight_text_length_packed` 就够。

## 4. 实施记录（2026-10-24）

### 4.1 本片**零实现改动**

`selfhost/*` 与 `crates/*` **零改动** ⇒ **零自举风险**、**零字节等同性风险**。改动面只有：**2 条固件钉** + **2 条前沿探针用例** + **文档**。

### 4.2 一个容易踩的坑：`plan_id_list` 不带空格

固件的 `plan_id_list` 用 `","` 拼接（**无空格**）⇒ 实参列表的文本是 `args=[0,1]` 而不是 `args=[0, 1]`。第一版钉按带空格写 ⇒ **首跑红**（`left: "intrinsic sico.text.concat args=[0,1]"` vs `right: "...args=[0, 1]"`）⇒ 改钉的期望值 ⇒ 绿。

> **教训**：**钉住的是「另一台机器的拼写」** ⇒ 拼写必须**读**（复用固件既有的 `plan_op_text`），不能**凭直觉写**。

### 4.3 5 个形状的判词**一致**

嵌套内建、嵌套用户、S 站点 ①、S 站点 ②、S 站点 ① 的 `let` 绑定形——**5 个全部** `ERR:E-SH-IR-EXPRESSION`（runner 与 CLI 两种口径一致）⇒ **没有隐藏的分层** ⇒ 下一刀只需要处理**一条**形状规则。

## 5. 验证记录（2026-10-24）

| 验证 | 口径 | 结果 |
|---|---|---|
| 固件全量 | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1`（`SICO_SELFHOST_COMPONENT_CACHE=0`，冷构建） | 见 §6.2 |
| oracle 真值 | `python target/tmp/step0531/measure_t2g2.py` | ✅ 4 个形状的 canonical IR（§3.2） |
| guest 判词 | `python target/tmp/step0531/guest_verdict.py` | ✅ 5 个形状全部 `ERR:E-SH-IR-EXPRESSION` |
| CLI 口径复核 | `python target/tmp/step0531/cli_verdict.py` | ✅ 两个 S 站点与探针期望一致 |
| 前端前沿探针 | `powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-frontend-frontier.ps1` | 见 §6.2 |
| 本机电池 | `target/tmp/step0531/battery.ps1`（**19 步**） | 见 §6.2 |

### 6.1 T2 规则表更新

[T2 支持面登记表 §3.2](../reports/m22-support-surface-t2-text-length.md#32-t2-g2-的实测真值step-0531实现未做) 新增 **T2-g2 的实测真值**小节（5 行形状表 + 规则 + 两条路由的落点）。

### 6.2 电池

本片电池 = `target/tmp/step0531/battery.ps1`（**19 步** = STEP-0530 的 19 步原样复用，把
`step0530-doc-check` 换成 `step0531-doc-check`），日志 `target/tmp/step0531/battery.log`。

| 步骤 | 耗时 | 裁决 |
|---|---|---|
| `workspace-integrity` | 51s | ✅ |
| `fmt-runner` | 1s | ✅ |
| `ci-one-key` | 411s | ✅（**CI GREEN**：fmt / clippy / build / test(workspace) / test(runner) / module boundaries / comment discipline / planning contract / profile matrix / cross-host / whitespace 全 PASS） |
| `validate-0245` | 123s | ✅ |
| `validate-0262` | 140s | ✅ |
| `probe-frontend-frontier` | 353s | ✅（**53** 条用例全部与期望一致） |
| `probe-ir-plan-frontier` | 26s | ✅ |
| `validate-0124` | 1s | ✅ |
| `validate-0131` | 1s | ✅ |
| `validate-0156` | 1s | ✅ |
| `git-diff-check` | 0s | ✅ |
| `line-ending-audit` | 0s | ✅ |
| `selfhost-comment-discipline` | 0s | ✅ |
| `step0526-footprint-audit` | 0s | ✅ |
| `step0526-census-branches` | 0s | ✅ |
| `step0526-differential` | 84s | ✅ |
| `step0528-census-t2` | 0s | ✅ |
| `step0531-doc-check` | 0s | ✅ |
| `workspace-integrity-final` | 51s | ✅ |

⇒ **BATTERY_GREEN 19/19**（**首轮即绿**——本片只动固件与文档，`cargo fmt` 在提交前已过）

### 6.3 独立 CI 回填（run #37）

- **STEP-0530** 推送（`13507b8`）触发的 **run #37**（run id `40024577187`，head_sha `13507b80862b694ca8b765cde73db55e12c9426b`）= 裁决由**本片回填**。
- **本片**推送后触发新一轮，裁决由**下一片**回填。

## 7. 诚实性

### 7.1 本片**只测量，不实现**

- `selfhost/*` 与 `crates/*` **零改动** ⇒ **T2-g2 仍未闭合**，**不得**读作「已支持嵌套调用实参」。
- **不推进任何出口门**；**M22 整体仍为 NO-GO**（门 1/4/5/7 GO，门 2 PARTIAL，门 3/6 NO-GO）。
- 前沿探针 **0 条期望翻转** ⇒ 前端前沿不动（本片不动前端）。

### 7.2 未闭合的（承自本片）

- **T2-g2 嵌套调用实参**（S 内 **2** 个调用点）仍 typed 拒绝 ⇒ **T2 族仍不得降级**。
- **语料覆盖仍 0 条**：S 内 322 个 T2 调用点一个都没进差分语料。
- **整文件自举仍未关闭**（guest 不能整体降 `codegen.sico`：`List[Text]` ⇒ `ERR:E-SH-IR-RECORD-TYPE`）。

### 7.3 下一刀（W2 族 T2 的第 4 拍 / **T2-g2 实现刀**）

**T2-g2 刀**：让 guest 前端认**实参本身是嵌套调用**的 `sico.text.length`。落点已由本片**实测定位**（§3.3）：**通用宿主调用路由** `gw_intrinsic_call_packed` 的实参分支 + **固定算子实参路径**。两条路都要认「实参是调用」这一形状，且**都不得**放宽 `sico.text.length` 的**元数**（恰好 1）与**实参类型**（`Text`）闸门。

**约束（承自 STEP-0525 / STEP-0529 的教训）**：**新形状必须落在既有可降形状的延长线上**——T1 族已经证明「嵌套调用实参」可降（`stc_call_operand_packed`），但它**只服务直线路由**；通用路由与固定算子路径是**新落点** ⇒ 新代码本身**必须落在 guest 前端可降的形状上**（STEP-0529 的 `match` 嵌套教训）。之后是 **T2 族收口片（X）**。
