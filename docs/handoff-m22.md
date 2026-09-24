# M22 交接手稿（living handoff，基线 STEP-0278）

> - 更新：2026-09-24（STEP-0278 落地后、owner 指令「提交推送」批次）
> - 前一份：[`handoff-m22-line-tokens.md`](./handoff-m22-line-tokens.md)（STEP-0260 基线，已被本文取代，同批删除）
> - 权威事实仍以 [`docs/steps`](./steps/README.md) / [`docs/STATUS.md`](./STATUS.md) 为准；本文只管「下一刀从哪下、哪些验证还没闭环」

## 0. 一句话状态

M22 处于 **R 阶段后半 + W1 合并债偿还中**：R0 已拍板（Option A，ADR-0017），
records v0 全封（STEP-0271..0275），在途 WIP 已吸收、STEP-0272 微分回归已修
（STEP-0276），W1 四债已完成 2/4（sha 门关闭 + legacy 三件套退役，STEP-0278）。
下一刀 = W1 片 C（E7002 指纹改结构性）。

## 1. 本批已落地（自 fd802c4 / STEP-0262 起，全部随本次提交进库）

| STEP | 内容 | 证据 |
|---|---|---|
| 0264 | M22–M26 路线重划（R 阶段收敛门 + R3 止损预登记） | planning-only |
| 0265 | 重启评估（不回退，分层重启；W1/W2/W3 波次） | [`M22 restart assessment v1`](./plans/M22-restart-assessment-v1.md) |
| 0266/0268 | R1 canary/燃料仪器化实测；R0 推荐修订（Option A） | tools/report-m22-canary.ps1 等 |
| 0267/0269/0270 | ADR-0017 起草 → RFC-0047 草案 → accepted + EC-4 双普查冻结 | ADR/RFC 文档 + census json |
| 0271–0275 | records v0 全封：EC-1 语料/前端、ABI 扁平化 + `Module.records`、EC-2 拒绝面 + EC-5 矩阵、D5 `List[record]` 列式单态、for 迭代 + 深度核对 | record_types.rs + 11 个 end-to-end 语料；三 crate test/clippy/fmt 绿 |
| 0276 | WIP 吸收（`Map[K,V]` 参数面 +228 行）+ guest `records` 表发射修复（6 新函数） | 本机 CLI 微分：formatter 前缀 14/16、driver --emit-ir 16/16、拒绝面 3 例 |
| 0277 | W1 四债计划 + 实测侦察 | planning-only |
| 0278 | W1 片 A+B：sha 门验证关闭（早已随 0246 落地）；legacy 三件套退役（删 947 行 + 3 单测；0215/0216/0219 改退役登记；0243-0245 suite 同步） | 本机：引用清扫 + ps1 语法 + 集成链 check 绿；CI 待裁决 |

代码面汇总：`sico-ir`/`sico-semantics`/`sico-codegen-wasm` records v0 +
script-v0 ABI 扁平化；`selfhost/parser.sico`（+WIP +records 发射，约 13,000
行）；`selfhost/` 删 legacy 三件套；`runner` 测试 +record_types.rs、
+7 行 dump_nm_region_ir 探针、删 3 个 legacy 单测；`tests/language-matrix`
+records-v0 行；工具脚本 6 个改写/更新 + 3 个 R1 仪器化脚本。

## 2. 验证状态（重要：两部分没闭环）

**本机已绿**：workspace 三 crate test/clippy/fmt（0275 批次实跑）；
records 语料 check/build/run；sico check 集成链（compiler/checker/formatter）；
STEP-0276 微分框架全套（formatter 前缀、driver 例、拒绝面）。

**待 CI 裁决（本机跑不了，勿在本地重试白等）**：
1. runner 测试仓全量（`cargo test --manifest-path runner/...`）——本机缺
   gcc/dlltool（ring/windows-sys 编译失败），**不是代码问题**；
2. 改写后的 0215/0216/0219（首次以「退役登记 + selfhost_checker」模式跑 CI）；
3. 0243/0244/0245 suite 列表变更后的 canary；
4. selfhost_compiler / selfhost_parser 全量断言（0276 只实测了代表子集：
   formatter 14/16——`close_code`/`direct_close` 为既有 GWPACK-OTHER 诚实
   拒绝、非 gated；driver 99 例中 16 例 + 拒绝面 3 例）。

## 3. 下一刀（按序）

1. **W1 片 C**：E7002 关键词指纹（`compiler_semantics.sico:582-590`）改结构
   性 + rename-safety 语料；行为期望先钉语料再动语义；改不完可退登记
   declared-subset（0277 §5 片 C）。
2. **W1 片 D**：0-indent corpus case 先行定 `same(raw, line)` 行为
   （`compiler_parser.sico:489/650`），再定 refused/structural。
3. W1 出口 → W2（Option A：selfhost 源层 records 重授权 + S3/S4 re-baseline，
   R3 止损计数在 W2 起算：连续 4 个实现 STEP canary 零增长且前沿不动即切换）。
4. guest 降阶前沿（独立于 W 波次，可做 R2 内容）：`close_code`/`direct_close`
   的 while-in-if（SKIP:GENERAL-WHILE-IF-REGION）。

注意：0277/0278 是合并债机械片，按 replan 规则**不计入** R3 止损窗口
（canary 未动是预期）；但连续只做这类片也会拖慢收敛，片 C/D 后应回主线。

## 4. 环境备忘

- `RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu`；cargo =
  `$USERPROFILE/.cargo/bin/cargo.exe`；CLI 在 `target/debug/sico.exe`。
- `sico check` 不能直查 module 文件：用入口
  `target/debug/sico.exe check selfhost/compiler.sico`（会带上 parser 模块）。
- runner CLI 预构建可用：`runner/sico-runner/target/debug/sico-runner.exe`
  （`--fuel 5000000000 prog.component.wasm [-- ARGS]`；guest 单跑约 40s 正常）。
- STEP-0276 微分框架（oracle dump + guest 组件构建 + diff 脚本）已按 0276 §5
  清理；重建命令见 STEP-0276 文档 §4（oracle = `dump_oracle_ir` 测试 +
  ORACLE_SOURCE/ORACLE_OUT 环境变量；guest = `sico build --profile script-v0`）。
- 语料冻结：corpus-v0.json 逐入口 bytes+sha256 由 selfhost_checker 守卫
  （改语料必须同步 sha）。

## 5. 纪律提醒

- 不主动 commit/push（AGENTS.md §9）；本批系 owner 2026-09-24 显式指令
  「先写个交接手稿，提交推送一下」进库。
- 冻结合同（语言/WIT）动前走 RFC；跨组件架构动前走 ADR。
- W2 之前 S3/S4 源层维持 SOA 不变式（ADR-0016 仍强制）。
