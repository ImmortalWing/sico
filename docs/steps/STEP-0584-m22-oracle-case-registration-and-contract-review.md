# STEP-0584 m22 **oracle 补登记 + 契约调查片**：补登 5 条 case id（IND-003/004/005、REV-201/203）到 `diagnostics/semantic-case-map.json`（41→46）、复算并同步 `m1_frontend_bench.rs` 例程常数（实测 78 总数 / 65 可解析 / 13 拒绝，非 STEP-0580 记的 66/78/66）、并产出 IND-003「b 集全部可解析」与 B≠A0 镜像两项契约的**调查与决策材料**（只出材料，不裁决、不实现）

| 项 | 值 |
|---|---|
| 片型 | 有界数据补登记 + 契约调查：只补登记既有条目格式一致的 case id、只同步例程常数与口径注释；**不**改契约、**不**放宽/删断言、**不**改冻结语料（`semantic-cases/`、`syntax-candidates/`、`syntax-mutations/` 内容文件）、**不**动 guest 源 |
| exit | **数据项闭合**：oracle 5 条补登完成且逐条与实测 `diagnostics[0]` 全等；bench 例程按复算值改绿；**契约项只出材料**（§6/§7），两项均标「待 owner 裁决」，**未裁决、未实现** |
| 不翻门 | 不声称任何 gate 翻转；M22 overall 维持 NO-GO |
| 证据等级 | `internal-fixture`：本机 2026-11-07，Windows x64，工具链 1.98.0-x86_64-pc-windows-gnu；`target/debug/sico.exe`（2026-11-07 09:54，晚于 parser 09:52 / semantics 09:53，为当前源码产物）；合并 `7bcdc35` 之后实测 |

## 1. 背景

STEP-0580 把 18 处硬编码普查锚对齐到实测值后，剩余红测根因**不是普查数**，而是三项未决问题（STEP-0580 §7 已登记）：①IND-003 解析拒绝用例违反「b 集全部可解析」契约；②oracle 缺 5 条 case id（IND-003/004/005、REV-201/203 未登记于 `diagnostics/semantic-case-map.json`，当时 41 条）；③B≠A0 镜像契约被 b 集增长打破。本片负责其中**可无争议关闭的数据项**（②：补登记 5 条；另含 bench 例程常数，STEP-0580 §7 明确「超出本片范围未改」），并把契约项（①③）整理成 owner 决策材料。

## 2. 方法（实测口径）

- **case 诊断**：用当前源码产物 `target/debug/sico.exe check --json <file>` 实测每个 b 文件的 `(verdict, diagnostic id, span)`；`diagnostics[0]` 的 `code`/`key`/`message`/`arguments` 即 oracle 条目要满足的值。**不臆造 span**：全部取自实测输出。
- **bench 常数**：按 bench 自身枚举口径（递归 `.sico` 枚举 `syntax-candidates/b` + `syntax-mutations/b`，`parse()` 成功计 success / 恰 1 错误计 failure）**复算**（临时探针 `_m1_probe.rs`，已删除），**未直接抄** STEP-0580 的「66/78/66」。
- **镜像契约**：读 `tools/measure-syntax-candidates.ps1:130-136` 与 `tools/validate-semantic-cases.ps1:125-151` 实现，并用 PowerShell 逐路径比对 A0/B/C 三集合。

## 3. A 项改动

### 3.1 `diagnostics/semantic-case-map.json`（41 → 46 条，+5）

按既有条目格式（`case`/`source`/`code`/`key`/`arguments`/`expected_message`，无 span 字段——span 存于本片 §4 证据）插入。REV-201/203 紧跟 REV-102；新增 IND 组（IND-003/004/005）置于 REV 之后、REC-001 之前（与既有族排序惯例一致：族连续、不跨族）。

| case | source（仓内相对路径） | code | key | arguments | expected_message |
|---|---|---|---|---|---|
| IND-003 | `syntax-candidates/b/indentation/invalid/zero-indent-recovery-declaration.sico` | E1001 | SYNTAX_MISSING_FUNCTION_CLOSE | `{}` | `missing end function` |
| IND-004 | `syntax-candidates/b/indentation/invalid/zero-indent-undeclared-capability.sico` | E4001 | UNDECLARED_CAPABILITY | `{"capability":"stdout"}` | `capability stdout is not declared at this boundary` |
| IND-005 | `syntax-candidates/b/indentation/invalid/zero-indent-stale-result.sico` | E7002 | UNCHECKED_STALE_RESULT | `{}` | `compare loaded.revision before applying loaded.text` |
| REV-201 | `syntax-candidates/b/revision/invalid/unchecked-stale-result-renamed.sico` | E7002 | UNCHECKED_STALE_RESULT | `{}` | `compare loaded.revision before applying loaded.text` |
| REV-203 | `syntax-candidates/b/revision/invalid/cross-payload-guard-stale-result.sico` | E7002 | UNCHECKED_STALE_RESULT | `{}` | `compare loaded.revision before applying loaded.text` |

> 注：IND-003 的 `key` 用目录 key `SYNTAX_MISSING_FUNCTION_CLOSE`（实测 `check --json` 输出），**不是**文件头 `// expect: reject(MISSING_FUNCTION_CLOSE)` 的短名；两者不同，以实测为准。

### 3.2 `crates/sico-cli/examples/m1_frontend_bench.rs`（例程常数）

STEP-0580 §7 记为「58/70/66 过期、实为 66/78/66」，本片**复算推翻其「66 可解析」**：IND-003 是解析拒绝用例，故 b 集 66 文件中只有 **65 个可解析**，另 1 个（IND-003）计入拒绝。复算值（临时探针实测，`parse()` 口径）：

| 量 | 旧值 | **实测新值** | 口径 |
|---|---|---|---|
| `paths.len()`（assert，:20） | 70 | **78** | 66 b + 12 mutations 递归 `.sico` |
| `success` 乘数（assert，:45） | 58 | **65** | 每轮可解析数：66 b − IND-003 + 0 mutations |
| `failure` 乘数（assert，:46） | 12 | **13** | 每轮拒绝数：1（IND-003）+ 12 mutations，每文件恰 1 错误 |
| `"corpus_files"`（:56） | 70 | **78** | 普查：总文件数 |
| `"canonical_files"`（:57） | 58 | **66** | 普查：`syntax-candidates/b` 文件数（**与 `successful_formats`=65 不同**，因 IND-003） |
| `"mutation_files"`（:58） | 12 | **12**（不变） | 普查：`syntax-mutations/b` 文件数 |

> 口径说明：`corpus_files`/`canonical_files`/`mutation_files` 是**普查字段**（文件计数，66+12=78），`successful_formats`/`diagnostic_parses` 是**运行结果字段**（65/13）。两套数因 IND-003 分叉（66≠65、12≠13），注释已写明。`mutation_files` 保持 12 是**真值**（mutation 目录确实 12 个文件），**不是**漏改；`diagnostic_parses`=13 的「13」含 IND-003，故与 `mutation_files` 不同。

## 4. 实测的 5 条 case 真实 `(verdict, diagnostic id, span)`

| case | verdict | diagnostic id | span（实测，1-based line / col，0-based byte） |
|---|---|---|---|
| IND-003 | reject（exit 1） | E1001 / SYNTAX_MISSING_FUNCTION_CLOSE | `line 10 col 1..1`、`byte 282..282`（零宽；`status.syntax=error`、`type_checker=not-run`） |
| IND-004 | reject（exit 1） | E4001 / UNDECLARED_CAPABILITY | `line 13 col 1..25`、`byte 459..483`（`status.syntax=ok`、`type_checker=error`；**共 3 条诊断**，E4001 为首条，另 E4002/E2031） |
| IND-005 | reject（exit 1） | E7002 / UNCHECKED_STALE_RESULT | `line 23 col 1..58`、`byte 635..692`（`status.syntax=ok`、`type_checker=error`；共 1 条） |
| REV-201 | reject（exit 1） | E7002 / UNCHECKED_STALE_RESULT | `line 21 col 3..55`、`byte 448..500`（`status.syntax=ok`、`type_checker=error`；共 1 条） |
| REV-203 | reject（exit 1） | E7002 / UNCHECKED_STALE_RESULT | `line 27 col 5..77`、`byte 611..683`（`status.syntax=ok`、`type_checker=error`；共 1 条） |

补登正确性：逐条比对 `sico check --json` 的 `diagnostics[0]`（code/key/message/arguments）与 oracle 条目，**5/5 MATCH**（§5）。

## 5. 验证

| 目标 | 命令 | 结果 |
|---|---|---|
| bench 例程 | `cargo run --offline -p sico-cli --example m1_frontend_bench -- 1` | 绿：`diagnostic_parses:13, successful_formats:65, corpus_files:78, canonical_files:66, mutation_files:12`，迭代断言全过 |
| oracle 对拍（5 条） | `sico check --json` × 5 + PowerShell 比对 code/key/message/arguments | **5/5 MATCH**（IND-003/004/005、REV-201/203 全等） |
| 未受影响 oracle 测试（4 组） | `cargo test --offline -p sico-semantics --lib tests::{numbers_and_nominal,match_and_result,capability_and_component,resource_task_and_stream}_cases_match_the_registered_oracle` | **4/4 绿**（前缀不受 IND/REV-201/203 影响） |
| 受影响 oracle 测试（cli） | `cargo test --offline -p sico-cli --test cli check_matches_all_58_semantic_oracles -- --exact` | **仍红**，但根因已从「oracle 缺 5 条 panic」变为 **IND-003 的 `type_checker=="not-run"` vs 断言 `"error"`**（cli.rs:136）——契约项，非数据项（§9） |
| 受影响 oracle 测试（semantics revision） | `cargo test --offline -p sico-semantics --lib tests::revision_cases_match_the_registered_oracle` | **仍红**，根因 = `expected.len()` 断言 2→4（lib.rs:4826）——补登 REV-201/203 使其过期，属**普查计数**，位于本片可动范围外的热文件（§9） |
| `remaining_b_groups_do_not_receive_unowned_diagnostics` | `cargo test --offline -p sico-semantics --lib tests::remaining_b_groups_do_not_receive_unowned_diagnostics` | **仍红**，根因 = IND-003 使 `analyze().unwrap()` 返回 `Err(Lower(Syntax{count:1}))`（lib.rs:4903）——契约项（§9） |
| fmt | `cargo fmt -p sico-cli -- --check` | exit 0（净） |
| diff 检查 | `git diff --check` | exit 0（净） |

## 6. 契约调查 1：IND-003 与「b 集全部可解析」

**契约的权威出处（可执行形式，无单条计划条款明示，隐于各测试全量假设）**：
- `crates/sico-parser/src/lib.rs:1205`（`all_b_canonical_cases_parse_losslessly`）：对 b 每文件 `assert!(parsed.is_success())`。IND-003 → `MissingFunctionClose`（1 错误）→ 红。**此为「b 集全部可解析」的核心断言**。
- `crates/sico-format/src/lib.rs:306`（`all_b_cases_preserve_ast_shape_and_are_idempotent`）：`parse(&original).ast().unwrap()` → IND-003 `ast()==None` panic。
- `crates/sico-hir/src/lib.rs:398-420`（`all_b_sources_lower_deterministically_with_complete_source_maps`）：lower 依赖 parse → IND-003 红。
- `crates/sico-semantics/src/lib.rs:4903`（`remaining_b_groups_do_not_receive_unowned_diagnostics`）：`analyze(&source).unwrap()` → `Err(Lower(Syntax{count:1}))`（本片实测 panic 于 4903）。
- `crates/sico-cli/tests/cli.rs:136`（`check_matches_all_58_semantic_oracles`）：对 reject 用例断言 `type_checker=="error"`，IND-003 实为 `"not-run"`（本片实测 panic 于 136）。
- 语料定义：b 集 = `syntax-candidates/b` 递归 `.sico`，`// syntax: B`、`// expect: accept|reject(KEY)`；A0/b 的 reject 惯例是**语义拒绝**（parse 成功、语义失败），IND-003 是**解析拒绝**（parse 失败），属不同类别。

**判定：IND-003 是「数据放错位置」还是「契约表述过宽」？** —— IND-003（`zero-indent-recovery-declaration.sico`）是一个**刻意**的解析拒绝用例：文件头 `// expect: reject(MISSING_FUNCTION_CLOSE)` + `// note:` 明示其测试「列 0 声明关键字在函数体内是 recovery declaration ⇒ 外层函数报未闭合 E1001」。它**不是误放**的坏数据，而是一个**有意的恢复行为测试**；但它被放进了「全部可解析」的 b 集（该集此前 58 个文件全部可解析），**冲突的是「b 集全部可解析」这个契约假设**（契约表述过宽：把「b 集 = 可解析语义用例」错当成「b 集 = 全可解析」）。同时，IND-003 的**类别**（解析拒绝）与 b 集/A0 的 reject 惯例（语义拒绝）不同，而 `syntax-mutations/b` **已是**解析拒绝语料（如 `MUT-001-missing-function-close.sico` → E1001），故 IND-003 更接近 mutation 类别。**结论：既有「数据错位」成分（类别错置），也有「契约过宽」成分（未预期解析拒绝用例）——两者都需 owner 裁决，本片不裁。**

**两方案后果对比**：

| | 方案 A：移出 b 集（数据） | 方案 B：改契约（放宽「b 集可解析」） |
|---|---|---|
| 做法 | 把 `zero-indent-recovery-declaration.sico` 移入 `syntax-mutations/b/`（或拒绝语料目录）；b 集回到「全可解析」 | 更新 parser/format/hir/semantics/cli 五处断言以**允许** b 集含带 `// expect: reject` 标记的解析拒绝用例（或对 IND-003 特判） |
| 动冻结语料 | **是**（`syntax-candidates/b/**` 内容文件）→ 需 owner | 否（语料不动） |
| 下游影响 | b 普查 **66→65**，mutation **12→13** ⇒ 需**再同步** STEP-0580 刚设的 66 普查锚（7 处 crate + 2 ps1 + bench）至 65/13；`semantic-case-map.json` 的 IND-003 条目需**改 source**（或随迁）；oracle `// case: IND-003` 随文件移动 | 五处测试改为「b 集可解析或带 reject 标记的解析拒绝」⇒ **「b 集全部可解析」保证被削弱**：b 集不再是纯可解析语料，而是「可解析 + 设计拒绝」混合；下游门（M1 前端全量解析、M2/M3 形状保持、M22 语料基准）对 b 集的可解析假设需**逐处重审** |
| 好处 | 恢复「b 集全部可解析」不变量，IND-003 归入其天然类别（mutation/解析拒绝） | 保留 IND-003 在 b 集的恢复行为测试覆盖，不动语料 |
| 代价 | 动冻结语料（owner）+ 一次普查重同步；IND-003 的「recovery declaration」语义在 mutation 里**语义标签改变** | 契约变宽，五个 crate 的测试语义分叉（reject 用例不再统一「语义拒绝」）；后续 b 集新增用例须再区分可解析/设计拒绝 |

## 7. 契约调查 2：B≠A0 镜像

**镜像检查实现（比较哪两集合）**：
- `tools/measure-syntax-candidates.ps1:130-136`：`$referencePaths` = A0（`semantic-cases`）相对路径（`group/valid|invalid/<file>.sico`，排序后按 `\n` join）；对 B、C 各取相对路径集合，**与 A0 做字符串全等**。不等则 `throw "$syntax paths do not mirror A0"`。→ 比较的是**相对路径集合的身份（exact path-set identity）**，不是文件内容、不是计数。
- `tools/validate-semantic-cases.ps1:125-138`（正向）：对每个 A0 case，要求 B/C 同相对路径存在且 `case`/`syntax`/`expect`/`semantics` 头全等；`:140-151`（反向）：B/C 文件数 == manifest `case_count` 且每个 B/C 路径 ∈ A0 已登记路径集。→ 同样比较**路径集合身份**，外加头部元数据全等。

**为何 b 集含语法专属组而 A0 不含**：本片实测（PowerShell 逐路径比对）——A0=58、B=66、C=58；**A0 的 58 条路径全部在 B 中（0 缺失）**，B 恰多 **8 条**：5 条 `indentation/`（IND-003/004/005 + `zero-indent-mid-statement` + `zero-indent-return`）+ 3 条 `revision/`（`cross-payload-guard-stale-result`、`unchecked-stale-result-renamed`、`stale-load-ignored-renamed`）；C 与 A0 **完全一致**（0 多 0 缺）。这 8 条是合并后新增（STEP-0279..0283 冻结的 +8），**只加到了 B**，A0/C 未镜像 ⇒ B ⊋ A0（严格超集），镜像被打破。`indentation` 是 B 语法专属组（列 0 / 缩进语义），A0/C 无对应。

**镜像失败的实际后果**：
- **计数不符**：B=66 vs manifest/基准 58 ⇒ `validate-semantic-cases.ps1:142` 与 `measure-syntax-candidates.ps1` 的 `files.Count != expected` 检查红（后者计数已由 STEP-0580 修到 66，但路径身份检查仍红）。
- **语义覆盖不符（路径集合层面）**：不是「A0 行为在 B 未覆盖」（A0 ⊂ B，0 缺失，全部覆盖），而是**反向**：B 有 8 个 case（indentation 恢复/缩进语义、revision 守卫变体）**没有 A0/C 镜像**。故镜像契约的「A0==B==C 同 case 集、三语法风味互镜像」保证失效——B 的 case 空间 ⊋ A0/C。
- **本质**：镜像契约断言的是**三语料同 case 集**；失效源是 B 新增 8 条未镜像到 A0/C。**既是计数问题也是覆盖不对称问题，根因是路径集合不再恒等。**

**两方案后果对比**：

| | 方案 A：恢复镜像（A0/C 补镜像） | 方案 B：收窄镜像契约 |
|---|---|---|
| 做法 | 把 8 条 b-only case 镜像到 `semantic-cases/`（A0）与 `syntax-candidates/c/`（C），各按其语法风味改写（A0 无 `// syntax:`、C 为花括号风格） | 把镜像从「A0==B==C 恒等」改为「A0 ⊆ B、A0 == C」（单向），或把语法专属组（indentation）从镜像范围排除 |
| 动冻结语料 | **是**（`semantic-cases/`、`syntax-candidates/c/` 内容文件）→ 需 owner | 否（语料不动） |
| 下游影响 | A0/C 普查 **58→66** ⇒ 重同步所有「A0/C=58」锚（`measure-syntax-candidates.ps1`、`validate-step-0016`/`0068` 的 A0 口径、语义 oracle 相关断言）；A0 新增 8 条须满足「无 `// syntax:`」与 valid/invalid 结构；**indentation 是 B 专属语义**，A0/C 改写其语法编码**意义存疑** | `measure-syntax-candidates.ps1:133` 与 `validate-semantic-cases.ps1:142,147` 改为允许 B 超集/排除 indentation；「三语料同 case 集」保证降为「A0 与 C 恒等、B 为超集」；B 的 8 条 b-only 成为 B 专属测试，不再要求 A0/C 覆盖 |
| 好处 | 恢复三语料同 case 集镜像不变量 | 不动语料；承认 indentation 是 B 专属语法；8 条 b-only 保留为 B 专属覆盖 |
| 代价 | 动冻结语料（owner）+ 重同步 A0/C 普查；indentation 的 A0/C 编码可能无意义（B 专属语义） | 镜像保证变弱（B 可为超集）；后续 A0 新增用例须同时入 B/C，B 新增用例可只入 B ⇒ 审计口径变化 |

## 8. 待 owner 裁决项

1. **IND-003 归属**：移出 b 集（§6 方案 A，动冻结语料 + 普查重同步）vs 放宽「b 集全部可解析」契约（§6 方案 B，改五处测试断言）。**本片未裁决、未实现**。
2. **B≠A0 镜像**：恢复镜像（§7 方案 A，A0/C 补 8 条）vs 收窄镜像契约（§7 方案 B，允许 B ⊋ A0 或排除 indentation）。**本片未裁决、未实现**。
3. **lib.rs:4826 普查计数**：`revision_cases_match_the_registered_oracle` 的 `assert_eq!(expected.len(), 2)` 因补登 REV-201/203 过期（实为 4）。一行普查重同步，**位于本片可动范围外的热文件**（`crates/sico-semantics/src/lib.rs`，其它代理在用）⇒ 未改，留 owner/后续片。
4. **IND-004 多诊断**：IND-004 实测**共 3 条诊断**（E4001/E4002/E2031），cli.rs:127 对 reject 用例断言 `len()==1` ⇒ 即使 §8.1 解决，IND-004 仍红（除非特判或改文件）。**新发现**，与 §6/§7 正交，需 owner 裁决（IND-004 首条为 E4001 是「oracle 语义」，但测试断言「恰 1 条诊断」更严）。

## 9. 诚实收窄（仍红项，均为非普查、非本片数据项）

| 仍红目标 | 根因（实测） | 性质 |
|---|---|---|
| sico-cli `check_matches_all_58_semantic_oracles` | IND-003 `type_checker=="not-run"` vs 断言 `"error"`（cli.rs:136）；另 IND-004 3 条诊断 vs `len()==1`（cli.rs:127） | 契约（§6/§8.1/§8.4） |
| sico-semantics `revision_cases_match_the_registered_oracle` | `expected.len()` 2→4（lib.rs:4826） | 普查计数（§8.3，可动范围外） |
| sico-semantics `remaining_b_groups_do_not_receive_unowned_diagnostics` | IND-003 `analyze()`→`Err(Lower(Syntax))`（lib.rs:4903） | 契约（§6/§8.1） |
| sico-parser / sico-format / sico-hir b 集测试 | IND-003 解析拒绝（parser:1205 / format:306 / hir:398-420） | 契约（§6/§8.1） |
| `tools/measure-syntax-candidates.ps1` / `tools/validate-semantic-cases.ps1` | B ⊋ A0 镜像（8 条 b-only） | 契约（§7/§8.2） |
| `tools/validate-step-0021.ps1:13` | 硬编码 `70/58/12`，与其快照 `tests/performance/m1-frontend-windows-release.json` 的 `66/54/12` **预先不一致**（本片未动，超出可动范围） | 既有不一致，非本片引入 |
| runner `selfhost_formatter` / runner 相关 | 工具链 ICE + gcc/dlltool 不在 PATH（STEP-0580 §5 已登记） | 环境阻塞 |

**本片已闭合的数据项**：oracle 5 条补登（41→46）、bench 例程常数（78/65/13 + 普查 78/66/12，复算非抄数）。**本片未闭合**：§6/§7 契约项（只出材料）、§8 待裁决项、§9 仍红项。

## 10. 边界

- 本片**未**改：`docs/steps/README.md`、`docs/STATUS.md`、`docs/ROADMAP.md`、`docs/handoff-m22.md`、`AGENTS.md`、`AGENT_GOAL.md`、`selfhost/`、冻结语料目录内容（`semantic-cases/`、`syntax-candidates/b|c/**`、`syntax-mutations/**` 内容文件）、`crates/sico-ui-renderer-d2d/**`、`runner/sico-runner/src/ui.rs`、runner provenance/limits、`crates/sico-package/src/provenance.rs`、`tools/*.ps1`、`crates/sico-semantics/src/lib.rs`、`crates/sico-{parser,format,hir}.src`。
- 本片**改**：`diagnostics/semantic-case-map.json`、`crates/sico-cli/examples/m1_frontend_bench.rs`、`docs/steps/STEP-0584-m22-oracle-case-registration-and-contract-review.md`。
- 临时探针 `crates/sico-cli/examples/_m1_probe.rs` 已删除（未提交、未留存）。
- 未 commit/push；保留所有既有改动（git status 既有 M/?? 文件未动）。
- 未运行全 workspace（仅 `-p sico-cli` / `-p sico-semantics` 定向目标 + 两个例程）；未运行 runner（环境 ICE，STEP-0580 §5 已登记）。
