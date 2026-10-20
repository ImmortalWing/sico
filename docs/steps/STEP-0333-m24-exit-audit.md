# STEP-0333: M24 出口审计（§3 逐门裁决 + 具名延期登记 + 文档同步）

> - status: audit-of-current-state（对**本机当前状态**的逐门裁决）；**不移动任何 §3 gate**；M24 overall = **NO-GO**（本机存在外部门与合同门，不可能翻 full GO）
> - phase: M24 执行卡 #8（出口审计），按 [STEP-0330 §5 队列](./STEP-0330-m24-24a-contract-alignment-codec-probe.md)
> - completed: 2026-09-30
> - renumber: 主权威线（`codex/m22-w1-ci` + M23）最高为 STEP-0329；M24 轨已占 STEP-0330/0331/0332，本片取下一空闲号 **STEP-0333**；最终号在 M24 轨并入时再确认
> - nature: audit + doc-sync only——本片不新增实现、不改合同；仅登记已落地证据、逐门裁决、订正文档-证据偏差（AGENTS.md §1「文档与可执行证据冲突时取下界」）
> - evidence: internal-fixture（M17 语料与拒绝 fixture）；pilot 相关证据本会话未产生（#5 受权帧/#6/#7 受阻）；无任何 external-pilot / production 声称

## Objective

对 M24 §3 七个 exit gate 在**本机（Windows evidence host，离线、无加速器、无 Direct2D 绑定、无 JPEG crate、包 ABI 合同门未决）**当前状态下给出显式 GO/NO-GO 裁决 + 具名残余登记（AGENTS.md §3「里程碑仅在每门有证据且有显式 GO/NO-GO 审计时算完成」），并同步 M24 plan 中已过时的标签。

## §3 逐门裁决

| # | gate | 本机裁决 | 证据 / 缺失输入 |
|---|------|----------|------------------|
| 1 | M17 gate 2 加速器 provider | **NO-GO / 具名外部门** | 需 ≥1 台 Windows host 上加速实现与 reference 实现在容差内一致（§8.1）。本机无具名加速器硬件；reference-only 路径可跑但不足以翻 gate 2。缺失输入=加速器硬件 + accelerator provider ADR 落地 |
| 2 | M17 gate 4 model provenance | **未达（open）** | RFC-0051 已接受；fixture model 五件套（digest load / budget limit+1 类型化 / cancel mid-inference / malicious-asset 拒绝 / provider-crash 隔离免重启）尚未实现。**此门理论上本机可离线建（fixture model，非 live）**，是 M24 内部线唯一仍未推进的可建项 |
| 3 | Roster 三包（match/grid/contour） | **NO-GO / 合同门阻挡** | STEP-0330 §8 实测：已接受的 E8017 包接口值集（`crates/sico-semantics/src/lib.rs:745`）**无 record 类型**，`region`(record) 的 `list<region>`/`result<region,_>` 返回被拒。非缺实现=合同前置未决；须 owner 裁决 region 编码 / 包 ABI（见 STATUS 2026-09-30 #7 探针） |
| 4 | M17 显式审计 | **整体 NO-GO + 具名残余登记**（本片） | gate 1/2/3/5 未达；M17 overall 不翻 GO。image-vision@1（gate-1 前置，STEP-0166）已绿但不覆盖本审计 |
| 5 | GUI 格式转换器试点 | **NO-GO** | step-1 需 JPEG↔PNG 双向：JPEG 半边=#3 具名外部门；presentation 需 native UI library 受权渲染=#4 具名外部门；roster=#3 合同门。PNG→PNG 非有意义增量 |
| 6 | 证据标签分离 | **纪律保持** | M17 语料/拒绝 fixture 标 internal-fixture；pilot 未产生证据故不标 clean-room-consumer；无 external 观察（owner-gated），本审计不隐含 |
| 7 | M0–M22 + 已落地 M23 回归绿 | **部分自证（局部）** | 本会话 #5 e2e 仅动 runner workspace（`src/ui.rs`/`src/lib.rs`/`tests/ui_authority.rs`），未动 root workspace；完整 runner 套件单线程全绿（runner 37/37、http2 6/6、image_codec 5/5、ui 4/4、ui_authority 2/2）。root workspace 回归基线在 STEP-0331 全绿；gate 7 完整口径需在 M24 轨并入权威线时以 root 全量套件复核 |

## 已落地的内部线证据（本审计承认的进度）

- **#2 PNG codec**（STEP-0331）：`sico:user/image-codec@1` PNG 双向纯 package，digest-pinned Rust core（ADR-0019 class b），runner 内容测试 5/5 绿，clean 重建 byte-identical。
- **#5 `ui@1` 绑定 + 默认拒绝授权层**（STEP-0332）：`sico:script/ui@1.0.0` host-provider 全 13 函数注册 + 授权门先于渲染器；**end-to-end 授权拒绝 runtime-verified**（`tests/ui_authority.rs` 2/2：拒绝→`authority-denied` exit=115 且渲染器零触达；受权→exit=handle 且一次触达）。安全属性由单元级升至 runtime-verified。

## 外部门 / 合同门总表（本机不可翻）

- **gate 1**：具名加速器硬件（外部）。
- **#4 / gate 5 presentation**：Direct2D/DirectWrite 离线绑定（外部；ADR-0018 生产 adapter 缺失，仅 `UiRenderer` seam + 测试 double）。
- **#3 / gate 5 JPEG 半边**：离线 JPEG 编解码 crate + 独立 oracle（外部；225-crate cache 仅 `png-0.18.1`）。
- **#7 / gate 3 roster**：E8017 包接口值集无 record 类型（**合同门**，须 owner 裁决；非外部硬件/依赖门）。
- **#6**：随 #3 + #5(受权帧 #4) + roster 延期，本机不可完整交付。

## 本机唯一仍未推进的可离线建项

- **gate 2（M17 model provenance 五件套）**：fixture model 路线不需加速器、不需 JPEG/Direct2D、不需 record 包返回（digest/budget/cancel/malicious/crash-isolation 均可用标量/`List<Text>`/`Result[level1,scalar]` 表达），是当前唯一在本机内部门可完整推进的 M24 实现项。留作后续 STEP（不在此审计分配未来号，AGENTS.md §3「勿仅为排期而预留未来 STEP」）。

## Changes（doc-sync，无实现）

1. 新增本 `STEP-0333`。
2. `docs/plans/M24-vision-closure-gui-application-pilot.md`：
   - §8.4 conversion-engine 行订正「image-vision 无 codec I/O」为「`image-codec@1` 现提供 PNG decode/encode；JPEG 为具名外部门」。
   - §8.4 RFC-0050 标签由「draft, owner acceptance required」订正为「accepted + A1；PNG 已落地 STEP-0331；JPEG 具名延期」，并记 step-1「JPEG↔PNG 双向」本机不能达。
3. `docs/steps/STEP-0330-*.md`：#7 行前置与 Honest residuals 记入 E8017 合同门；新增 §8 探针记录；订正早前「#7 本机可建」不实声明（取下界）。
4. `docs/STATUS.md` / `docs/ROADMAP.md`（如适用）：登记 STEP-0332 e2e 与 STEP-0333 审计、#7 合同门。

## Executed validation

- 本审计为 doc + audit；证据引用均来自本会话已跑测试：
  - `cargo test --offline -- --test-threads=1`（runner workspace）：全绿（见上）。
  - `cargo clippy --all-targets -- -D warnings` + `cargo fmt -- --check`：绿。
- `git diff --check`：CLEAN（M24 轨 worktree `codex/m24-vision-ui`，仅动 M24 文件，无 unrelated dirty）。
- 未跑 root 全量 workspace（本会话 #5 e2e 未触 root crate；gate 7 root 口径留待并入权威线时复核，已如实登记为局部）。

## Verdict

- **M24 overall（本机）= NO-GO**：§3 gate 1/3/5 有外部门/合同门，gate 2 未实现，gate 4 审计据以下 NO-GO 并出具具名残余登记。
- **不替代、不静默改合同**：外部门（gate 1 硬件、#4 Direct2D、#3 JPEG）如实登记；合同门（#7 roster region 编码）呈 owner 裁决，未自裁、未伪造 Bytes 编码。
- **可交付内部线**：#2 PNG（✓）、#5 `ui@1` 绑定/授权拒绝含 end-to-end runtime-verified（✓）；gate 2 model provenance 为本机唯一剩余可离线推进项，未在本审计实现。

---

## Addendum 2026-09-30 (STEP-0334 之后；同轮**新鲜证据复核**，非重复旧结论)

本 addendum 用**本轮当场实测**刷新上方结论，并订正一处过时表述。原表按登记时状态保留（审计快照不追溯重写），差异以本 addendum 为准。

### A. 门 #3 / #4「外部门」——本轮实测确认（此前部分凭记忆）

- 现扫 `$HOME/.cargo/registry`（225 个 `.crate`）：**无任何 JPEG/image 系**（`zune-jpeg`/`jpeg-encoder`/`image`/`mozjpeg`/`turbojpeg` 皆无），仅 `png`。⇒ **#3 JPEG 确为离线不可建（真外部门），非未尝试**。
- **`windows-sys` 0.52.0 与 0.61.2 均在 cache**，但其 `Win32::Graphics` 子模块仅 `Dwm/Gdi/GdiPlus/Hlsl/OpenGL/Printing`，**不含 Direct2D/DirectWrite**（`ID2D1*`/`IDWrite*` 符号全无；那些属完整 `windows` crate，未缓存）。⇒ **#4 生产渲染器确无离线绑定**。
- **关键裁定：GDI/GdiPlus 存在，但 ADR-0018 把渲染器 home 冻结为 Direct2D/DirectWrite**——以 GDI 代偿 = AGENTS §5 禁止的「语义不同的替代实现」，其帧亦非 ADR-0018 冻结面，**不构成 #4 进度**。本 addendum 显式关闭「用 GDI 冒充 D2D」诱饵路径，防未来静默替代。

### B. 门 #2（M17 gate 4 model provenance）——推进 + 新合同门

- 原表「尚未实现 / 唯一可离线建项」**已过时**：STEP-0334 已落 `sico.m17.provenance.v0` 封闭 schema + 载入校验原语（`provenance.rs`，10/10 单测，internal-fixture/unit-level，未翻门）。
- 其**余下可建半部（carry + install-verify + budget 接线）本身撞到新的合同空白**：RFC-0051 §2/§3 未规定 manifest 携带位置 / `name`→payload 绑定 / `budgets`→`RuntimeLimits` 映射优先级——已起草 **RFC-0051 A1（PROPOSED，待 owner）**。即便 A1 获批，property 3/4 仍需**一份真实冻结 fixture-model 载荷**（模型资产=0，STEP-0287）。
- **订正结论：本机当前不存在「无需任何 owner 裁决即可完整实现」的 M24 门**——provenance 收口需 A1 裁决、roster 需 E8017 裁决、#3/#4/gate1 为外部门。内部可推进项已收敛为：两项合同裁决 + 一份 fixture model。

### C. Gate 7 回归——本轮合并全绿（刷新原「局部自证」）

- 本轮 `cargo test --offline --locked --manifest-path runner/sico-runner/Cargo.toml -- --test-threads=1`（`SICO_TEST_WASMTIME=1`）**重跑整个 runner 工作区**：**全绿、0 failed、exit 0**——41 lib + dap + 30 集成套件（`runner 37`、`selfhost_compiler 56`、`image_codec_package 5`、`http2 6`、`ui_authority 2` 等），单线程规避已知 DAP 并行 flaky。
- root `sico-package`（含 provenance）本轮 13+4+1+10 单测绿、clippy/fmt 绿。
- gate 7 仍**非 root 全量口径**（编译器/语言核心回归属 M14/M22 权威线）；此 addendum 把「M24 内部线（runner + sico-package）」升为**当前合并实测全绿**，root 全量复核仍留待并入权威线。

### D. Package **consumer** 路径 runner 边界 fail-closed 覆盖（本轮新增，纯测试、无生产代码/合同改动）

- **发现的真实空白**：此前所有 package 测试（`package_builder.rs` /
  `vision_package.rs` / `image_codec_package.rs`）都用**空 linker 直接实例化
  package**，绕开 `Runner` 的 link+limits 机制——即 §8.3「consumers install them
  through the M7 registry path」与 §8.5「typed outcomes end to end」所在的**消费
  者边界**从未被测。而真实 `sico run` CLI（`main.rs` 用
  `run_program_with_policy_and_packages`/`prepare_program_with_net_packages`）恰恰
  依赖 `link_packages`/`prepare_package` 这条安全相关路径，**零测试覆盖**。
- **新增 `runner/sico-runner/tests/package_consumer_authority.rs`（3 测，绿）**：
  以 `wasm-encoder` 手编「导入 `sico:user/csv@1.0.0` 的 Program」经真实 runner
  准备路径断言三类 **fail-closed 启动拒绝**（均 `RunOutcome::Launch`，未跑到
  guest）：
  1. **未备包导入** → guest 导入某 `sico:user/…` 实例而 prepared packages 无对应
     导出 → `"no prepared package exports it"`（消费者不能凭空拉入未验证包）。
  2. **arity 不匹配** → guest 声明 `parse-line` 参数数 ≠ 真实 `csv` 导出（桥接用
     动态 `Val` 直转 package export，错 arity 不安全）→ `"arity mismatch"`
     （桥接在链接期即拒，绝不落到部分/错误调用）。
  3. **不纯 package** → 把「自身带 import 的组件」当 package 传入 →
     `prepare_package` 在任何链接前即拒 `"package component must not import"`
     （v0 package 必须纯；import 只能收窄权限，package 无 grant 可收窄，故绝不
     能把环境授权偷运进消费者 Store）。
- **验证**：`--test package_consumer_authority` **3/3 绿**；完整 runner 套件单线程
  重跑 **234 passed / 0 failed**（含本片 + 前轮 ui_authority 4/4，无回归）；
  clippy `-D warnings` / fmt 绿。**证据级别 = internal-fixture / contract-verified
  at the runner boundary（真实 wasmtime 组件编译 + 链接器桥接），非 package-runtime
  feature 声称**；**不移动任何 §3 gate**，gate 6 标签纪律保持，gate 7 消费者边界
  fail-closed 属性由「未测」升为「runner 边界实测」。
- **仍未覆盖（如实登记）**：happy-path 端到端「Program 成功调用 package export
  并取回字节」仍无测（需手编带 `run` 提升 + `result<list<string>,string>` 全 ABI
  的 Program，风险/成本高于本片的 fail-closed 安全属性收口）；#7 试点级
  「cancellation mid-conversion」「save-path」仍 gated on #4/#3 外部门。

## Addendum E (2026-09-30, 同轮：登记于 Addendum D 的 happy-path 残余现已闭合)

上方 Addendum D 的「仍未覆盖」项——「happy-path 端到端 Program 成功调用 package
export 并取回字节」——**本会话已补**（`runner/sico-runner/tests/package_consumer_forward.rs`）：

- **构造**：`wasm-encoder` 手编一个真实 Program 组件，component 级导入
  `sico:user/csv@1.0.0`（声明 `parse-line: func(string) -> result<list<string>,
  string>`，实例面**仅导出该函数**——`link_packages` 要求每个实例导出皆为函数，
  早期误把 `string-list` 类型别名一并导出触发 `"…string-list must be a function"`
  失败，已订正），对共享 transport 降形 `parse-line`（`result<list,string>` 扁平
  >1 核类型 → return-area 形，`(ptr,len,retptr)`），guest 核体经提升的 `run`
  两次调用之，只读 return-area 的 **discriminant @0** 与 ok 分支 **list 长度 @8**。
- **断言**：经 `Runner::prepare_program_with_packages` + `.run(...)` 走真实消费者
  桥接（`func_new` 动态 `Val` 转发到同 Store 实例化的 package），`exit =
  ok_disc*1_000_000 + ok_list_len*10 + err_disc`。冻结期望 **21** 一次锁死三事实：
  `parse-line("a,b")` 经桥接**取回真实 2 元素 list**（ok_disc=0、len=2），
  `parse-line("\"abc")` 返回类型化 err 分支（err_disc=1，即 package 的
  `unterminated-quote` 拒绝）。`consumer_run_is_repeatable` 复核每次 run 重新
  `instantiate_packages` 进 fresh Store 的可重复性。
- **验证**：`--test package_consumer_forward` **2/2 绿**；完整 runner 工作区单线程
  **37 套件全绿 / 0 failed**（无回归）；`clippy --all-targets -D warnings` +
  `fmt --check` 绿。
- **证据级别**：internal-fixture；runtime-verified 的是**消费者桥接的转发+回取**
  本身（真实 wasmtime 组件编译 + `func_new` 动态转发 + `result<list,string>` 全
  ABI），**非** package 内容语义（那由 `package_builder`/`image_codec_package`
  覆盖），**非**渲染/试点级 feature 声称。**不移动任何 §3 gate**：此闭合把 §8.5
  「typed outcomes end to end」在消费者边界的 happy-path 半部补全，但 gate 3
  roster / gate 5 试点的 **cancellation-mid-conversion / save-path-failure** 半部
  仍分别 gated on #7b（E8017 合同门）与 #4/#3（外部门）。M24 overall 本机仍 NO-GO。
