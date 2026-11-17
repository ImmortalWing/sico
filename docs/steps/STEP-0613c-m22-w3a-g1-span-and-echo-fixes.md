# STEP-0613c — M22 W3-a G1 span 修复 + echo 回归修复 + MSVC 构建路径

> 状态:**implementation complete locally(部分验证)** —— G1 能力钉与自举前沿守卫全绿;**`frozen_match_site_ledger` 仍红(MS-024 source digest),归因待精确根因**。未翻任何 gate,未 commit/push。
> 日期:2026-11-10。基线 HEAD `e93f59ad`(工作树未提交)。worktree `D:/SorftWare/sico-m22-w3a`,分支 `codex/m22-w2-step0613`。

## 1. 本片目标

续 [STEP-0613](./STEP-0613-m22-w3a-o1-oracle-hygiene-g1-guest-multiblock-run.md)/[STEP-0613b](./STEP-0613b-g1-guest-is_utf8-helper-closure-fix.md):G1(guest 最小多块 `run`)此前**已写完但因 mingw 工具链缺失从未跑过 cargo 正确性钉**。本片:
1. 建立 **MSVC 构建路径**(owner 已装 VS 18 Community + SDK 10.0.26100 + rustup `x86_64-pc-windows-msvc` target),首次跑通 G1 的 cargo 钉。
2. 修复跑钉暴露的真实失败。

## 2. MSVC 构建路径(本片基础设施产出)

- 工具链:`RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-msvc`;`cargo test --locked --offline --target x86_64-pc-windows-msvc --manifest-path runner\sico-runner\Cargo.toml --test selfhost_compiler`。
- **关键坑**:root `.cargo/config.toml` 的 `[target.x86_64-pc-windows-gnu] linker="rust-lld"` 只作用于 gnu target,msvc 构建忽略之 ⇒ 不需改 config。
- **关键坑 2**:若钉 `RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu`(AGENTS §8 旧口径),该工具链**无 msvc target 的 core** ⇒ `error[E0463]: can't find crate for core`。msvc 构建必须用 msvc 工具链。
- **rustc 自定位 MSVC**:现代 rustc 经 vswhere/registry 自找 `link.exe`+SDK,**无需 vcvarsall 的 PATH**(`where cl.exe` 失败也不影响 `rustc --target msvc` 链接成功,冒烟 hello world 验证)。
- 测试二进制:`runner/sico-runner/target/x86_64-pc-windows-msvc/debug/deps/selfhost_compiler-*.exe`;单测快跑 `<exe> --exact <name> --test-threads=1`。
- **承重认知(订正本片早期的错误前提)**:测试经 `include_str!`(selfhost_compiler.rs:23-24)把 `selfhost/*.sico` **烘进二进制**,`cached_component` 按其哈希建 guest ⇒ **改 `.sico` 必须重建二进制才生效**,不是运行时读盘。

## 3. 修复一:guest 前端 `project` span bug(真实 bug,非钉错误)

`selfhost_compiler.rs:3989` `sico_run_emits_the_minimal_multiblock_run_component_byte_exactly` 失败:`byte mismatch at 949; actual_len=3130, expected_len=3130`,长度相等、仅 `project stdin` 的 `range.start` 差 5(guest 468/489 vs oracle 473/494,end 相同)。

- **根因**:`selfhost/parser.sico` `gw_ok_wrap_rhs_packed`(:5007-5008、:5022-5023)用 **base-start**(`expr_start`)算 `project` 的 canonical-IR `range.start`;而永久 oracle(`crates/sico-ir/src/lower.rs:3328`)用 **dot-start**(`token_range(&tokens[index..index+2])` = `.field` 段)。guest 自己的**一般**字段访问路径也用 dot-start ⇒ 仅这个特化 ok-wrap 路径不一致。
- **canonical 判定**:oracle 的 dot-start 为准(永久 reference;guest 一般路径亦然)。`input.stdin` 整表达式字节域是 [468,479],但 canonical-IR `Project` span 约定是 `.field` 段。
- **修法**(`gw_ok_wrap_rhs_packed` 4 行,行数中性):`word_span_offset(..., receiver_idx, gow_e1)` → `gos_d1`(start=第 1 个 `input.stdin` 的 dot);`gos_p1e`/`gos_p2s`/`gos_p2e` 同理改走 `gos_d1`/`gos_d2`。**未削弱钉、未改 expected**。
- 结果:span 变 [473,479]/[494,500],与 oracle 逐字节等同。

## 4. 修复二:echo `run_instruction_segment` STEP-0613 回归

全量套件暴露第 2 个失败 `sico_compiler_emits_the_run_instruction_segment_byte_exactly`(`selfhost_compiler.rs:4715`,echo fixture,`--emit-run-instr-hex`):`actual_len=350, expected_len=182`,前 182 字符一致、guest 多出 result-area return + `0b`。

- **判定:STEP-0613 回归,非既有**(重建到 HEAD 源后该测通过)。
- **根因**:`selfhost/codegen.sico` `cgs_run_instr_hex`(:4130)STEP-0613 G1 把 `cgs_function_instr_hex(bytes, crih_run, U64.literal(0))` → `U64.literal(1)`(`terms=1`),使单块 echo `run` 也发射 per-block terminator(result-area return + `0b`)。多块 `is_utf8` fixture 需要 `terms=1`,G1 全局开启 ⇒ 在单块 echo 路径**过度发射**。
- **修法**(2 函数):`cgs_run_instr_hex`(:4130)`U64.literal(1)`→`U64.literal(0)`(指令段路径重新止于 result-area return 之前);`cgs_run_body_hex`(:4858)与 `cgs_run_instr_hex` **解耦**,直调 `cgs_function_instr_hex(bytes, crbh_run, U64.literal(1))` ⇒ 整 body 保留 per-block terminator/branch/逐 return result-area。body 输出与 STEP-0613 **逐字节相同**(同调用同参)⇒ 多块/组件钉不受影响。
- **实现约束(实测)**:这两个函数体内加 `//` 注释会触自举形状前沿(`step0507`/`step0526` 报 `ERR:E-SH-IR-STATEMENT`)⇒ 函数体刻意无注释,函数前既有注释保留。

## 5. 验证(coordinator 独立复验,重建后二进制)

7/7 全绿:
- `sico_run_emits_the_minimal_multiblock_run_component_byte_exactly`(G1 能力钉,曾 FAILED → ok)
- `sico_is_utf8_gated_run_lowers_byte_exactly`(翻转门,ok)
- `sico_compiler_emits_the_run_instruction_segment_byte_exactly`(echo 回归,曾 FAILED → ok)
- `sico_compiler_emits_the_run_body_byte_exactly`(ok)
- `sico_compiler_lowers_the_step0507/step0513/step0526_codegen_function*_byte_exactly`(自举前沿守卫,全 ok)
- `cargo fmt --check`(runner workspace)净;`git diff --check` 净。

全量 `selfhost_compiler` 套件:**187 passed / 2 failed / 5 ignored**(6430 s)。2 failed = echo(本片已修)+ `frozen_match_site_ledger`(见 §6)。子代理另验 `ok_return_construct`/`step0519`/`step0520`/2 个 run-refusal 边界测 ok。

## 6. 开放项:`frozen_match_site_ledger` MS-024 source digest —— **根因已确证(独立只读诊断片)**

`sico_compiler_reproduces_the_frozen_match_site_ledger` 红:`MS-024: parser.sico:while_call_rhs_packed` source digest 失配,测试在首个失配即停。

**【2026-11-10 根因确证,订正本节早先「归因存疑」的措辞】** 独立只读诊断(读 census 工具 + ledger 测试体 + std-only Rust 复现算法,精确复现两个值故自验证)确证:

- **`source_sha256` 算法** = `sha256(HOST_CALL_SHAPE_PRELUDE || closure || "\n" || HOST_CALL_SHAPE_RUN)`,其中 `closure` = 站点所属函数的**传递同文件 callee 闭包**(按文件序,每个函数贡献其 `function <name>`..`end function` 行的原文,行间 `\n`;上限 256 fn / 1 MiB)。**纯原文,不含行号/偏移/IR**。权威:`selfhost_compiler.rs:9750-9751`(digest 源)、`:9671-9708`(闭包)、`:3734-3735`(PRELUDE/RUN);census `.py:194-202` 只是从 `actual.json` 拷贝该值。
- **机制**:`while_call_rhs_packed` 的闭包 = 80 函数 / 326,739 B,**含 `gw_ok_wrap_rhs_packed`**(传递 callee)⇒ §3 的 4 行 span 改动虽行数中性且在「另一函数」,但**改变了闭包原文** ⇒ digest `a7539b1c…`(修复前)→ `1b23b03f…`(修复后)。
- **复现**:HEAD parser.sico → `a7539b1c…` = **账本冻结值**(HEAD 处对齐);worktree(span 修复后)→ `1b23b03f…` = 二进制 actual 值。⇒ **账本相对未提交的 span 修复过期,非 HEAD 既有失配**。
- **非 codegen.sico**:`source_sha256` 只依赖 parser.sico(同文件闭包),codegen.sico/lib.rs 的 STEP-0613 改动不影响 parser 站点 digest。
- **仅 source_sha256 失配**:verdict 断言(`:10042`,`ir-equal+component-other:FuelExhausted`)通过 ⇒ 纯变量重命名,canonical IR 与 component 均不变 ⇒ **非正确性回归,是 digest 同步问题**。
- **标签订正**:测试 `assert_eq!(site.source_sha256, outcome.source_sha256)` 中 `left`=site=冻结=`a7539b1c…`、`right`=outcome=actual=`1b23b03f…`;失败消息的 "(actual)/(frozen)" 措辞与左右相反,以裸 hex 为准(ledger 存 `a7539b1c…`,`1b23b03f…` 在其中出现 0 次)。
- **重冻结范围**:**26 个 parser.sico 函数**的闭包含 `gw_ok_wrap_rhs_packed`(`while_call_rhs_packed`/`gw_compare_packed`/`chain_condition_packed`/`gw_rhs_packed`/`fixed_guard_chain_ir`/`scalar_ir` 等),这些站点的 `source_sha256` 需重测。
- **规范路径阻塞**:`census-m22-match-sites.py --update` 需 Python,**本机无可用 Python**(AGENTS §10.1.4 ⑦)⇒ 需非规范合并(手改 JSON)或先解决 Python。**因触及冻结 gate 证据且路径非规范,留待 owner/独立切片处置,本片未重冻结**。
- **未跑**:ledger 慢速逐组重算;重冻结后 ledger 测试未重跑(单跑 ~90 min)。

## 7. Gate 与边界

- **未翻任何 gate,M22 overall 仍 NO-GO**。本片是 G1 能力片(前端 span + codegen run 发射),非 gate flip。
- 未动 WIT/RFC/ADR/限额;未改 formatter.sico;未 commit/push(遵 AGENTS §10.2 owner 授权)。
- W3-a 仍 open:G1(最小多块 run)已通,G2(formatter 实形 2 分支+3 return,逐字节 == oracle 28459 B)与 G3(W3-a 收口)未开工。
- 工作树改动(均未提交):`crates/sico-codegen-wasm/src/lib.rs`(O1,STEP-0613)、`selfhost/codegen.sico`(G1 + echo 修复)、`selfhost/parser.sico`(span 修复)、`runner/sico-runner/tests/selfhost_compiler.rs`(钉 + fmt 空白);新增本 doc + 既有 STEP-0613*/0613b docs。