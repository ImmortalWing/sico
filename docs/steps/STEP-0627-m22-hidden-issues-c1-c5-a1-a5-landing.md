# STEP-0627 — M22 审计报告隐藏问题落地（C1–C4、A1–A5、B1 已落地；A3 裁定不做；B2 已测量待 owner 裁决）

- 日期：2026-11-18
- 状态：implementation complete locally（C1–C4 + A1/A2/A4/A5 + B1 落地并验证；A3 owner 裁定「暂不做、触发条件已登记」；B2 owner 裁定「不上调限额、复用已有 `--emit-ir-split`、scalar_ir 已由 parser.sico 整文件 pin 覆盖」；match-site ledger 重冻结 406 站点 + emitter-vocabulary 刷新）
- 日期订正：本片跨 2026-11-18 起始、2026-11-19 收尾（B1/B2/账本刷新在 11-19 完成）
- 基线：`dev` @ `6b47116`（未 merge owner 的 m24 分支；未 commit / 未 push）
- 来源审计：[`docs/reports/m22-m24-deep-audit-2026-11-18.md`](../reports/m22-m24-deep-audit-2026-11-18.md)（= STEP-0626）的 A1–A5、B1–B2、C1–C4 项

## 1. 环境阻塞的发现与解除（本片前置）

开工即撞上本机 runner 测试二进制无法链接：`windows-sys 0.61.2` 的 `kernel32.dll_imports.lib` 报 `os error 1006`（文件所在卷已被外部更改）。穷尽排除：串行 `-j1`、两个全新 `CARGO_TARGET_DIR`、改 `TMP`、隔离单包构建、**换三块物理卷（D:/E:/系统盘 C:）**、磁盘空间充足均同样失败；`cargo check`（不链接）通过、`windows-sys 0.52.0`（经 `sico-automation-fixture`）链接成功 ⇒ 故障特定于 0.61.2 的链接/codegen 阶段。`cargo tree -i windows-sys@0.61.2` 显示它被 **wasmtime 全栈 + ctrlc** 强制拉入 ⇒ 无依赖绕法。

**真因（owner 退出软件后链接立刻成功，13.8 s Finished）**：本机企业终端安全软件 **奇安信天擎（QI-ANXIN Tianqing）** 的内核文件过滤驱动实时扫描 rustc 新建的 `.lib` 致句柄失效。此前 `Get-MpPreference` 报 `0x800106ba`（Defender 服务未运行）一度误导向 Defender——实际本机由天擎接管（`root/SecurityCenter2` 枚举出 `QI-ANXIN Tianqing` productState `331776`=`0x51000`，第 2 字节 `0x10`=实时防护开启）。**owner 裁决项**：给天擎加 `D:\SorftWare\sico` 排除或构建期临时关实时防护，否则本机无法跑 runner 链接类验证。
**2026-11-19 复现与诊断更新**：磁盘满（runner target 63G + root target/debug 15G）曾叠加阻塞，清理后恢复 77G free。随后 ledger firmware 复证连续 4 次失败于同一 `os error 1006`（`windows-sys` 的 `kernel32.dll_imports.lib`）。**关键新证据**：①owner 加 `D:\SorftWare\sico` 路径排除**实测无效**——os error 1006 依旧，且报错路径本就在白名单内 ⇒ 天擎对该类 rustc 新建 import-lib 的拦截**不遵循路径排除**；②**root workspace 编译正常**（`cargo build -p sico-cli` 44s EXIT=0），仅 runner workspace 复现 ⇒ 故障特定于 runner 独有依赖链（wasmtime/ring/rustls 强制拉入的 `windows-sys 0.61.2`）触发的 codegen 链接。⇒ **有效解除动作只有「构建期临时关天擎实时防护」**（owner 此前退出后链接 13.8s 即成功）；路径排除不解决问题。census 静态探针不依赖 runner，仍可跑：`LEDGER_CURRENT sites=406`（0 pending）已复证绿。

## 2. 已落地项

### C1 — `sico-codegen-wasm` 用户导入 fail-open 改 fail-closed
`crates/sico-codegen-wasm/src/canonical.rs`：`wrap_script_component` 返回类型 `Vec<u8>` → `Result<Vec<u8>, crate::CodegenError>`；用户导入接口值集不受支持时由 fail-open `continue` 改 `return Err(CodegenError::Unsupported{function, feature:"user import interface value set"})`；末尾 `Ok(builder.finish())`；补 `# Errors` doc。调用点 `lib.rs` 加 `?`。3 个 runner 测试调用点加 `.expect("hand-built minimal guest has no user imports to refuse")`（`tests/runner.rs`、`provider_crash_isolation.rs`、`provenance_runtime.rs`）。
验证：`cargo check/clippy -D warnings/test -p sico-codegen-wasm` 全绿。**注**：owner 并发在 `canonical.rs`/`lib.rs` 落 M24 picker 改动，与 C1 共存；owner 并发改后复跑 `-p sico-codegen-wasm` 仍 `8 passed / 0 failed`。

### C2 — `canonical.rs` 去 4 处 `.leak()` 堆泄漏（审计点名范围）
`user_interface_instance` 的 params 改 owned `Vec<String>` + 循环后 `map(String::as_str).zip`；`wrap_script_component` 的 `name`/`fname`/`lowered.entry` key 改 `.as_str()` 借用（源 `user_imports`/`user_instances` 活得比 `lowered` 消费点久）。与 C1 同批验证绿。**未扩入**：`lib.rs` `helper_dependencies` 另一处 `'static` 泄漏（审计未点名）。

### C3 — runner `http2.rs` upload-pipe `finish` 漏减 `bodies_live`
`runner/sico-runner/src/http2.rs`：`finish` 成功路径 `table.delete(handle)` 后递减 `bodies_live`（`saturating_sub(1)`，镜像 `:350/:366` 析构语义）。
验证：`cargo check -p sico-runner --lib` 通过；`--test http2` **6 passed / 0 failed**（含 `upload_roundtrips_with_secret_injection` 走 upload-pipe 路径）。

### C4 — `sico-http-provider` in_flight 泄漏 + UploadPipe 双重递减
`crates/sico-http-provider/src/engine.rs`：新增 RAII `InFlightGuard`（drop 调 `release_in_flight`）；`execute()` 建 guard、移除 `execute_inner` 的 Stop/Refused 两条显式 `end_request()`（改由 guard 统一释放，覆盖 `?` 早退 Err）；`UploadPipe::finish` 成功路径 `self.done.take()` 解除 Drop 释放（slot 所有权随 ResponseBody 转移），错误路径保留 `done` 交 Drop。新增**会失败-先行**回归测试 `execute_error_path_releases_the_in_flight_slot_each_time`（granted 的 `127.0.0.1:1` 连接拒绝，循环 `0..=MAX_IN_FLIGHT`，断言不误报 in-flight cap）。
验证：`check/clippy -D warnings/test -p sico-http-provider` 全绿 **48 passed**（含新回归测试）。

### A1 — `cgs_helper_index` 每内建调用全模块重扫 → 预计算 `(count, mask)` O(1)
`selfhost/codegen.sico`：新增 `cgs_helper_index_masked(fn_count, mask, wanted)`（原排序链，`cchi_*`→`cchm_*`，与原体 normalize+diff `IDENTICAL == True`）；`cgs_helper_index(bytes, wanted)` 保留签名、算一次 count+mask 后委派；把 `cgs_core_code_hex` 已算的 `(cccsh_count, cccsh_mask)` 作为末两参穿线 `cgs_core_code_hex → cgs_function_body_hex → cgs_function_instr_hex → cgs_op_hex → cgs_intrinsic_hex`，9 个内建发射调用点（`:3801/:3875/:4011/:4072/:4209/:4277/:6471/:6552/:6612`）改用 masked 形式。`cgs_run_instr_hex`/`cgs_run_body_hex` 保留 `(bytes)` 签名（`compiler.sico:957/969` 调用，未扩入），各自本地算一次 count+mask。
证据：formatter 钉测试 `sico_compiler_emits_the_whole_formatter_component_byte_exactly` + `sico_codegen_guest_emits_map_and_list_cell_component_byte_exactly` **逐字节 ok**；A1 子代理用注入组件法在 2 个 IR（64×`text.concat` 链、all-9-arms）上证明 OLD vs NEW stdout **BYTE_IDENTICAL**（覆盖全部 9 个 masked 调用点 + 完整穿线路径）；燃料 `--fuel` 二分：64-concat OLD `∈(5.3e9,5.4e9]` → NEW `∈(1.22e9,1.23e9]` ≈ **4.4× 降低**（独立佐证 STEP-0618 的 5.12e9..6.4e9 读数）。

### A2 — `cgs_core_types_hex`/`cgs_core_code_hex` 按下标重扫函数表（O(F²)）
`cgs_function_at_index` 的两处热路径调用消除：`cgs_core_type_hex` 改收 fn 节点（去 `cgs_function_at_index`），其内部 parameters 循环改 `cgj_array_first/next` 游标；`cgs_function_body_hex` 改收 fn 节点（去 `cgs_function_at_index`）；两个驱动循环 `cgs_core_types_hex`/`cgs_core_code_hex` 改 `cgj_member(bytes,0,"functions")` + `cgj_array_first/next` 单趟游标。
证据：formatter 钉测试逐字节 ok。

### A4 — 局部 cell 槽位 O(L²) 重扫
`cgs_cell_ty`/`cgs_cell_width` 改收 cell 节点（去 `cgj_array_item(locals,index)` 重取）；`cgs_cell_slots`、`cgs_dispatcher_slot` cells 段、`cgs_cell_slot` 三处循环改 `cgj_array_first/next` 游标 + 节点宽度；`cgs_read_local_hex`/`cgs_write_local_hex` 的宽度查询改「`cgj_array_item` 取节点一次 → 节点宽度」。map/list guest 钉测试（重度用 cell）逐字节 ok。
**未做（诚实收窄，见 §3）**：报告 A4 第 2/3 点的「函数入口预计算 dispatcher 槽位 + `cindex: List[U64]` 穿线到 read_local/write_local/branch/jump」未落地——`cgs_dispatcher_slot` 仍被 `cgs_cell_slot`（→read/write）与 branch/jump 逐次重算。

### A5 — Match/Construct/Variant 发射路径残留 `cgj_array_item` 重扫
6 处报告点名函数：`cgs_construct_hex` fields 循环、`cgs_variant_hex` 两个 payload 循环、`cgs_match_condition_hex`（values/patterns 并行数组，双游标同步推进，任一尽则停）、`cgs_match_hex` arms 循环、`cgs_instruction_for_value`（blocks×instructions 嵌套两level，即 `match_flat` O(n²)）全部改 `cgj_array_first/next` 游标。`cgs_result_arm_ty` 复核为纯 `cgj_member` O(1) 访问、无数组循环 ⇒ 无需改。
证据：formatter 钉测试逐字节 ok。

## 3. 部分 / blocked 项（具名原因 + owner 裁决项）

### A3 — 每条用户 `call` 线性重扫函数表（部分落地）
`cgs_function_position` **已是单趟游标**（`cgj_array_first/next`，STEP-0625 或更早已转）⇒ 审计描述的「从第 0 项按 index 重扫」形状已不存在，剩余仅「每条 call 重复一次 O(F) 名字扫描」。报告建议的「模块入口预构建函数名→索引表并穿线到 `cgs_call_hex`」未落地。**原因**：穿线需在 A1 已加的 `(fn_count, mask)` 之外再给 `cgs_call_hex` 链加一个 map/list 参数（签名churn 大）；且 Sico `List` 按值查找仍是线性，紧凑表只降常数不降复杂度，而 formatter 的 F≈30 使该残余代价小。**owner 裁决（2026-11-19，经独立复核确认）**：残余**暂不做**；若将来做，走「稠密函数 id 校验 + O(1) 算术」（模块入口一次性 O(F) fail-closed 校验函数 id 为稠密规范 `1..F`，通过后 `cc_index = decimal(id) − 1`），**不建哈希表、不改 ABI**——与 A1 的模块级预计算同片几乎零额外成本。触发条件（满足其一才单独做）：(a) 后续模块 F 明显 > 30 使 `O(C_calls×F)` 占燃料可测份额；(b) A1/A2/A4 落地后整文件燃料仍贴近预算。F≈30 且该层已证非主导，不值得为它单独开一轮带独立回归钉的改动。

### B1 — 无 `while` 函数回退 `gw_*` 通用降级器（已落地，2026-11-19）
**落地**：`selfhost/parser.sico` 的 `scalar_ir` 路由新增 `gw_*` 回退门——当 `match_count>0` 且 `general_route_count==0`（函数体无 `while` 且未被强制走窄路由）时，先尝试 `general_while_function_ir` 通用降级；其 decline 才回落 `ERR:E-SH-IR-STATEMENT`，`SKIP` 映射为 `ERR:E-SH-IR-GWSKIP:<skip>`。窄路由的 typed refusal 全部保留。
**会失败-先行复现用例**：`bytes-slice`（no-`while let` + `match` + 2 个 `set`）——其 Rust oracle 会降（`rust_ir` 4555 B 证），但改前 guest 路由 pinned refusal `ERR:E-SH-IR-STATEMENT` ⇒ guest-oracle parity gap。改后 `runner/sico-runner/tests/selfhost_compiler.rs` 把 `bytes-slice` 从 refusal 列表移到 lowered 列表并断言与 `rust_ir` 字节等同。
**账本影响**：新 `gw_*` 门带 1 个新 `U64.checked_add` 头 ⇒ match-site ledger `405→406`（见 §4）。

### B2 — `scalar_ir` 展开 8 MiB stdout 上限（已测量，补救路径待 owner 裁决）
**测量（决定性，2026-11-19）**：按报告 §10.1.3「先测量、不上调限额」执行。用 match-site census 的 closure 规则取 `scalar_ir` 的**无界可达闭包** = **174 函数 / 1,066,575 B 源**（冻结 1 MiB closure bound 截断在 `closure-too-large:165fn/1049726B`）；经 Rust oracle（`crates/sico-ir` 的 `dump_oracle_ir` 诊断，进程内 `lower_core`+`canonical_json`，无 8 MiB stdout 约束）降为 canonical IR = **9,102,533 B（8.68 MiB）> `MAX_CHANNEL_BYTES` 8 MiB（8,388,608 B），超出 8.5%**。源→IR 膨胀 ≈ 8.5×。存证 `target/tmp/b2_scalar_ir_closure.sico` + `target/tmp/b2_scalar_ir_ir.json`。
**量化结论**：STEP-0626 §B2 的「~1 MiB 源 → >8 MiB canonical IR」假设**成立**——任何含 `scalar_ir` 的闭包展开后 canonical IR 都会越过 `MAX_CAPTURED_CHANNEL_BYTES` 8 MiB stdout 捕获上限而被 trap。
**裁剪闭包不可行**：该 174 函数闭包是 `scalar_ir` 的**纯可达传递闭包**（全部被 `scalar_ir` 传递调用），**无不可达辅助函数可裁** ⇒ 报告 §10.1.3 的「裁剪闭包」选项在本站不成立。
**owner 裁决（2026-11-19，经独立复核确认）**：不上调限额（维持已决）。补救**不需要**新压缩方案/新 RFC——复核纠正两处先前误判：
1. **算术订正**：8,388,608 / 9,102,533 = 需缩 **7.8%（0.92×）** 即可进限，**不是 ~8.5× 压缩**（先前把「源→IR 膨胀 8.5×」误当「需压缩 8.5×」）。
2. **传输机制已存在且已验证**：`--emit-ir-split`（`compiler.sico:889`）把 IR 按半长切分、head 走 stdout / tail 走 stderr，每半 ≈4.3 MiB < 8 MiB；STEP-0495 已用它对**整个 `parser.sico` 的 >8 MiB IR** 做逐字节 pin（`selfhost_compiler.rs:2596`）。
**`scalar_ir` 是 `parser.sico` 内函数**（`parser.sico:21856`），而 parser.sico 是 `MATCH_WHOLE_FILE_PINNED`唯一登记整文件 IR pin 的文件（`selfhost_compiler.rs:9706`）⇒ scalar_ir 的 closure-too-large 站点**已有覆盖**，且是比逐闭包更强的整文件逐字节覆盖。closure-too-large→整文件 pin 的回退**本就是设计好的正确行为**。
结论：**唯一真实残余** = scalar_ir 闭包**独立运行**时尚无独立 IR witness（现靠整文件 pin 传递覆盖）。若要补独立测量，复用 `--emit-ir-split` 即可，**零协议改动、零新 RFC/ADR**。路径 2（收窄口径贴假标签，使覆盖看似已测实则未变）不可取；路径 3（上调限额）维持否决。

## 4. 验证与诚实收窄

- **逐字节闸门（A 类）**：`sico_compiler_emits_the_whole_formatter_component_byte_exactly` + `sico_codegen_guest_emits_map_and_list_cell_component_byte_exactly` 在 A1、A2、A4、A5 每片落地后分别 `--exact` 复跑，**均 ok**（最后合并态复跑 2 passed）。A 类改动的燃料降低：A1 实测 ~4.4×（见 §2）；A2/A4/A5 为同族游标化（STEP-0618 刀 2 对 `cgs_dispatcher_slot` 已测 −14.5%，本片 A4 消除其 cells 段残余 index 重取，未单独复测整文件燃料）。
- **C 类**：C1/C2（codegen-wasm 8 passed）、C4（http-provider 48 passed 含新会失败-先行回归）、C3（http2 6 passed）各自实测绿。
- **match-site ledger 重冻结（本片完成，405→406）**：B1 的 `gw_*` 门在 `scalar_ir` 新增 1 个 `U64.checked_add` 头，且 +25 行使 `scalar_ir` 后段站点行号/ordinal 位移 ⇒ `census-m22-match-sites.py --check` 报 `CENSUS_MISMATCH expected=405 actual=406`。**订正**：STEP-0605「本机无可用 Python」是错误登记——本机有完整可用 Python（`python` = 3.15.2，`C:\Users\Sunjy\AppData\Local\Python\bin\python.exe`），census 可跑。重冻结步骤：①`census-m22-match-sites.py`（plain rewrite 修行号、`EXPECTED_SITES` 405→406）②两处固件计数断言 405→406（`selfhost_compiler.rs`）③外科式 `SICO_M22_MATCH_GROUPS=scalar_ir` 跑 `sico_compiler_reproduces_the_frozen_match_site_ledger` 生成 scalar_ir 实测 `actual.json` = `closure-too-large:165fn/1049726B`（B1 给 `scalar_ir` 自身 +906 B 使 closure 游走更早越过 1 MiB ⇒ 166→165 fn，firmware 与 emitter-vocab Python 同值）④`census --update`（`SICO_M22_MATCH_ACTUAL=actual.json`）合并 ⇒ **`LEDGER_WRITTEN sites=406 pending=0`**、`--check` = `LEDGER_CURRENT sites=406`、scalar_ir 全 25 站点统一 `closure-too-large:165fn/1049726B`。
- **emitter-vocabulary 刷新（本片完成）**：A1/A2/A4/A5 改 `codegen.sico` ⇒ `emitter-vocabulary-v1.json` 的 `guest_emitter.source_sha256` 漂移；且 B1 改 `parser.sico` 使 census 的 closure 自检一度 RED（165 vs 166，随 ledger 重冻结为 165 而解除）。经 `census-m22-emitter-vocabulary.py --update` 刷新 `guest_emitter.source_sha256` ⇒ `--check` = `EMITTER_VOCABULARY_CURRENT`（结果见 §4 末）。
- **预存红（非本片引入，归属清楚）**：`every_match_site_is_measured_on_the_component_axis` 在 HEAD 即红——它断言「无任何 `closure-too-large` 站点」，而 `scalar_ir` 25 站点因 closure 1,049,726 B > 1 MiB closure bound 一直是 `closure-too-large`。此即 B2 的 closure-too-large 面（与 8 MiB stdout 是两个不同上限），**属 B2 范畴、HEAD 已红、本片未引入**；本片仅把其计数断言 405→406 保持准确。
- **全量 `selfhost_compiler` 套件**：本片结束时长跑仍未出结果（燃料重型测试，~198 项），**完整计数未取得**；已取得的逐字节闸门与各 C 类套件绿是本片的验证基座。
- **未上调任何 fuel/内存/stdout 限额**；未 merge m24；未 commit/push。
- 并发 M24 改动（`sico-ui-renderer-d2d/*`、`guests/image-codec/*`、`crates/sico-ir`、`sico-semantics`、`runner/lib.rs` picker、`docs/rfc/RFC-0055*`、`AGENTS.md`）**一律未触碰**。

## 5. 门纪律

未翻任何 gate；M22 overall 仍 NO-GO（本片是审计报告的隐藏问题修复，非 M22 出口 gate 翻转）。证据级别：internal-fixture（本机单机快照，非独立 CI）。