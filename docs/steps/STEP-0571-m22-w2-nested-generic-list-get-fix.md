# STEP-0571 · M22-W2 · 嵌套泛型 `list.get[U64]` 前端 token 泛化：修复配方已验证，但被冻结 ledger 爆炸半径阻挡

| 项 | 值 |
|---|---|
| 片型 | **实现片（先红后绿已达成）+ 落地阻挡登记**：把 STEP-0563 收窄的单行根因（`gw_list_get_subject_packed` 不消费 `[U64]` 泛型 token）落地为可工作修复，**实测把 4 个既有红中的 2 个（step0519/step0520）转绿且 guest/Rust 逐字节等同**；但全量回归暴露**第二个共享热点 `selfhost/match-component-coverage-v1.json`（396 站点冻结 ledger）被行号位移打破**，本片按纪律**回退保基线**、交付配方 + 阻挡证据，不破坏全蜂群冻结契约 |
| 状态 | **配方验证 GO / 落地 NO-GO（本片内）**。`parser.sico` 工作树**已回退到 HEAD 基线**（sha256 `b6cb6ab0…` = 开工前记录值），`match-component-coverage-v1.json` 同样回退。工作配方完整保存在 `target/tmp/parser.sico.step0571.fixed`（23165 行，4 处修改点齐全） |
| 门 | **不翻任何门**。M22 overall **NO-GO** 不变；本片未改 gate 文本/证据等级/契约/语料 |
| 证据等级 | `internal-fixture`：本机 Windows x64，debug 构建，runner 独立 workspace，`--test-threads=1` |

## 1. 开工前热文件纪律记录（实测）

| 文件 | 开工前 sha256 | 收尾时 |
|---|---|---|
| `selfhost/parser.sico` | `b6cb6ab05669dbda9d1bcbe986950ccf66acf1fdfdaec8e796213e48e4d5ef31` | **同值（已回退保基线）** |
| `runner/sico-runner/tests/selfhost_compiler.rs` | `0e32b6ff2fc892463b552626b2360d7e8b5c25c09661e367eac2270935c318dc` | 未被本片改动（工作树 `M` 是**上一批**的格式化改动，本片保留未动） |

HEAD `a9398627e51c21af90b019b0e7caae6615f30d73`，分支 `codex/m22-w2`。

## 2. 先红：4 个钉基线（实测，预构建二进制 `selfhost_compiler-4fa1b02d73be6796.exe`）

| 钉 | 第一个拒者 | 基线诊断 |
|---|---|---|
| `step0507` | `cgs_result_return_hex` | `ERR:E-SH-IR-EXPRESSION`（EXPRESSION 层，**独立根因**） |
| `step0519` | `cgs_value_ty` | `ERR:E-SH-IR-CALL-SHAPE`（**本片根因**） |
| `step0520` | `cgs_value_slot` | 同 `cgs_value_ty` 形状（**本片根因**） |
| `step0526` | `cgs_function_instr_hex` | 非 Output 结果（嵌套 `while`，**独立根因**） |

CLI 交叉复现（`sico run selfhost/compiler.sico`）：夹具 `I_if_list_u64_3param.sico`（嵌套 `if` × `List[U64]` × `[U64]`）= `ERR:E-SH-IR-CALL-SHAPE`；对照 `H_body_list_u64_3param.sico`（全身体）= `sico.ir.v0` LOWERED。与 STEP-0563 §3 判别矩阵一致。

## 3. 修复配方（4 处，逐字节照搬既有形状，已验证）

全部在 `selfhost/parser.sico`，纯插入式 + 既有拒绝行原样保留。工作副本：`target/tmp/parser.sico.step0571.fixed`。

### 3.1 `gw_list_get_subject_packed`（:13958）入口泛型 token 扫描

在 `let glg_open = next_index(glg_get_seg, glg_n)` 之后插入（照搬 `list_get_match_ir:19012` 的 `[` 分支形状）：

```sico
  let glg_elem_json = "string"
  let glg_elem_kind = "Text"
  let glg_generic = I64.literal(0)
  if is_word(word_at(words, glg_open), "["):
    let glg_gen_ty = next_index(glg_open, glg_n)
    let glg_gen_close = next_index(glg_gen_ty, glg_n)
    if is_word(word_at(words, glg_gen_close), "]"):
      let glg_gen_ok = I64.literal(1)
    else:
      return "ERR:E-SH-IR-CALL-SHAPE"
    end if
    if is_word(word_at(words, glg_gen_ty), "U64"):
      let glg_gen_ty_ok = I64.literal(1)
    else:
      if is_word(word_at(words, glg_gen_ty), "Text"):
        let glg_gen_ty_text_ok = I64.literal(1)
      else:
        return "ERR:E-SH-IR-CALL-TYPE"
      end if
    end if
    set glg_elem_json = "u64"
    set glg_elem_kind = word_at(words, glg_gen_ty)
    set glg_generic = I64.literal(1)
    set glg_open = next_index(glg_gen_close, glg_n)
  end if
```

关键：`glg_open` 的**首绑定必须是 `let`**（首版误写成 `set` ⇒ guest 前端报 `unsupported set without prior let`，整个 `compiler.sico` 无法降低；已修正）。后续 `glg_list_atom`/`glg_close` 由更新后的 `glg_open` 派生 ⇒ 自动指向 `(` 与其匹配 `)`。

### 3.2 同函数 list 参数 kind 门放行 `List[u64]`

原 `:14045` 只认 `{"kind":"list","data":{"kind":"string"}}` ⇒ 对 `List[U64]` 返回 `CALL-TYPE`。改为 string/u64 二者皆放行（否则 `return "ERR:E-SH-IR-CALL-TYPE"` 原样保留）。

### 3.3 同函数结果类型 + 内建 op 名随元素类型泛化

- `glg_ty` 由硬编码 `"...\"ok\":{\"kind\":\"string\"}..."` 改为 `sico.text.concat(prefix, sico.text.concat(glg_elem_json, "\"},\"error\"..."))`（注意 `\"}` 的闭合引号，首版漏写 ⇒ `{"kind":"u64}` 字节错，已修正）。
- op 名由硬编码 `"sico.list.get"` 改为按 `glg_generic` 追加 `[` + `glg_elem_kind` + `]` ⇒ `sico.list.get[U64]`（照搬 `list_get_match_ir:19498-19504`）。

### 3.4 `gw_match_open_pack`（:15703）match 结果类型随元素类型泛化

`:15887` `gmo_is_list_get` 分支把 `gmo_ok_ty` 硬编码为 `"string"` ⇒ `#match1` cell 的 result 类型错。改为按主语 `sico.list.get[U64]` 的 `[` token（`gmo_subject` 后第 5 个 token）判 `U64`/`Text`，分别置 `"u64"`/`"string"`（`gmo_subject`→`[` 是 **5** 次 `next_index`，首版误用 4 次 ⇒ 指到 `get` 而非 `[`，已修正）。

## 4. 先绿：验证矩阵实测

重建测试二进制（`cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler --no-run`，rust-lld 链接成功）后：

| 项 | 结果 |
|---|---|
| `step0519`（`cgs_value_ty`） | **转绿** `test result: ok`（CALL-SHAPE → 逐字节等同） |
| `step0520`（`cgs_value_slot`） | **转绿** `test result: ok` |
| `step0526`（`cgs_function_instr_hex`） | **仍红**（非 Output，独立根因，未回退） |
| `step0507`（`cgs_result_return_hex`） | **仍红** `ERR:E-SH-IR-EXPRESSION`（EXPRESSION 层独立根因，按任务边界**未扩大改动面**） |
| CLI 夹具 `I_if_list_u64_3param` | `sico.ir.v0` LOWERED（原 CALL-SHAPE） |
| typed 拒绝身份 | 既有拒绝行原样保留；泛型实参非法时仍 `CALL-TYPE`/`CALL-SHAPE` |

## 5. 全量回归：暴露冻结 ledger 爆炸半径（本片回退的根因）

重建后跑完整 `selfhost_compiler` 套件（`--test-threads=1`，264s）：

```
test result: FAILED. 184 passed; 3 failed; 5 ignored; 0 measured
```

3 个红中 2 个是**基线既有**（step0507 EXPRESSION、step0526 非 Output），但第 3 个是**本片新增**：

```
MS-267: the frozen site line no longer holds the match head
```

根因（实测）：`selfhost/match-component-coverage-v1.json` 是 **396 站点冻结期望表**，每站点钉 `line`/`function_line`/`source_sha256`/`ir_sha256`/`component_sha256`。本片在 `gw_list_get_subject_packed`（:13958）和 `gw_match_open_pack`（:15703）插入行 ⇒ **130 个 parser.sico 站点位于插入点之下、行号全部位移**；MS-267 正是 `gw_list_get_subject_packed` 自己的站点（原 `line:14102`）。`tools/census-m22-match-sites.py --check` 报 `LEDGER_STALE`。

尝试用 census 重建 ledger：`python tools/census-m22-match-sites.py` ⇒ `LEDGER_WRITTEN sites=396`，但 `git diff --stat` = **257 行变更**。census 对"未移动"站点**保留旧 verdict/digest**，而本片改的两个函数内部站点 source 字节已变 ⇒ digest 失配风险未消除；且 ledger JSON 由 census 工具维护、**不在本片写权限内**（本片仅 `parser.sico` + `selfhost_compiler.rs`）。ledger firmware test 还在 IR + Component 双轴对 396 站点跑 wasmparser 校验（单测 >8 分钟未完）。

## 6. 纪律决定：回退保基线

按任务 §6 明文（"若 census 或全量套件出现**非预期**红：先 `git checkout -- selfhost/parser.sico` 回退保基线…宁可**不交付不破坏**"）+ AGENTS.md §10.2 热文件爆炸半径：本片**回退 `parser.sico` 与 `match-component-coverage-v1.json`**，保护全蜂群冻结契约基线。收尾复跑 `sha256sum selfhost/parser.sico` = `b6cb6ab0…`（= 开工前值）确认回退干净。

⇒ **本片交付 = 已验证的 4 处修复配方（§3，可原样重放）+ 2 钉转绿证据（§4）+ ledger 爆炸半径阻挡的精确定位（§5）**，供拥有 ledger 重建权限/预算的后续片执行"配方落地 + `census-m22-match-sites.py` 重建 + ledger firmware 复跑"的协调变更。**诚实边界**：配方仅前端 token/类型泛化，未动语言面、未触发 RFC-0033 门；`cgs_result_return_hex`（step0507 EXPRESSION 层）与 `cgs_function_instr_hex`（step0526 非 Output）是**独立根因，本片未修、未为其转绿扩大改动面**。

## 7. 残余风险 / 后续片待办

1. **ledger 协调重建**：落地配方需同时 `census-m22-match-sites.py` 重建 130 个位移站点行号，并对两个被改函数（`gw_list_get_subject_packed`/`gw_match_open_pack`）内部站点**重测 source/ir/component digest + verdict**（component 轴 FuelExhausted 站点可能翻转）。
2. **ledger firmware 双轴校验耗时**：单测 >8 分钟，需独立时间预算。
3. **step0507 / step0526 独立根因未动**：EXPRESSION 层与非 Output 各自归因，需另片收口。
4. **配方重放**：`target/tmp/parser.sico.step0571.fixed` 是完整工作副本（含全部 4 处修改），可直接 diff 重放；临时目录可能被清理，配方正文已完整固化在本文件 §3。

## Addendum（2026-11-07）——配方已落地，两个目标红测转绿；账本 firmware 红为既有燃料墙乐观口径

> 本 addendum 订正本文件表头的「配方验证 GO / 落地 NO-GO（本片内）」与 §6「回退保基线」状态：**§3 配方现已落地到 `selfhost/parser.sico` 工作树**（HEAD 未变，`af6dcb7`），**两个目标红测（step0519/step0520）已转绿且逐字节等同**。落地过程与隔离取证详见 [`STEP-0605`](./STEP-0605-m22-w2-step0571-recipe-landing.md)。

- **配方已落地**：`selfhost/parser.sico` 现含 §3.1–§3.4 的 4 处纯插入式编辑（`gw_list_get_subject_packed` ~:13958 与 `gw_match_open_pack` ~:15703），全部 typed-refusal 臂原样保留。工作树 `parser.sico` sha256 = `b67ec56636d0211ce437b88dbfeeaceb9bad52ec45cff390922bf89d9eafd4aa`，23168 行（相对已提交基线 `b6cb6ab0…` 为 +52/−3）。
- **两个目标红测转绿**（`--exact` 复跑，toolchain 1.98.0-x86_64-pc-windows-gnu，coordinator 亲跑）：`sico_compiler_lowers_the_step0519_codegen_functions_byte_exactly` → ok（1 passed，4.21 s）；`sico_compiler_lowers_the_step0520_codegen_functions_byte_exactly` → ok（1 passed，3.52 s）。两者均为 `*_byte_exactly` ⇒ guest/Rust 组件逐字节等同确认。
- **隔离守卫不变**（证明配方未波及两个独立红测）：`step0507`（`cgs_result_return_hex`）仍 FAILED，`ERR:E-SH-IR-EXPRESSION`（EXPRESSION 层独立根因）；`step0526`（`cgs_function_instr_hex`）仍 FAILED，非 Output 嵌套 `while` 独立根因。
- **账本已随配方重编号**：`selfhost/match-component-coverage-v1.json` +257/−257（两个插入锚点之下 parser.sico 站点行号位移，如 MS-267 14102→14139、MS-268 14113→14151）。`selfhost/codegen.sico` 未触碰（clean）。
- **账本 firmware 测试仍红，但为既有问题、非本配方回归**：`sico_compiler_reproduces_the_frozen_match_site_ledger`（`--exact`，3259 s ≈54 min）失败于 MS-081——账本冻结 verdict `"ir-equal+component-equal"` vs 实测 `"ir-equal+component-other:FuelExhausted"`。**既有、与配方无关的证明**：(a) HEAD 的 MS-081 具完全相同 verdict 与相同 line 4577；(b) MS-081..084 是 `fixed_literal_const_segment`（checked_add）站点（行 4577/4590/4622/4629），全部位于配方插入锚点（13958/15703）**之前**，而配方只改泛型 `list.get` ⇒ 不可能影响它们。这正是 AGENTS §10.1.3 记录的乐观口径：账本 36 个 `component-equal` 站点（含 MS-081..084）实为 FuelExhausted（共 331 个耗尽站点），旧「36 component-equal」偏乐观。
- **MS-081 修正的阻塞项**：正确的 FuelExhausted 态 `component_sha256` 只能由账本重算工具 `tools/census-m22-match-sites.py` 产出，但本机**无可用 Python**（仅 WindowsApps stub；`SICO_PYTHON` 未设；`Python312` 目录只有 Lib/Scripts，无 python.exe）⇒ MS-081 修正顺延到燃料墙/D8 轨。
- **诚实溯源**：`parser.sico` 配方编辑与账本重编号在落地 sub-agent 运行时**已在工作树中存在**（HEAD 未变，`af6dcb7`）；该 sub-agent 未施加任何编辑。coordinator 已核 diff 恰为 §3 配方且两个目标测试逐字节转绿，故**以已验证状态采纳**——但精确施加时间/agent 不确定（可能为本会话早前一次 sub-agent 运行）。此处如实记为「已验证状态采纳 + 溯源不确定」，不作过度声称。
- **未翻任何门**：M22 overall 仍 **NO-GO**；未提交、未推送。