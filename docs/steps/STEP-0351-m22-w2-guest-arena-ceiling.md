# STEP-0351 — M22 W2: guest 线性内存上限 64→128 MiB（RFC-0029 预算表修订），自宿主全源降低解锁

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（S6 前置能力）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**根因定位**（0350 登记的"三个 trap"复查）：

1. 四结局探针（OK / REFUSE / DOMAIN / REAL-TRAP）复检：纯 return split_lines
   与缺被调用例实为 `RunOutcome::Domain` typed refusal，**探针误标为 trap**，
   机器行为正确（0350 文档已附更正）。
2. 唯一真实 trap：**约 52 函数的 fixture 上 guest 线性内存耗尽**。wasm 帧
   34(main)→207(scalar_ir)→208/220(codegen 生成辅助)；内层 `unreachable` =
   `fs_transport_module` realloc 的 ARENA_LIMIT 检查（canonical.rs）。bump
   指针是 wasm 可变全局（GlobalSet(0)），整个 run 累计不清零——跨函数顺序
   依赖由此而来（lex_words 大分配在前、call_instruction_json 在后触顶）。
   逐函数去除二分证实：去掉任一"必需"函数即 OK 或正常 DOMAIN，无一仍 trap；
   提升上限后原 trap fixture 直接 OK。

**修复 = 预算表修订**（非机器逻辑缺陷，64 MiB 是 RFC-0029 冻结的上限）：

- `sico-codegen-wasm/canonical.rs`：ARENA_LIMIT 64→**128 MiB**，
  MEMORY_PAGES 1024→**2048**（min=max 固定内存）。
- `sico-runtime/canonical.rs`：宿主侧 ARENA_LIMIT 同步 128 MiB。
- runner/runtime 的默认 `memory_bytes` 授权同步 128 MiB（64 MiB 授权无法
  启动新组件，api_agent_pilot 曾以 MemoryLimit 暴露此依赖）。
- `tests/wasm/artifacts.hex`：确定性组件快照按流程重生成
  （SICO_DUMP_WASM=1；无内存节的核心组件字节不变——numeric/control
  逐字节相同，改动只影响声明内存的组件）。
- 128 MiB 幅度依据：68 MiB 已可通过全源降低卫兵（真实需求略超 64），
  取 2× 为剩余 63 函数翻绿与 S6 自编译迭代留余量；256 MiB 验证过亦通过。

**解锁（本片无函数翻绿，canary 0，R3 0）**：256/128 MiB 下
**完整 parser.sico（122 函数，~160 KB）整源降低不再 trap**——机器走过全部
函数后在当前前沿以 typed `ERR:E-SH-IR-CALL-ARGUMENT` 拒绝（全源差分的
S6 前提就位）；census fixpoint 恢复运行（54 翻基线，13 个真实拒绝类 +
约 40 个缺被调 ORACLE-REFUSE，守门人 = word_span_offset/u64_list_at/
gw_decimal_value/gw_instr_count 等）。

## 2. 前沿更新

- 修正 0350 的前沿记录：真实缺口只有一类——机器降低能力（typed refusal
  frontier），无健全性 trap。
- 剩余 63 的拒绝类地图（census r1）：CALL-ARGUMENT（fixed_operation_
  instruction_json、gw_map_empty_rhs_packed、record_named_type、summarize）、
  EXPRESSION（word_span_offset、parameters_ir、operand_nonparam_kind、
  argument_instruction_count）、RETURN-COUNT（fixed_operation_operand_atom_idx、
  gw_instr_count）、MATCH-SHAPE（u64_list_at）、CALL-TARGET（gw_decimal_value，
  缺被调 gw_instr_count）、STATEMENT（straight_environment_operand_check）；
  其余为缺被调链（解锁上述守门人后重跑 census 连锁）。

## 3. 验证与预算

- `--test selfhost_compiler`：**63/63 绿**（新增 arena 卫兵：原 trap 组合
  53 函数字节全等 + 全 parser.sico 走到 typed CALL-ARGUMENT）。
- 完整 runner 工作区套件：34 个测试目标 ok、0 失败；dap RSS/句柄两测试在
  干净树上同flake（环境敏感，非本片回归——HEAD 上复现 3 项失败含同两项）。
- 根工作区受影响 crate：`-p sico-runtime -p sico-codegen-wasm` 全绿
  （快照更新后 13+7 全过）。
- `cargo fmt --check` 绿；`validate-module-boundaries.ps1` OK；
  `git diff --check` 绿。探针按纪律删除。
- selfhost 共享运行限额 fuel 5e9→20e9、timeout 30s→120s（全源降低需要；
  既有测试余量增加无行为影响）。
