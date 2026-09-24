# STEP-0279: W1 片 C — E7002 关键词指纹改结构性 + 语料冻结扩容 215→230

> - status: complete（guest 实现 + 语料重冻结 + 测试计数更新已落地；runner
>   测试仓本机不可重建——gcc/dlltool 缺失——集成链以 CI 裁决）
> - phase: M22 compiler self-host — W1 consolidation（[`M22 restart assessment v1`](../plans/M22-restart-assessment-v1.md) §5 item 2；计划 = STEP-0277）
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: internal-fixture（本机：oracle 对照 + checker 组件 230 条全量探针 230/230；CI：集成链全量）

## 1. Objective

按 STEP-0277 §5 分片顺序执行 W1 片 C：把 E7002（UNCHECKED_STALE_RESULT）
从「`Model`/`loaded`/`model` 关键词指纹」改写为与 Rust oracle
（`sico-semantics/src/lib.rs:1336-1432`）同构的结构性判定，并以
rename-safety 语料钉死行为。

途中发现一笔卫生债：0e4cbb9 批（STEP-0276 等）新增 12 个 tracked
`tests/end-to-end/record-*.sico` 语料源，但未重冻结 corpus——
`update-m22-corpus.ps1 -Verify`（validate-step-0214/0220/0246/0248 引用）
以「清单覆盖 roots 下全部源文件」为不变式，属必还债。本片一并吸收：
冻结扩容 215 → 230，guest 不一致的 8 条以显式豁免清单登记（W2 关闭）。

## 2. Changes

### 2.1 guest 改写（`selfhost/compiler_semantics.sico`）

判定语义（与 oracle 对齐）：函数有形参 P 的类型是 record 且含
`field text`（任意类型）+ `field revision: Revision`（versioned payload）；
函数体内任一行有词对 `[P, text]` 且 P 不在守卫集 → E7002。守卫 = 含
`[P, revision]` 词对且行内含 `==`（`=` 字节 ≥ 2）的 `if` 行，效力到配对
`end if`（按 if 嵌套深度弹出，深度以 `sico.u64.to_text` 存 Text——
语义层 List 仅支持 `List[Text]`）。

- 第一遍声明扫描：`field` 分支补平行 `field_types` 表（`at(parts, 2)`）。
- 新增 helper：`record_has_field` / `record_field_type` /
  `is_versioned_record` / `versioned_payloads_for`（8 参）/
  `words_adjacent`。
- 函数头分支计算 `versioned_payloads` 并重置守卫状态；`end function`
  分支同步清空。
- E7002 apply 检查置于 in-function 分支顶部（旧位置只查 `return` 行，
  新位置查任意行——有意的行为扩展，与 oracle「任何行」一致）。
- `if`/`end if` 行处理替换旧 `revision_guard` 单比特机制（嵌套/弹出语义
  经手工推演，见 §3）。
- 删除旧指纹块（`at(parts,1)=="Model" && word_after(text)==loaded &&
  word_after(revision)==model && revision_guard==0`）。
- **declared-subset 限制（如实登记）**：guest `words()` 丢标点，字段访问
  只能以词对近似（`fresh.text` 与 `fresh . text`、
  `x.fresh.textual` 不可区分）；oracle 以 token 窗 `[P, Dot, text]` 判定。
  语料无此形态，近似在当前冻结面无损。

### 2.2 途中发现并修复：关键字作参数名 check/build 缝隙

新 helper 初版把参数命名为 `record`，`sico check` 通过但 script-v0
lowering 报 `unsupported atomic expression`——关键字 `record` 作标识符
check 层未拒绝。**已改为 `record_name` 规避**；缝隙本身登记为
source/backend honesty 候选（M14 的 check/build/run 无暗隙 gate 相关），
本片不扩大化。

### 2.3 rename-safety 语料（oracle 本机实取期望）

| 文件 | oracle（`sico check --json`） | guest 探针 |
| --- | --- | --- |
| `syntax-candidates/b/revision/invalid/unchecked-stale-result-renamed.sico`（REV-201） | E7002 | E7002 ✅ |
| `syntax-candidates/b/revision/invalid/cross-payload-guard-stale-result.sico`（REV-203） | E7002（守卫只覆盖被比较的 payload） | E7002 ✅ |
| `syntax-candidates/b/revision/valid/stale-load-ignored-renamed.sico`（REV-202） | accepted | accepted ✅ |

REV-203 初版用 `!=`/`<` 表达式——实测 `!` 与 `>` 在 guest 词法字母表
外（oracle 亦 `UnexpectedCharacter` 词法拒绝），改为跨 payload 守卫
形态（old/other 有守卫、fresh 没有 → 仍须 E7002），测试守卫的
per-payload 作用域而非运算符词法。

### 2.4 语料重冻结 215 → 230

- `tools/update-m22-corpus.ps1` 重生成 corpus-v0.json +
  script-build-corpus-v0.json（本机无 rg，临时以 Get-ChildItem 枚举，
  生成后已还原脚本；枚举结果与 tracked+untracked 集合逐条核对一致，
  230 = 227 tracked + 3 新文件，无忽略文件差异）。
- 新增 15 条 = 3 条 revision（§2.3）+ 12 条 record-*（0e4cbb9 批漏冻结，
  §1）。既有 215 条零变动（逐路径字段比对）。
- 4 条 record 正例 guest 已与 oracle 一致（accepted）；8 条不一致：
  - `record-basic.sico`：oracle accepted，guest 误拒 E2011（字段表
    近似误伤，属 W2 records 重授权层范围）；
  - 7 条 `record-refusal-*`：oracle 拒（E2001/E2010/E2011/E2021..E2024），
    guest 放行（guest 尚无 E2021..E2024 规则与部分 E2xxx record 判定）。

### 2.5 测试同步

- `bootstrap_bundle.rs:164`：entry_count 215 → 230。
- `selfhost_checker.rs`：
  - 条目数 215 → 230；
  - 新增 `RECORD_DIVERGENCE_WAIVER`（8 条，§2.4 清单）：豁免条目计入
    unsupported，且**必须保持 diverge**——guest 一旦追上 oracle 即
    panic 要求重新分类，防止豁免清单静默腐烂；
  - 分区断言 (116, 34, 65, 0) → (116, 36, 70, 8)
    （+2 E7002 identity、+5 accepted〔REV-202 + 4 条 record 正例〕、
    +8 waived unsupported）。

## 3. Validation（本机实测）

- `sico check selfhost/checker.sico` ok；`sico build --profile script-v0`
  产出 checker.component.wasm 成功（含关键字参数修复后）。
- oracle 对照：3 条新语料期望全部本机实取（§2.3 表）。
- **checker 组件 230 条全量探针：230/230 pass**（8 条豁免按 diverge
  语义核对：7 条 guest-accept vs oracle-refuse、1 条 guest-E2011 vs
  oracle-accept，与 §2.4 登记一致）。驱动 = 预构建 runner CLI，
  `--fuel 5000000000`，8 路并行，约 197 s。
- `validate-step-0124.ps1`、`git diff --check` 绿（见提交前复跑）。
- 本机不可运行（已知宿主限制）：runner 测试仓 cargo 构建（gcc/dlltool），
  集成链 230 条全量 + canary 以 CI 裁决（同 STEP-0272/0276/0278 惯例）。

## 4. Exit test 对照（W1 item 2）

- E7002 不再依赖 `Model`/`loaded`/`model` 字面量（rename-safety 语料
  REV-201 钉死）✅；守卫 per-payload 作用域（REV-203）✅；嵌套 if 弹出
  语义手工推演 + REV-202/203 覆盖 ✅；旧 `revision_guard` 机制与指纹块
  零残留（grep 证实）✅；230 全量绿（本机探针）✅；CI 裁决项如实登记。

## 5. Follow-ups

- 片 D（item 4）：0-indent 语料先行定 `same(raw, line)` 行为
  （实位 `compiler_parser.sico:489/650`），再定 refused/structural。
- W2：records 重授权层关闭 8 条豁免（E2021..E2024 + record 字段判定
  近似误伤），随后删除 `RECORD_DIVERGENCE_WAIVER`。
- 候选债（登记不扩大化）：关键字作参数名 check 通过 / lowering 拒绝的
  缝隙；guest `words()` 词对近似 vs oracle token 窗的 declared-subset
  差（当前冻结面无损）。
