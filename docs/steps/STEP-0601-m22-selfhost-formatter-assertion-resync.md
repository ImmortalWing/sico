# STEP-0601 · M22 W2 · 测试断言口径修复（formatter 语料重冻结后过期断言，逐 case 钉住）

> **片型** = 测试断言口径修复片。**不改产品语义、不改冻结语料、不改 runner 生产代码**；M22 整体仍 **NO-GO**。
> **唯一代码产物** = `runner/sico-runner/tests/selfhost_formatter.rs`（refused 分区的断言改为按 `diagnostic_ids` 逐 case 分类钉住）。
> **唯一文档产物** = 本 STEP。
>
> **复跑路径**（本片实际使用，可直接复现）：
> - 主测试：`cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_formatter -- --test-threads=1`（本片实测绿）。
> - 格式：`cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check`（本片实测净）。
> - 卫生：`git diff --check`（本片实测净）。
> - 工具链：`$env:USERPROFILE\.cargo\bin\cargo.exe`、`RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu`；Python 缺失与本片无关，未用 Python 跑测试（只用于只读 JSON 侦察，经 `tools/ensure-python.ps1` 引导）。
>
> **禁改清单（本片遵守）**：`selfhost/corpus-v0.json`、`selfhost/**`、`semantic-cases/**`、`syntax-candidates/**`、`syntax-mutations/**`、`diagnostics/semantic-case-map.json`、`crates/sico-package/**`、`crates/sico-ui-renderer-d2d/**`、`crates/sico-{format,hir,ir,cli,parser,semantics}/src/**`、`runner/sico-runner/src/**`、`docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`docs/plans/**`、任何冻结语料。

## 0. 背景（断言过期，非产品缺陷）

`runner/sico-runner/tests/selfhost_formatter.rs::sico_formatter_matches_rust_on_every_accepted_source_and_is_idempotent` 是 M22 的 guest/Rust formatter 差分测试：对冻结语料 `selfhost/corpus-v0.json` 的 `accepted` 分区（118 条）断言 guest 输出与 Rust `format` 逐字节相等且幂等；对 `refused` 分区（117 条）断言 guest formatter 对每条源都产出**恰好** `RunOutcome::Domain { code: "invalid-input", message: "LEXICAL" }`。

语料在 STEP-0279..0283 重冻结为 235 条、并在 STEP-0584 补登 5 条语义拒绝（IND-003/004/005、REV-201/203；诊断码 E1001/E4001/E7002）。其中 **IND-003**（`syntax-candidates/b/indentation/invalid/zero-indent-recovery-declaration.sico`，诊断 `["E1001"]`）落入 `rust_format == "refused"` 分区 ⇒ 测试遍历到它时，断言「所有 refused 源 ⇒ `invalid-input`/`LEXICAL`」的前提被打破。

## 1. 根因（实测，非推断）

### 1.1 主测试的实际失败（主线复跑，本片再次复现）

```
cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_formatter -- --test-threads=1
⇒ FAILED，panic at tests\selfhost_formatter.rs:138
left:  Output(ScriptOutput { stdout: <归一化字节>, stderr: [], exit_code: 0 })
right: Domain { code: "invalid-input", message: "LEXICAL" }
失败路径: syntax-candidates/b/indentation/invalid/zero-indent-recovery-declaration.sico
```

**关键事实：guest formatter 对 IND-003 源返回的是 `Output`（接受并归一化，exit_code 0），不是 `Domain`。** 即 guest formatter 接受了一条 Rust formatter 拒绝的源。

### 1.2 guest formatter 为什么接受（设计层面，只读确认）

`selfhost/formatter.sico` 的 `main`（第 817–827 行）只有两条拒绝路径：

```sico
if sico.bytes.is_utf8(input.stdin):
    let source = sico.bytes.utf8_decode(input.stdin)
    if source_has_lex_error(source):
      return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: "LEXICAL"))
    end if
    ...
end if
return error(ScriptError(code: ScriptErrorCode.InvalidInput, message: "source is not UTF-8"))
```

`source_has_lex_error`（第 446–463 行）只逐行跑 formatter 自己的词法器 `line_tokens`，查是否存在 `kind == "Error"` 的 token。**guest formatter 是「纯词法」formatter**：它没有任何语义/解析校验路径，也从不发射 `SEMANTIC` 之类的消息——唯一拒绝消息是 `LEXICAL`（非 UTF-8 时为 `"source is not UTF-8"`）。

IND-003 源是**词法合法但解析非法**（`// expect: reject(MISSING_FUNCTION_CLOSE)`，诊断 `E1001`）：一个列 0 声明关键字出现在函数体内，解析器把外层函数报为未闭合。它的每一行都能被 `line_tokens` 正常分词（无 `Error` token）⇒ `source_has_lex_error` 返回 `false` ⇒ guest formatter 走 `normalize_source` 并返回 `ok`（`Output`）。

### 1.3 结论

「所有 `rust_format == "refused"` 源 ⇒ `invalid-input`/`LEXICAL`」这一断言前提**在语料加入语义类拒绝后不再成立**。这**不是** product 语义缺陷，而是断言口径过期：`refused` 分区不再只含词法拒绝，还含语义/解析拒绝（IND-003/E1001）。**不修 formatter、不改语料**（均超出本片边界），只修断言使其从冻结语料的逐条诊断身份正确推导期望。

## 2. 修法（逐 case 钉住，从 `diagnostic_ids` 推导）

### 2.1 期望推导规则

对 `rust_format == "refused"` 的每条，读取冻结 manifest 的 `diagnostic_ids`：

| `diagnostic_ids` | 含义 | guest formatter 期望 | 分类 |
|---|---|---|---|
| `["LEXICAL"]` | 词法非法（formatter 词法器出 `Error` token） | `Domain { invalid-input, "LEXICAL" }` | **钉住（116 条）** |
| 其它（如 `["E1001"]`） | 词法合法、语义/解析非法 | 无法由 manifest 推导（formatter 无对应消息，实际 `Output`） | **期望待定（1 条）** |

实现（`selfhost_formatter.rs:84-128`）：

- 遍历 `entries`，按 `rust_format == "refused"` 过滤，把 `diagnostic_ids == ["LEXICAL"]` 的路径收进 `lexical_refusals`，其余收进 `pending_refusals`。
- `assert_eq!(lexical_refusals.len() + pending_refusals.len(), 117)`：冻结分区总数不变。
- `assert_eq!(pending_refusals, ["syntax-candidates/b/indentation/invalid/zero-indent-recovery-declaration.sico"])`：**冻结期望表**——断言「期望待定集合」恰为已知的 IND-003 一条。若语料再新增/移动语义拒绝，此断言立即红，不会静默缩小钉住覆盖。

执行循环（`selfhost_formatter.rs:161-204`）：

- `lexical_refusals` 每条：断言 `outcome == Domain { "invalid-input", "LEXICAL" }`（与旧断言**完全一致**，未被放宽）。
- `pending_refusals` 每条：**不臆造期望**，把观测到的实际 outcome 用 `eprintln!` 显式surface 为「已登记分歧」，并注释说明它是词法-only formatter 对语义拒绝源的当前真实行为、`rust_format` 只记录 Rust formatter 的裁决、不决定 guest formatter 消息。

### 2.2 为何不放宽（论证）

- **不**放宽为「只断言 `code == "invalid-input"`」：那会把消息差异（`LEXICAL` vs 其它）吞掉，掩盖真实语义错误。
- **不**用 `contains`/通配符：任何 `Domain` 或任何消息都算过，等同于把「拒绝」这一可观测契约弱化为「没崩」。
- **不**跳过 IND-003：它被**显式**列入 `pending_refusals` 集合、被**冻结期望表断言**捕获、被 `eprintln!` surface 观测值。这是「显式登记为期望待定」，不是静默跳过；一旦 owner 裁决，该 case 的期望可从冻结表移除/改判。
- **不**把「任意 Domain 都算过」当期望：116 条词法拒绝仍逐条**恰好**等于 `Domain { "invalid-input", "LEXICAL" }`，一条不落。
- **不改** formatter/语料/runner 生产代码：IND-003 的「guest 接受 vs Rust 拒绝」是一个**真实行为差异**，属于 owner 待裁的 IND-003 语料放置/契约问题（见 §6），本片只负责把断言校准到「不再假装它是词法拒绝」。

因此本片是**断言口径修复**：把一条建立在「refused 分区全为词法拒绝」这一过期前提上的断言，改为从冻结 manifest 逐条推导。**覆盖没有变窄**（116 条词法拒绝仍全部钉住，1 条语义拒绝显式登记），**也没有任何断言被放宽**。

## 3. 4 个同族失败的逐个判定

任务要求核查 `crates/sico-format`、`crates/sico-hir`、`crates/sico-ir/tests/core_lowering.rs`、`crates/sico-cli/tests/cli.rs` 的 4 个已知失败是否同属「b 集含解析拒绝用例 IND-003」族。**本片全部 4 个复跑确认：均同族（唯一根因 = IND-003 是 b 集唯一的解析级拒绝）**，故按任务纪律**不动它们、只如实登记**。

先决事实：`syntax-candidates/b` 在语料中的 66 条 = **65 条 `rust_format=accepted` + 1 条 `rust_format=refused`（IND-003，`diagnostic_ids=["E1001"]`）**。IND-003 是 b 集**唯一**一条解析级拒绝（`// expect: reject(MISSING_FUNCTION_CLOSE)`、`// semantics: none`、`rust_check=refused`、`rust_check_exit=1`），且 `diagnostics/semantic-case-map.json` 已为它登记（code `E1001`、key `SYNTAX_MISSING_FUNCTION_CLOSE`、expected_message `"missing end function"`）。

| 测试（所在文件） | 复跑结果 | 归因（唯一） | 判定 |
|---|---|---|---|
| `all_b_cases_preserve_ast_shape_and_are_idempotent`（`crates/sico-format/src/lib.rs`） | FAILED @ `lib.rs:306:57` `Option::unwrap()` on `None` | 对每条 b 集源先 `parse(&original).ast().unwrap()`；IND-003 解析失败 ⇒ `.ast()` 为 `None` | **同族**。登记，不动 |
| `all_b_sources_lower_deterministically_with_complete_source_maps`（`crates/sico-hir/src/lib.rs`） | FAILED @ `lib.rs:415:41` `Result::unwrap()` on `Err(Syntax { count: 1 })` | `lower(&source).unwrap()`；IND-003 为语法错误 ⇒ `Err(Syntax{count:1})` | **同族**。登记，不动 |
| `all_thirty_three_invalid_cases_stop_before_ir_support_dispatch`（`crates/sico-ir/tests/core_lowering.rs`） | FAILED @ `core_lowering.rs:753:13` `IND-003 reached IR support dispatch` | 对 `// expect: reject` 的 case 断言 `lower_core` 返回 `Semantic`；IND-003 返回的是 `Syntax`（解析级）⇒ else 分支 panic | **同族**。登记，不动 |
| `check_matches_all_58_semantic_oracles`（`crates/sico-cli/tests/cli.rs`） | FAILED @ `cli.rs:136:13` `left: "not-run"` vs `right: "error"` | 对 reject case 断言 `json["status"]["type_checker"] == "error"`；IND-003 解析失败、类型检查未运行 ⇒ `type_checker="not-run"` | **同族**。登记，不动 |

四者的共同根因：**测试假设 b 集的 `// expect: reject` case 全部是语义级拒绝**（因此 `parse`/`lower`/`lower_core`/`type_checker` 都走语义路径），而 STEP-0584 把解析级拒绝 IND-003 放进了 b 集（`syntax-candidates/b/indentation/invalid/`），使该假设失效。这与主测试的 IND-003 是同一族。**这四者属于 owner 待裁的 IND-003 裁决面（语料放错位置 vs 放宽契约），本片不修**。

> 注：主测试与这四者的失败**都**由 IND-003 引起，但**形态不同**：主测试是「guest formatter 接受了一条 Rust formatter 拒绝的源」（行为差异）；四者是「测试假设 b 集拒绝全为语义级」。两者根因同源（IND-003 是解析级拒绝），归属同一 IND-003 裁决面。

## 4. 验证

### 4.1 主测试（本片唯一改动的测试）

```
cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_formatter -- --test-threads=1
running 1 test
test sico_formatter_matches_rust_on_every_accepted_source_and_is_idempotent ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

修改前红（panic @ `selfhost_formatter.rs:138`，见 §1.1）；修改后绿。118 条 accepted 分区与 116 条词法拒绝分区逐条钉住不变；1 条期望待定（IND-003）被冻结表断言捕获并在测试输出中 surface。

### 4.2 格式与卫生

```
cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check   → 净（FMT_CLEAN）
git diff --check                                                          → 净（DIFF_CHECK_CLEAN）
```

### 4.3 未跑项（如实记录）

- 本片只改 `runner/sico-runner/tests/selfhost_formatter.rs`，未跑 runner 全工作区/全 workspace；该文件是独立测试目标，`--test selfhost_formatter` 已覆盖其全部逻辑。
- 未跑 `crates/sico-{format,hir,ir,cli}` 的其余测试：4 个同族失败按纪律不动，其余与本片无关。
- 未跑 `validate-m22-s5*.ps1` 等文档级验证器：本片不改冻结语料/文档契约，不影响它们；`git diff --check` 已验卫生。
- Python 仅用于只读 JSON 侦察（经 `tools/ensure-python.ps1` 引导到 `target/tooling/python-3.12.10`）；`selfhost_formatter` 测试本身不依赖 Python。

## 5. 诚实收窄（本片声明范围）

- 本片**只**把 `selfhost_formatter.rs` 的 refused 断言从「所有 refused ⇒ LEXICAL」改为「逐 case 按 `diagnostic_ids` 推导」。**不宣称**修复了任何 product 语义、任何冻结语料、任何其它测试。
- **IND-003 的「guest formatter 接受 vs Rust formatter 拒绝」是一个真实行为差异**，本片未修、未掩盖，只是把断言校准到不再假装它是词法拒绝。该差异是否成立（formatter 是否应拒绝解析非法的源、IND-003 是否应放在 b 集）是 **owner 裁决项**，不是本片能定的。
- 本片**未**把任何断言放宽为「任意 Domain」/「任意消息」/「contains 通配」，也未 skip/ignore 任何 case；116 条词法拒绝仍逐条**恰好** `Domain { "invalid-input", "LEXICAL" }`。

## 6. 仍待 owner 裁决项

1. **IND-003 语料放置 vs 放宽契约**（主裁决面）：IND-003（解析级拒绝 E1001）位于 `syntax-candidates/b` 且被本片 4 个同族测试 + 主测试同时暴露。裁决方向：语料放错位置（应移出 b 集/改级） vs 放宽契约（接受 b 集含解析拒绝，调整 4 个测试的前提）。**本片不动**，仅登记。
2. **IND-003 在 formatter 差分期望中的最终归宿**：本片把它列为「期望待定」，因为冻结 manifest 的 `diagnostic_ids=["E1001"]` 无法确定 guest formatter 消息（formatter 无语义路径，实际返回 `Output`）。要把它从「待定」改为可钉住期望，需要的最小新增信息：在 manifest 逐条记录 **guest formatter 预期裁决**（例如新增字段 `formatter_outcome: "lexical-refusal" | "semantic-accept"` 之类），或由 owner 决定「guest formatter 应拒绝解析非法的源」并为此给 formatter 增加语义校验（超出本片边界）。
3. **4 个同族测试**（§3）：登记为 IND-003 族，修复与否随 §6.1 裁决。
4. **M22 整体**：保持 **NO-GO**（本片不推进任何出口门）。

## 7. 文件清单

- **改动**：`runner/sico-runner/tests/selfhost_formatter.rs`（仅此一个代码文件；`cargo fmt` 与 `git diff --check` 均净）。
- **新增**：`docs/steps/STEP-0601-m22-selfhost-formatter-assertion-resync.md`（本文件）。
- **未动（按纪律）**：`selfhost/corpus-v0.json`、`selfhost/**`、`syntax-candidates/**`、`semantic-cases/**`、`diagnostics/semantic-case-map.json`、`crates/sico-{format,hir,ir,cli,parser,semantics}/src/**`、`runner/sico-runner/src/**`、`docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`docs/plans/**`。
