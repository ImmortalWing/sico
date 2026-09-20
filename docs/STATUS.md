# Sico project status

> - updated: 2026-09-20 (STEP-0243 adds the frozen E1001-E1016 syntax identities to the integrated Sico parser/checker)
> - phase: M12 complete (full GO per STEP-0127); M13 closed — authoritative live-model run measured and budgets met under ADR-0012 (STEP-0128); M7 public deployment and M6/mobile remain deferred
> - phase status: M11 complete (GO 2026-09-02); M12 complete (GO 2026-09-05, gates 10/10, Windows x64 + Linux x64 native runtime evidence); M22 compiler self-host track **S1 complete，S2/S3/S4/S5 partial，S6 尚未进入**。STEP-0193–0212 已把 lowering 扩至递归表达式、multi-parameter SSA、typed user calls、常量/定宽字面量/嵌套调用实参与定宽运算；STEP-0213–0221 冻结 215-source 语料、关闭 formatter 与 lossless lexer、补 accepted declaration metadata、native build oracle 和 modular frontend evidence。
> - current step: STEP-0243 在 integrated parser/checker 中按真实块结构、定界符与 module/use/interface 形状实现 E1001–E1016；12 个 mutation 与 4 个 shape oracle 经真实 runner 全绿，STEP-0242 的 215-source 词法分区无误判回归保持绿。34 个非词法 Rust 语义/模块拒绝仍开放，因此 S2 保持 partial。
> - current support step: 无
> - last completed support step: STEP-0196 (M19 支持轨：fresh-host CI 修复——`tools/ensure-python.ps1`（CPython 3.12.10 embeddable SHA256 钉住，WindowsApps 存根排除 + Stop 偏好下的 stderr 终止错误修复）+ `run-ci.ps1` 三项修复（msys2 binutils PATH、Python 进 PATH、runner debug 构建先于 workspace 测试）；console-control 测试失败根因据实修正为缺 Python 依赖（非 TTY 理论未被实验证实），补 Python 后 2/2 绿；修复后完整 run-ci **CI GREEN 11/11**（clippy 本机首跑 -D warnings 全过）；此前 STEP-0123 (M13 closure audit: a/b/c GO, d blocked-external-evidence))
> - last completed active step: STEP-0243 (checker E1001–E1016：结构化检测、16/16 stable identities、215-source 无误判)；此前 STEP-0242 (116/99 lexical partition)。
> - next step: M22 S2 实现 34 个非词法 Rust 拒绝的语义/模块诊断与稳定身份，并淘汰 checker 的 free-form header 报告；随后收拢 S3/S4、实现 RFC-0011 codegen 与 ADR-0015 bootstrap harness；自举 GO 后再进入原生 UI/GUI 转换器。
> - roadmap decision: owner approved the application-layer sequence on 2026-09-04 — M14 application-ready language, M15 Web/UI, M16 Native Automation Host, M17 vision/model packages, M18 representative AI applications and external pilots; M19 production engineering, M20 cross-platform/language v1/completion audit, M21 DX/stdlib batch 2/ecosystem activation and M22 compiler self-host followed by owner session directives (2026-09-13/14); on 2026-09-17 the owner directive 「继续完成sico，M22-M25」 confirmed continuing M22 and registering M23–M25 (language v1 batch 3, vision closure + block-game pilot, release v1.0 completion audit); M13 quality-budget gap (STEP-0128) is a recorded product fact for M17/M18 planning
> - next support step: 无（M13 已收口：live-model 实测达标，ADR-0012 记录一个既定缺口 B-repair 0.833，供未来质量步骤跟踪）

## 0. M6 exit state

STEP-0054–0061 host-side work, 8,192 security properties, Desktop result `42`, Mobile Host metadata parity and full workspace regression are complete. M6 remains NO-GO because this environment has no Android build toolchain, buildable Host or runner; no Android Runtime, native touch/IME/TalkBack or startup evidence is claimed. Android and Harmony tracks are deferred. M7 STEP-0062–0069 is complete locally, including a two-release Wasmtime pilot; production, third-party, live-model, mobile/Linux-native and final product exit claims remain gated.

## 1. Current objective

M8 Script Profile 与 M9 streaming/async/interactive 已分别通过出口审计。M10 Runtime observability/debugging 已由 STEP-0102 发出 GO（Windows x64 GNU 实证）：identity-bound debug triplet、typed Runtime source faults、typed signal/client cancellation、task-aware bounded events、机器清单驱动的 minimal DAP 与 data-only editor/AI boundary 全部完成。当前主动目标是 M11 bounded structured-concurrency Runtime：STEP-0103 的 [`ADR-0010`](./adr/ADR-0010-single-store-structured-concurrency.md) 已接受 single-Store cooperative 设计，STEP-0104 完成 [`RFC-0036`](./rfc/RFC-0036-structured-concurrency-semantic-ir-v0.md) semantic/IR contract，STEP-0105 已在 runner 内落地有界 scheduler core，STEP-0106 完成 downward cancellation tree 与 scheduler 级 race/select，STEP-0107 完成 bounded channels/backpressure；下一项为 STEP-0108 persistent runner/watch/REPL/DAP task 集成。M11 完成后 M12 再实现 Secure HTTP Provider/Automation SDK。公网部署仍等待真实域名/hosting、生产身份与密钥托管输入；Android、Harmony 与非 Windows runner 继续保留为外部/后置平台轨，内部里程碑不替代这些 gate。

## 2. Current step

[`STEP-0069`](./steps/STEP-0069-third-party-pilot-m7-exit.md) 已用独立于 `examples/` 的两版洁净室应用完成 LSP/AI、Component、签名包、本地 registry、Host upgrade 和 Wasmtime `42`，并拒绝四类攻击。[`M7 exit audit`](./reports/m7-exit-audit.md) 结论为 `blocked-external-evidence`。[`STEP-0070`](./steps/STEP-0070-mobile-platform-development-handbooks.md) 与 [`STEP-0071`](./steps/STEP-0071-linux-development-handbook.md) 只保存平台手册，没有改变平台证据状态。

[`STEP-0072`](./steps/STEP-0072-user-manual-information-architecture.md) 已把根 README 改为用户入口，将原开发状态内容迁入独立开发手册，并建立安装、入门、语言、CLI、Runtime、trust、LSP、AI、排障与限制的细分用户手册；文档变化不提高任何平台或 production 证据等级。

[`STEP-0073`](./steps/STEP-0073-openjdk-style-modular-monorepo.md) 已在两个远端归档完整 `v0.0.1` 开发流程，并按 OpenJDK 模式保持单仓库、拆分 `sico` 与 `sico-app` 命令、建立 22 个 package 的机器依赖边界。该步骤不提高任何平台或 production 证据等级。

[`STEP-0074`](./steps/STEP-0074-production-deployment-origin-and-ux.md) 新增第 23 个 workspace package `sico-registry-server`，通过 loopback HTTP origin、operator ZIP、release composition 和真实 Wasmtime `42` 完成本地生产部署基线。它没有创建公网域名、TLS、生产发布者或真实密钥，证据等级为 `complete-local`。

[`STEP-0093`](./steps/STEP-0093-editor-ai-execution-integration.md) 新增第 24 个 workspace package `sico-tooling-protocol`，由 LSP 与 AI tools 共同依赖；module-boundary validator 精确覆盖 24 packages 与 4 条窄依赖例外。

[`STEP-0142`](./steps/STEP-0142-m15-kickoff-inventory-modules-wit-rfc.md) 已完成 M15 计划 §7 要求的开工盘点（镜像 STEP-0129 模式）：实测探针证明编译单元恰为单文件、`module`/`use` 触发 E1013、`interface` 声明被语义静默丢弃（check ok + outline 可见 + 代码生成忽略——未声明行为缺口）、调用目标在 codegen 期才解析（未定义调用过 check、build 拒绝）；[`RFC-0039`](./rfc/RFC-0039-source-modules-package-resolution-user-wit-v0.md) 草案冻结前置轨合同（显式 `module` 声明 + `use` 导入、经 M7 lock 的 versioned 包解析、user WIT 导入 v0 值域、13 类 typed 拒绝 + limit+1），状态 draft 待 owner 接受；同文件冻结洁净室消费者 `pilots/log-analyzer` 有界特性清单（M15 入口条件 3 的关闭载体，owner 2026-09-07 决策保留条件按证据关闭）。

[`STEP-0129`](./steps/STEP-0129-m14-inventory-profile-rfc.md) 已完成 M14 计划 §7 要求的开工前盘点：实测探针证明递归过 check 但被"match 分支必须 return"与 fixed intrinsic 面封死，loop/if 语法尚不存在；[`RFC-0038`](./rfc/RFC-0038-application-profile-v0.md) 冻结应用 profile（循环/分支/集合/位运算/源级 PRNG/http@0.2.0 emission）与出口语料（方块求解器增强语料 + 流式 + capability 状态机），状态 draft 待 owner 接受。

[`STEP-0130`](./steps/STEP-0130-general-control-flow.md) 已落地 RFC-0038 profile 的控制流切片：`while`/`if`-`else`/`break`/`continue`/`set` 重新赋值经 IR mutable cells 与通用结构化 CFG 降级全链路可执行（冻结的 all-return match、revision-if、直线形状字节不变）；端到端语料 `tests/end-to-end/control-flow-counter.sico` 经真实 runner 验证确定性结果；顺带修复 watch cancel bridge 抢跑消费请求文件的实缺陷。

[`STEP-0131`](./steps/STEP-0131-dynamic-collections-map-set.md) 已落地 RFC-0038 profile 的集合切片：`Type::Map`/`Type::Set`（插入序、copy-on-write、首插键序）与 11 个带元素后缀的 canonical intrinsics（`sico.map.empty/put/get/has/length/keys`、`sico.set.empty/add/has/length/to_list`）经语义推断、IR verifier 与 Wasm 代码生成全链路可执行；键按字节内容比较（Text/Bytes），修复 split_words 关闭条件反转的预存在缺陷（元素长度此前从未 e2e 可执行）；machine-readable 支持矩阵 `tests/language-matrix/application-profile-v0.json` 与校验器 `tools/validate-step-0131.ps1` 落地；端到端语料 `tests/end-to-end/map-set-frequency.sico`（词频 Map/Set 计数）经真实 runner 验证。

[`STEP-0132`](./steps/STEP-0132-bit-operations.md) 已落地 RFC-0038 profile 的位运算切片：`I64`/`U64` 的 `bit_and/bit_or/bit_xor/shl/shr`（移位量同类型，wasm 掩码至 [0,63]；`I64` 算术右移、`U64` 逻辑右移）经语义、IR verifier 与代码生成全链路可执行；端到端语料 `tests/end-to-end/bit-ops-bitboard.sico`（popcount 循环 + 位恒等断言 + 移位语义区分）经真实 runner 验证。

[`STEP-0133`](./steps/STEP-0133-map-keys-stride-and-arena-audit.md) 已承接 STEP-0131 缺陷取证报告的 §2.6 移交项：逐一审计 `sico-codegen-wasm` 全部 `$alloc` 站点（零尺寸块仅在证明无存储/不逃逸时判定安全），发现并修复一个真实缺陷——`map.keys` 此前把 16 字节步距的 map entries 表原样当作 8 字节步距的 `List[Text]` 表返回，索引 ≥1 的元素读到的是前一条目的 value 槽（基线实证 `"alpha|beta"` 被 join 成 `"alpha|"`）；修复为压缩拷贝，`set.to_list` 保持零拷贝（步距本就精确）；新增端到端语料 `tests/end-to-end/map-keys-traversal.sico`（遍历助手元素内容、内容回查、交错分配、空/单例 map）经真实 runner 验证。

[`STEP-0134`](./steps/STEP-0134-checked-mul-div.md) 已补齐 RFC-0038 profile 第 5 项的乘除切片：`I64`/`U64.checked_mul/checked_div` 经语义、IR verifier 与代码生成全链路可执行；mul 溢出用回除法检测（含 `a==0`/`a==-1` 预排空，文档化 soundness 论证），div 上报新增的 `NumericError.division-by-zero` 第三 case（仅生成组件类型导出，非 WIT 合同变更）；`NumericError` 枚举扩展使 `fixed-width-component` 快照 +17 字节（追加式快照约定，core 快照字节不变）；端到端语料 `checked-mul-div.sico`（19 项 guest 断言）经真实 runner 验证。

[`STEP-0135`](./steps/STEP-0135-source-level-prng.md) 已落地 RFC-0038 profile 第 7 项：源级确定性 PRNG 无需任何编译器新表面——xorshift64（13/7/17）以纯 Sico 源码（`U64` 位运算 + while 循环）实现，种子 88172645463325252 的前 8 个输出钉住 Python 参考序列；无任何 ambient 随机性进入任何表面；端到端语料 `prng-xorshift.sico` 经真实 runner 验证。

[`STEP-0141`](./steps/STEP-0141-m14-exit-audit.md) 已出具 M14 出口审计：**GO**。逐项证据：无未声明 check/build/run 缺口（矩阵+校验器；一处行为缺口——字符串 `
` 转义静默保留——已修复并声明）；三个验收应用经真实 Component Runtime 全部可执行；求解器与冻结 Python oracle 4/4 字节级一致且节点记账等价（limit fixture 204,077/204,077）；limit+1 全部 typed 且宿主可复用（StackLimit 复用测试）；M7/M12 安全语料原样承载；Windows（除已文档化的 console-control 环境敏感 flake，隔离复跑通过）与 Linux x64（WSL2 原生，14/14 套件全绿，证据 `target/evidence/step-0141/linux/`）双平台复核完成；gate 7（M14 profile 的 AI 回归重测）按 M13 先例记为 blocked-external-evidence（owner 凭据门控）。范围诚实声明：M14 计划 §3.3 的 modules/WIT 项不在 RFC-0038 冻结 profile 内、无 STEP、不作宣称。

[`STEP-0140`](./steps/STEP-0140-sico-test.md) 已落地 M14 §3.4 的 `sico test`：`NAME.sico` + `NAME.test.json` 成对发现（递归、排序、忽略无清单源），清单字段 `stdin/args/expect_exit/expect_stdout`（拒绝未知字段），经与 `sico run` 相同的缓存身份构建并通过真实 `sico-runner` 默认限额执行，stdout 字节级比较；流式组件在 v0 外（typed 拒绝）；`tests/sico-tests/` 附带两个已提交用例；CLI 集成测试 2/2（排序发现/失败优先/exit 0-1/args 与 expect_exit 生效）。

[`STEP-0139`](./steps/STEP-0139-stream-and-state-machine.md) 已落地 RFC-0038 §3.2 两个伴随验收应用：流式行号变换（4 KiB 有界分块读取，1 MiB 输入经默认预算全量回环；非 UTF-8 输入产生 typed 部分失败 exit 122；流读写走宿主可取消通道）与 fs 持久化能力状态机（sealed/open/burned 三态、单元串修订守卫、typed 非法迁移与陈旧拒绝、缺失/损坏文档确定性恢复）；顺带修复两个预存在 stdlib 缺陷——`text.split_lines` 的非 LF 分支 `br 0` 错指 if 标签导致逐字节落行并越界写表（元素内容此前从未 e2e 覆盖），及该助手内三处 `len == 0` 守卫反转（末尾 LF 计数与 CR 剥离失效）——并补齐 `unquote` 的字符串转义解码（此前 `
` 静默保留为两个字符）。

[`STEP-0138`](./steps/STEP-0138-block-solver-port.md) 已落地 RFC-0038 §3.1 主验收应用：8x8 位算术方块求解器以纯 Sico 源码实现（位运算/一般递归/checked 算术/确定性搜索/typed node-limit），与固定 Python oracle 的冻结语料 4/4 fixture 字节级一致（trivial/medium/hard 完整搜索 + limit 部分证据）；`medium`/`limit` 触发 200,000 节点预算并携带最佳部分方案（出口 gate 4）；双方在 limit fixture 的节点记账完全一致（204,077）；移植期间修复五个移植缺陷（bridge 饱和/SWAR 回绕乘不安全/原点 off-by-one/陈旧字符 cell/第 4 棋子 flush 缺失）；冻结 capability 计数更新为 20 lowered/5 refused（STEP-0137 spill 释放 explicit-result-handle）。

[`STEP-0137`](./steps/STEP-0137-bounded-recursion.md) 已关闭 RFC-0038 profile 第 2 项：递归全链路可执行——修复冻结 all-return match 对计算主题 payload binding 的误拒（spill 到编译器生成 cell，冻结形状字节不变）；runner 将 wasmtime `Trap::StackOverflow` 分类为新 typed `RunOutcome::StackLimit`（`resource-limit.stack`/exit 125），guest 实例化+调用移到 64 MiB 栈专用 worker 线程，深递归先触发确定性 4 MiB wasm 栈预算而非原生栈崩溃（修复前探针直接打崩进程）；耗尽后 Host 可复用（出口 gate 4）；端到端语料 `recursion-depth.sico`（万层 countdown + fib(12)）与 `recursion-unbounded.sico` 经真实 runner 验证。

[`STEP-0136`](./steps/STEP-0136-http2-source-emission.md) 已落地 RFC-0038 profile 第 8 项：源码级 `sico.http2.request`（buffered one-shot，Host 默认 options：空 headers、不跟随重定向、不重试、Host 默认超时）发射 `sico:script/http@0.2.0` 导入，与冻结的 0.1.0 兼容面并存；typed `http-error` 经 data 段静态表映射为 case 名 Text（Host 文本不过界，遵守 RFC-0037 无泄漏规则）；修复 bring-up 期间发现的判别式边界检查在 ok 侧误读 status 低半字的 trap；streaming/upload 与资源保持 Component 级（矩阵已声明）；端到端经真实 TLS fixture server 验证（授权回环 `200:source-http2`；未授权 `error:permission` 且零连接）。

[`STEP-0128`](./steps/STEP-0128-live-model-evaluation.md)（dev 分支）已按 M13 §5 插入规则完成权威 live-model 评测：DeepSeek `deepseek-chat`，96×30=2880 次真实调用，成本 $0.78（封顶 $10），实测 **0.9095**（2245/2880；generation 0.40/0.50/0.30、repair 0.914/0.833/0.994、understanding 1.00/1.00/0.999）。对照 superseding ADR-0012（owner 2026-09-05 决定：真实模型对新语言实测 ≈0.9 为现阶段达标水平）：**预算下达且达标**，§13 (d) = **实测 GO**；记录一个既定缺口 B-repair 0.833 < 0.850 target，供未来质量步骤跟踪。报告：[`docs/reports/ai-eval-live-model-v1.md`](./reports/ai-eval-live-model-v1.md)。

[`STEP-0126`](./steps/STEP-0126-linux-provider-corpus.md) 已在 WSL2 Ubuntu-24.04（Linux x64 native）重跑全部 provider 语料与 runner http2 语料：provider 48/48、runner lib 37/37、http2 fixture 6/6、runner 集成 35/35（2 个为 `#[cfg(windows)]` 控制台测试）、STEP-0089 oracle 13/13，全部绿（证据 `target/evidence/step-0126/linux/validator.log`）。[`STEP-0127`](./steps/STEP-0127-m12-full-go-audit.md) 据此把 STEP-0118 的 GO-core 升级为 **M12 完整 GO**：10/10 exit gates GO，自动 NO-GO 清单零违例；M12 后不再有未关闭的内部里程碑门槛，M14 进入条件（M12 完整 GO + M13 收口 a/b/c GO + M0–M13 regression 可重跑）已满足。

[`STEP-0125`](./steps/STEP-0125-http2-runner-integration.md) 已完成 STEP-0118 遗留交付物 (a)：`sico:script/http@0.2.0` 在 runner 内 guest-visible——provider 新增流式读取/连接池/幂等重试/流式上传与确定性 TLS HTTP 服务器 fixture，runner 链接 0.2.0 接口（缓冲、流式读、流式上传、secret 注入、per-Store 池与 abandoned fail-closed），CLI 新增 `--allow-endpoint/--http-trust-roots/--secret-file`，6 个手编 guest Component 测试全绿（池复用 1 连接 2 请求、chunked 字节精确、permission/protocol typed 拒绝、secret Host 侧注入、run 级取消）。源码层 import emission 留给 M14。交付物 (b) Linux x64 语料重跑为 STEP-0126。

[`STEP-0075`](./steps/STEP-0075-script-profile-contract.md)–[`STEP-0084`](./steps/STEP-0084-m8-exit-audit.md) 已完成 M8，出口审计为 GO。M9 的 [`STEP-0085`](./steps/STEP-0085-streaming-script-rfc.md)–[`STEP-0094`](./steps/STEP-0094-m9-exit-audit.md) 也已完成并发出 GO；M9 Runtime execution 只在 Windows x64 上实证。M10 [`STEP-0095`](./steps/STEP-0095-observability-debug-contract.md)–[`STEP-0102`](./steps/STEP-0102-m10-exit-audit.md) 已全部完成：contract、debug triplet、typed source faults、Windows console/client cancellation、bounded task-aware events、exact DAP subset（12/20/6）与 editor/AI data boundary，exit audit 在重跑 M0–M9 aggregate 后发出 GO（[`M10 exit audit`](./reports/m10-exit-audit-v0.md)，Windows x64 GNU）。M11 STEP-0103 设计轨已完成 [`ADR-0010`](./adr/ADR-0010-single-store-structured-concurrency.md)（accepted-design），STEP-0104 完成 [`RFC-0036`](./rfc/RFC-0036-structured-concurrency-semantic-ir-v0.md) semantic/IR contract，STEP-0105 完成 scheduler core，STEP-0106 完成 cancellation tree/race/select，STEP-0107 完成 bounded channels；下一项为 STEP-0108 persistent runner, watch, REPL and DAP task integration。

## 3. Verified repository facts

基线检查时间：2026-07-29。

| 事实 | 结果 | 状态 |
|---|---:|---|
| `.sico` 文件 | 214 | measured |
| 代表性程序 | 10 | verified by `examples/` |
| M9 stream throughput | 1/16/256 MiB exact；256 MiB peak RSS 10.8 MiB | Windows runtime-verified |
| M9 blocked I/O cancellation | read/pump/write exit 123；127/124/136 ms total incl. 100 ms timer | Windows runtime-verified |
| M9 persistent watch | one PID generations 1/2/3；exits 0/122/0；warm median 5.994 ms | Windows runtime-verified |
| M9 REPL bounds | 4 KiB line；256 cells；16 KiB history；7.8 MiB peak RSS | Windows runtime-verified |
| P0 A0 语义案例 | 58 | verified by semantic case validator |
| 候选 B 案例 | 58 | verified by semantic case validator |
| 候选 C 案例 | 58 | verified by semantic case validator |
| 单点语法错误变体 | 36 | verified by mutation validator |
| 固定 AI 评测任务 | 96 | verified by AI evaluation validator |
| 稳定诊断 code/key | 41（其中 13 个由 parser 产生） | verified by diagnostic validator |
| 已映射非法 case | 33 | verified by diagnostic validator |
| Rust `.rs` 文件 | 111 | measured |
| `Cargo.toml` | 34 | measured |
| 数值原型单元测试 | 10 passed | verified |
| resource/async 动态测试 | 10 passed | verified |
| WASI 0.3 WIT parser 测试 | 1 passed | verified |
| 真实 WebAssembly Component | 2 | verified |
| Component sync/async 重跑 | 2 + 2 passed | verified |
| 4096-bit/Decimal ABI roundtrip | 512 bytes + true | verified |
| 正式编译器 workspace | 12 crates, build/test pass | verified |
| source/line index | 6 tests passed | verified |
| lossless lexer | 8 tests; B 58/58 | verified |
| B happy-path parser | 58/58 + 58 snapshots | verified |
| parser recovery/E1xxx | 12/12 mutation；E1001–E1012；top-level E1013 | verified |
| canonical formatter | 58/58 AST stable + idempotent | verified |
| `sico` CLI | 6 commands；8 integration tests；`.sapp` build/run/inspect | verified |
| deterministic frontend properties | 8,192 inputs | verified |
| parser limits | depth 256；diagnostics 100 | verified |
| M1 performance | median 1061 ms / 4.190 MiB/s；no SLA | measured |
| 实际 `.sico` core/nominal type-check | 7 valid pass；9 invalid exact primary | verified |
| 实际 `.sico` match/Result check | 6 valid pass；8 invalid exact primary | verified |
| 实际 `.sico` capability/Component check | 4 valid pass；4 invalid exact primary | verified |
| 实际 `.sico` resource/task/stream check | 6 valid pass；6 invalid exact primary | verified |
| 实际 `.sico` revision check | 2 valid pass；2 invalid exact primary | verified |
| compiler-produced Semantic Index | 10 complete B modules；10 honest partial A modules；5 queries | verified |
| semantic CLI full oracle | 25/25 valid；33/33 exact invalid；text/JSON | verified |
| deterministic semantic properties/limits | 2,048 inputs；diagnostics 100；locals 2,000；depth 200 | verified |
| M2 performance | median 1242.186 ms / 2.808 MiB/s；no SLA | measured |
| typed Sico IR v0 | 14 types；19 operations；5 terminators | verified |
| independent IR verifier | 9 mutation classes；diagnostic cap 100 | verified |
| deterministic core lowering | 12 core valid lowered；7 task/flow cases；19/6 cumulative；33 invalid blocked | verified |
| effect/resource/revision/task lowering | 7 flow cases；cumulative valid 19/25；task-scope table + 4 ops；verifier task mutations | verified |
| deterministic Core Wasm backend | 2 byte-identical artifacts；wasmparser + Node engine；42/7/9 | verified |
| compiler-generated Component | 2 byte-identical artifacts；wasmparser + Wasmtime 46.0.1；42 | verified |
| WIT Result/record/resource boundary | Ok/Err；512-byte Int；Decimal；owned/borrowed/drop；host output 50 | verified |
| future/stream Runtime contract | 3 Components；roundtrip/close/cancel；capacity 1/5；stable SHA-256 | verified |
| async compiler boundary | Task/Future/Stream 3 feature-specific refusals | verified |
| end-to-end CLI | file/stdin build；Int/Bool/Unit Wasmtime run；exit 0/1/2；failure no artifact | verified |
| M3 deterministic quality | 2,048 scalar sources；1,000-function Component；3 Runtime cases | verified |
| M3 performance | median 51.055 ms / 3.213 MiB/s；3 × 3,000 builds；no SLA | measured |
| M4 Runtime limits | 11 dimensions；7 fault classes；6 tests；真实 infinite-loop + host survival | verified |
| M4 package CLI/cache | deterministic `.sapp`；signed inspect/trusted run；corrupt cache refusal | verified |
| M4 security properties | 2,048 signed mutations；1,024 Component mutations；256 resource permutations | verified |
| M4 performance | build/verify/signature median 5.459/8.571/48.177 ms；3 × 1,000；no SLA | measured |
| M5 Host install/open | signed-only；app+signer/revision/capability hashes；3 integration tests | verified |
| M5 Windows Desktop Host | signed install/open/Wasmtime `42`/uninstall；native Forms；zip smoke | runtime-verified |
| M5 desktop adapters | Windows runtime；macOS/Linux generated association artifacts | contract-verified where no runner |
| M5 host properties | 10,240 identity/capability/UI/open inputs | verified |
| M5 startup performance | median mean 32.344 ms；3 × 20 release launches；no SLA | measured |
| M10 deterministic debug triplet | atomic Component/map/identity；real Core offsets；byte-identical rebuilds | verified |
| M10 typed Runtime faults | 9 stable classes；exact verified source frames；256-frame bound | Windows runtime-verified |
| M10 bounded execution events | 64 KiB chunks；1 MiB/channel capture；256-event/4 MiB queue；explicit overflow marker | verified |
| M10 minimal DAP | 12 supported requests；20 typed refusals；6 events；real entry/source breakpoint、pause、nested stack、scalar locals | Windows runtime-verified |
| M10 session isolation | 100 sequential DAP sessions；handles 92 → 92；bounded RSS | measured |
| M10 exit audit | M0–M9 aggregate green；Windows x64 GNU only；external gates unchanged | verified |

M0/M1/M2 已完成。STEP-0022–0029 证明完整 B HIR、25/25 valid、29/29 exact invalid、semantic CLI、compiler index/query 与有界质量基线；[`M2 exit audit`](./reports/m2-exit-audit.md) 已授权进入 M3 STEP-0030。M11 STEP-0104 已完成 [`RFC-0036`](./rfc/RFC-0036-structured-concurrency-semantic-ir-v0.md) 语义/IR 契约：E5003/E5103–E5105 四个新稳定诊断、忠实 task IR lowering（19 lowered / 6 typed-refused / 33 invalid blocked）、sequential-v1 codegen 投影与 `collect_tasks` 执行证据，M9 顺序行为逐字节保持。M11 STEP-0105 已在 `sico-runner` 内落地 ADR-0010 的有界 scheduler core：1,024 live-task 表、64 层 scope 树、1,024 FIFO ready queue、1,024 records / 16 MiB identity-checked Host completion ingress 与 16 MiB metadata 预算；1/2/16/256/1,024-task 合成与真实 guest 工作负载均在默认 run bounds 内执行，全部 limit+1 为稳定 typed faults，stale/duplicate/cross-run 完成记录 fail-closed，101 次重复运行无句柄/RSS 增长（Windows 实测 93→93 handles）。M11 STEP-0106 在 scheduler core 之上完成 downward cancellation tree（reverse-ownership commit、pending Host operation abandon、不上行）、scheduler 级 race/select（≤256 operands、canonical winner 与 wall-clock 无关、loser 确定取消）、timer readiness 与 typed deadlock 检测；completion-vs-cancel、timeout-vs-cancel、parent-vs-child failure、simultaneous-ready 四个矩阵均为单一结果，1,024 深任务链取消在毫秒级完成（release 实测 6.7 ms）。M11 STEP-0107 为 scheduler core 增加 task-aware bounded channels：每 channel 0..=1,024 items + 显式字节预算（≤16 MiB）、rendezvous（capacity 0）、FIFO waiter 队列与 suspend 式反压（被阻塞生产者不在 ready queue，无 busy spin）、owner-only close/drain、affine move、use-after-close/double-close/越界预算全部 typed；取消任务会移出 wait queue 并 failure-close 其拥有的 channel，teardown 按 idempotent cleanup path 反向关闭剩余 channel；1 GiB relay 通过 4-item/4 MiB channel 时 scheduler metadata 恒定（368 B）、进程 RSS 平稳。

## 4. Completed assets

- 方向与非目标：[`DIRECTION.md`](../DIRECTION.md)；
- 工程与阶段架构：[`DEVELOPMENT.md`](../DEVELOPMENT.md)；
- 核心语义草案：[`SEMANTICS.md`](../SEMANTICS.md)；
- 候选语法与评测方法：[`SYNTAX.md`](../SYNTAX.md)；
- 10 个代表性程序与问题矩阵：[`examples/`](../examples/README.md)；
- 候选 A 跨样本语义审计：[`examples/SEMANTICS-AUDIT.md`](../examples/SEMANTICS-AUDIT.md)；
- 58 个 P0 正反例：[`semantic-cases/`](../semantic-cases/README.md)；
- A0/B/C 的 162 个一一对应案例与完整静态指标：[`syntax-candidates/`](../syntax-candidates/README.md)；
- 36 个可复现单点结构错误变体：[`syntax-mutations/`](../syntax-mutations/README.md)；
- 96 项 AI v1 评测协议与离线执行器：[`ai-eval/`](../ai-eval/README.md)；
- 41 个稳定诊断（12 个真实 syntax、29 个 M2/M11 设计）、12 个 syntax mutation 与 33 个 semantic case 映射、JSON Schema 和校验器：[`diagnostics/`](../diagnostics/README.md)；
- 10 模块 Semantic Index fixture、五类查询 JSON v0 与离线校验器：[`semantic-index/`](../semantic-index/README.md)；
- `Int`/Decimal Rust 原型、WIT 候选、RFC 和可复现报告：[`prototypes/numeric/`](../prototypes/numeric/README.md)；
- resource/async Rust 原型、compile-fail、WASI 0.3 WIT、RFC 和报告：[`prototypes/resource-async/`](../prototypes/resource-async/README.md)；
- 真实 Component host call、resource、数值记录和原生 async 往返：[`prototypes/component-host-call/`](../prototypes/component-host-call/README.md)；
- 桌面/Android Runtime 基线、发布政策与 Android 最小探针：[`ADR-0002`](./adr/ADR-0002-runtime-platform-baseline.md)、[`runtime report`](./reports/runtime-desktop-android-v0.md)；
- M1 B labeled-block 语法基线、候选取舍与复审门槛：[`RFC-0005`](./rfc/RFC-0005-labeled-block-syntax-baseline.md)；
- 12 类 AI-oriented 错误 taxonomy（真实频率未测量）：[`error-taxonomy.json`](../ai-eval/error-taxonomy.json)；
- M0 requirement-by-requirement GO 结论与递延登记：[`M0 exit audit`](./reports/m0-exit-audit.md)；
- STEP-0015–0021 前端工程计划：[`M1 compiler frontend`](./plans/M1-compiler-frontend.md)；
- 正式 Rust workspace、lexical/source v0 与 21 个 contract case：[`STEP-0015`](./steps/STEP-0015-compiler-workspace-lexical-source.md)、[`RFC-0006`](./rfc/RFC-0006-lexical-source-contract-v0.md)；
- strict source/span、line index 与 58-file lossless lexer：[`STEP-0016`](./steps/STEP-0016-source-span-lossless-lexer.md)；
- B happy-path lossless parser 与 AST shape：[`STEP-0017`](./steps/STEP-0017-b-grammar-lossless-parser.md)；
- parser recovery、E1001–E1012 与 text/JSON span：[`STEP-0018`](./steps/STEP-0018-parser-recovery-syntax-diagnostics.md)；top-level E1013：[`STEP-0092`](./steps/STEP-0092-top-level-script-syntax-decision.md)；
- canonical formatter、comment/trivia policy 与 error-tree refusal：[`STEP-0019`](./steps/STEP-0019-canonical-formatter.md)；
- `sico check`/`format`/`outline`、退出码与 M2 capability boundary：[`STEP-0020`](./steps/STEP-0020-cli-check-format-outline.md)；
- frontend fuzz/limits/performance 与 M1 GO：[`STEP-0021`](./steps/STEP-0021-fuzz-performance-m1-exit.md)、[`M1 exit audit`](./reports/m1-exit-audit.md)；
- STEP-0022–0029 静态语义执行计划：[`M2 static semantics`](./plans/M2-static-semantics.md)；
- full B HIR/name/prelude contract：[`STEP-0022`](./steps/STEP-0022-full-hir-name-prelude-contract.md)、[`review report`](./reports/full-hir-name-prelude-v0.md)；
- core/nominal type checker：[`STEP-0023`](./steps/STEP-0023-core-nominal-types.md)、[`review report`](./reports/core-nominal-types-v0.md)；
- match/Result control flow：[`STEP-0024`](./steps/STEP-0024-match-result-control-flow.md)、[`review report`](./reports/match-result-control-flow-v0.md)；
- effects/capabilities/Component boundary：[`STEP-0025`](./steps/STEP-0025-effects-capabilities-component.md)、[`review report`](./reports/effects-capabilities-component-v0.md)；
- resource/task/stream flow：[`STEP-0026`](./steps/STEP-0026-resources-async-streams.md)、[`review report`](./reports/resources-async-streams-v0.md)；
- revision contract dataflow：[`STEP-0027`](./steps/STEP-0027-revision-contract-dataflow.md)、[`review report`](./reports/revision-contract-dataflow-v0.md)；
- compiler Semantic Index/query：[`STEP-0028`](./steps/STEP-0028-compiler-semantic-index.md)、[`review report`](./reports/compiler-semantic-index-v0.md)；
- semantic CLI、quality baseline 与 M2 GO：[`STEP-0029`](./steps/STEP-0029-semantic-cli-fuzz-m2-exit.md)、[`review report`](./reports/semantic-cli-fuzz-performance-v0.md)、[`M2 exit audit`](./reports/m2-exit-audit.md)；
- STEP-0030–0037 IR/Component 执行计划：[`M3 Sico IR and Component`](./plans/M3-sico-ir-component.md)；
- typed Sico IR contract/verifier：[`STEP-0030`](./steps/STEP-0030-typed-sico-ir-contract-verifier.md)、[`RFC-0008`](./rfc/RFC-0008-typed-sico-ir-contract-v0.md)、[`review report`](./reports/typed-sico-ir-contract-v0.md)；
- core lowering/evaluation order：[`STEP-0031`](./steps/STEP-0031-core-lowering-evaluation-order.md)、[`RFC-0009`](./rfc/RFC-0009-core-lowering-evaluation-order-v0.md)、[`review report`](./reports/core-lowering-evaluation-order-v0.md)；
- effect/resource/revision IR flow：[`STEP-0032`](./steps/STEP-0032-effect-resource-revision-flow.md)、[`RFC-0010`](./rfc/RFC-0010-effect-resource-revision-ir-flow-v0.md)、[`review report`](./reports/effect-resource-revision-ir-flow-v0.md)；
- deterministic Core Wasm backend：[`STEP-0033`](./steps/STEP-0033-deterministic-core-wasm-backend.md)、[`RFC-0011`](./rfc/RFC-0011-deterministic-core-wasm-backend-v0.md)、[`review report`](./reports/deterministic-core-wasm-backend-v0.md)；
- Component/WIT boundary：[`STEP-0034`](./steps/STEP-0034-component-wit-boundary.md)、[`RFC-0012`](./rfc/RFC-0012-component-wit-boundary-v0.md)、[`review report`](./reports/component-wit-boundary-v0.md)；
- async/task/stream backend contract：[`STEP-0035`](./steps/STEP-0035-async-task-stream-backend.md)、[`RFC-0013`](./rfc/RFC-0013-async-backend-support-v0.md)、[`review report`](./reports/async-task-stream-backend-v0.md)；
- minimal end-to-end CLI：[`STEP-0036`](./steps/STEP-0036-minimal-end-to-end-cli.md)、[`RFC-0014`](./rfc/RFC-0014-minimal-build-run-cli-v0.md)、[`review report`](./reports/minimal-build-run-cli-v0.md)；
- M3 quality/exit GO 与 M4 execution plan：[`STEP-0037`](./steps/STEP-0037-m3-quality-exit-audit.md)、[`M3 exit audit`](./reports/m3-exit-audit.md)、[`M4 plan`](./plans/M4-sapp-runtime.md)；
- `.sapp` format/builder/trust/capability/storage/limits：[`STEP-0038–0043`](./steps/README.md)、[`RFC-0015–0018`](./rfc/README.md)、[`ADR-0003`](./adr/ADR-0003-isolated-storage-wasi-host-v0.md)；
- package CLI/cache 与 M4 GO：[`STEP-0044`](./steps/STEP-0044-package-cli-source-cache.md)、[`STEP-0045`](./steps/STEP-0045-m4-security-quality-exit.md)、[`M4 exit audit`](./reports/m4-exit-audit.md)；
- STEP-0046–0053 Desktop Host execution plan：[`M5 plan`](./plans/M5-desktop-host.md)；
- 长期自治执行目标：[`AGENT_GOAL.md`](../AGENT_GOAL.md)。

## 5. Incomplete M0 work

无。真实 AI、Android、Future/Stream Runtime 与 proposed RFC 接受条件已登记为后续/外部证据，不是 M0 完成声明。

## 6. Blockers

M6 当前被外部 runner 阻塞：本机没有已授权 Android SDK/NDK、ADB、x86_64 emulator/AVD 或 arm64 device。仓库也尚无可构建 Gradle Android Host、`HostActivity`、JNI `cdylib` 或 APK。用户接受许可并提供 runner 后，仍需先补齐这些实现，不能直接把 cross-check 视为 Runtime evidence。

2026-07-16 复查还确认当前 stable Rust 1.97.0 未安装 Android targets；仓库固定的 version-named toolchain 未单独安装，但同版本 stable toolchain 可用于平台无关工作。Harmony 没有 SDK、DevEco、仓库计划或实现；当前状态是 deferred scope，而不是已完成或已验证的目标。

真实 AI API 批量评测仍需要模型凭据和成本授权；协议和离线工具已经完成，因此该条件不阻塞 STEP-0015/M1。没有真实调用前不产生模型分数。

## 7. Risks

- B frontend 与确定性 fuzz/limit 已完成，但 coverage-guided 长期 fuzz 和跨平台性能仍是后续质量工作；
- `Int`/Decimal 记录已通过 Rust 原型和真实 Component 往返，但 RFC-0003 仍待非 Windows 重现与稳定限额诊断分类；contract invariant 和 Result 表层写法仍是草案；
- STEP-0023–0029 已有真实 checker/index/CLI；仍无真实模型实测；
- WIT 0.253 已真实往返 resource、数值记录、`async func`、`future<T>` 与 `stream<T>`；M9 compiler 已实现 parallelism-1 sequential Task scope 与 typed cancellation edge，真正并行 scheduler、first-class Future/Stream 和 task collection 仍保持专用 refusal；
- Core Wasm、compiler-generated Component 与同步 scalar CLI 已由独立 engine/selected Wasmtime 执行；一般 aggregate/import adapter、package capability host 与 sandbox 留在 M4；
- Wasmtime Android aarch64/x86_64 仍是 Tier 3；Pulley/真机/JNI/商店政策只有 M0 选择，尚无仓库实测；
- E1xxx 与 catalog-mapped E2–E7 已有真实 compiler code、跨度、bounded cascade 与 CLI text/JSON；
- 语义查询已有 compiler producer 与五类直接查询；跨包/IR facts、accuracy/latency 和真实模型收益仍未测量。
- M10 DAP/debug 已把 runner 固定到 Wasmtime 47.0.2（unaligned debug-frame reads 修复）；未来引擎升级必须重跑 exact DAP 与 mapping 证据，process-global console 测试保持串行；
- M10 非 SLA cold/warm launch 目标未达（P95 691.2816/138.9047 ms），已记录为待更多主机采样，不构成 correctness failure。

## 8. Next step

下一项执行 M12 STEP-0126：在 WSL2 Ubuntu-24.04 上重跑 sico-http-provider 全部语料与 runner http2 语料（Linux x64 native），随后 STEP-0127 复审 M12 exit gates（gate 7 pooling/retry 已由 STEP-0125 关闭）并发出 M12 完整 GO/NO-GO。实现期间不得改变 RFC-0037 冻结的 authority/TLS/framing/redirect/secret 语义与 STEP-0125 的 typed-error 边界。公网 rollout、真实 pilot/model 与移动平台仍服从各自外部 gate；源码层 http@0.2.0 import emission 属于 M14 应用就绪语言基线。
