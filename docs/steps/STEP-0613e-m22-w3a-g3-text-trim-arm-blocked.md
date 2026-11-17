# STEP-0613e — M22 W3-a G3 首臂 `sico.text.trim`:落地但字节等同**未证**(被 co-gap 阻塞)

> 状态:**implementation partial + honest block(诚实记录)** —— `sico.text.trim` 发射臂已落地、编译通过、guard-safe(fail-closed);**但字节等同未证**——任何 run fixture 都需 `utf8_decode` + `text.encode` 把 Text 透过 Bytes stdout,而**这两者在当前上下文都拒** `unsupported codegen source shape`,故 text.trim 无法隔离验证。**未翻任何 gate、未 commit/push。**
> 日期:2026-11-10。基线 HEAD `e93f59ad`。续 [STEP-0613d](./STEP-0613d-m22-w3a-g2-scoping-correction.md)。

## 1. 本片目标

按 STEP-0613d 的 G3 待办,做**第一个最小臂** `sico.text.trim`(比 map 系简单),建立"运行时 helper 发射臂"模式 + 隔离字节验证 + 去风险自举前沿。

## 2. 实现(委托模式,循 is_utf8 先例)

`selfhost/codegen.sico` 新增(工作树 sha `5042ad42…` → `3f58a479…`):
- `cg_text_trim_helper_body_hex()`:text.trim 运行时 helper 体(对齐 oracle `stdlib.rs:411 emit_trim`)。
- `cgs_text_trim_hex(bytes, run, instr, data, result0, vindex)`:intrinsic 臂(对齐 oracle `lib.rs:4585-4595` call-site:`LocalGet(src_ptr)/LocalGet(src_len)/Call(helper)/LocalSet(len)/LocalSet(ptr)`)。
- `cgs_intrinsic_hex` 一行分派(`cg_is_word(cih_name,"sico.text.trim")` → `cgs_text_trim_hex`,循 `cgs_is_utf8_hex` :3851-3853 委托模式)。
- `cgs_module_has_intrinsic(...,"sico.text.trim")` / `cgs_helper_index(...,"sico.text.trim")` 纳入 helper 表。
- 函数体刻意无注释(否则触 `step0507`/`step0526` 的 `ERR:E-SH-IR-STATEMENT`,实测教训)。

## 3. 验证结果(诚实)

**通过的部分:**
- 编译通过(BUILD_EXIT=0,guest 从新 codegen.sico 构建无错)。
- **8/8 既有守卫全绿**:`sico_run_emits_the_minimal_multiblock_run_component_byte_exactly`、`sico_is_utf8_gated_run_lowers_byte_exactly`、`sico_compiler_emits_the_run_instruction_segment_byte_exactly`、`sico_compiler_emits_the_run_body_byte_exactly`、`step0507`、`step0513`、`step0526`、`sico_compiler_lowers_full_formatter_byte_exactly`。
- fail-closed:臂对未覆盖子形状**拒绝而非发射错字节**。

**未通过的部分(本片核心诚实点):**
- 新增字节钉 `sico_compiler_lowers_the_text_trim_intrinsic_byte_exactly` **无法转绿**,故标 `#[ignore]`(循 STEP-0608 测量探针模式)。
- **根因 = co-gap,非 text.trim 臂本身**。隔离实验:fixture 逐步简化后,**去掉 text.trim、只留 `utf8_decode`+`text.encode` 的 fixture 仍拒** `Domain{"invalid-input","unsupported codegen source shape"}` ⇒ 拒绝来自 `utf8_decode`/`text.encode`(它们虽有臂 :3830/:3827 但在此上下文不工作)或单块 `run` 返回 `ok(ScriptOutput(...))` 的形状,**不是 text.trim 臂**。
- **结构性困境**:run 的 `stdout` 是 `Bytes`,要把 `text.trim` 的 `Text` 结果送出去**必须经 `text.encode`**;而 `text.encode`/`utf8_decode` 自身有未证 gap ⇒ **无法构造隔离 text.trim 的 run fixture**。text.trim 的字节等同验证**被 utf8_decode/text.encode co-gap 阻塞**,须先证那两者。

## 4. 处置决定(诚实标注)

- **保留** agent-26 的 text.trim 臂:guard-safe、fail-closed、非拒绝原因;回退到已验证 G1 态 `5042ad42` **无干净备份**(`/tmp/sico_triage_backup/codegen.sico.step0613` = `def9fe6c`,是 STEP-0613 基线非 G1 态),手术移除有引入错误风险 ⇒ 保留风险更低。
- 但**不 claim 它字节等同**——它**未证**,仅 guard-safe。钉标 `#[ignore]` 如实记录 co-gap 阻塞。
- **连带**:codegen.sico 变更使 emitter-vocabulary 指纹(`guest_emitter.source_sha256` 须 == codegen.sico sha)过期 ⇒ 需 `census-m22-emitter-vocabulary.py --update` 刷新,但**本机无 Python**(AGENTS §10.1.4 ⑦)⇒ 与 ledger 重冻结同类阻塞。

## 5. Gate 与边界

- **未翻任何 gate,M22 overall 仍 NO-GO**。
- 未动 WIT/RFC/ADR/限额;未 commit/push。
- `cargo fmt --check`(runner workspace)净;`git diff --check` 净;`#[ignore]` 钉在正常跑中跳过(非红)。
- `frozen_match_site_ledger` MS-024 为独立项(STEP-0613c §6);codegen.sico 变更不影响 parser 站点 source digest。
- **G3 真实代价已显形**:helper 臂不是独立的——`text.trim` 被同 fixture 的 `utf8_decode`/`text.encode` co-gap 阻塞 ⇒ **G3 必须按 co-gap 簇推进**(可能需同时证 utf8_decode/text.encode/text.trim 三者),且 map 系(map.get/put/empty)更复杂。W3-a 组件面仍 open。