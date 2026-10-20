# STEP-0486 m22-w2 S5 component-codegen wave opening

> - status: registered（实现未开工；本 STEP 只冻结波次合同与顺序）
> - date: 2026-10-20
> - owners: autonomous-agent
> - depends: RFC-0011（byte oracle）、RFC-0015/0016、ADR-0015/0016、M22 plan §4 门 3、STEP-0484（treadmill = 内存前置条件已清）
> - gate: M22 门 3（ADR-0015 `A == B == C` + `.sapp` 链）的唯一剩余实质工作面；ADR-0016 canary 顺序第七项「deterministic Component codegen」。

## 1. 背景与定位

STEP-0484 后 M22 的证据面：census 149/149 全绿（parser.sico 全函数闭包 IR 字节等同）+ 全源 parser.sico 自举降低 exit 0 + 出口审计（2026-10-20）门 1/4/5/7 GO、门 2 PARTIAL、门 3/6 NO-GO。

门 3 欠账的实质 = **自举编译器只产出 IR JSON，不产出 Component 字节**。ADR-0015 决策 2 要求 `B = run(A, S)` 的输出是 Component artifact 且 `A == B == C` 字节全等；ADR-0016 canary 顺序固定第七项 = deterministic Component codegen。自举轨的 parser/lowering 面已闭（0483/0484），codegen 面是最后的大山。

## 2. 本波次合同（冻结项）

1. **范围**：在 Sico 源码侧实现 RFC-0011 确定性 Component 发射（wasm 编码、canonical ABI、stdlib 助手、 arena 布局——逐字节对齐 `crates/sico-codegen-wasm` 的 Rust 实现）。新增 selfhost 源码单元（建议 `selfhost/codegen.sico`；命名以首实现 STEP 登记为准），与 `parser.sico` 同走 SOA 状态纪律（ADR-0016 决策 1）。
2. **byte oracle**：`crates/sico-codegen-wasm` 的 Rust 输出是唯一直接 oracle；比较面 = 冻结语料（ADR-0016 第六项 37 条 accepted Script 语料 + formatter.sico + 全源 bundle）上 guest 发射字节 vs Rust 发射字节，逐字节全等；任何 out-of-subset 形状 typed 拒绝，禁止静默回退（ADR-0016 决策 6 顺序纪律）。
3. **无 Rust compile hostcall**（ADR-0016 决策 5）：发射全程 guest 内完成；Rust 只当 oracle，不提供服务。
4. **证据等级**：internal-fixture；全部经真实 runner 执行。Step 序遵守 canary 顺序：先 37 语料单函数级 codegen 差分，再 formatter，再全源 bundle，最后 ADR-0015 harness（A==B==C + `.sapp`）。
5. **budget**：门 4 已记录的全源降低预算（25.93s / 800B fuel / 2GiB+treadmill / 8GiB hostcall / 120s）为参照面；codegen 波次的预算记录在各实现 STEP 末尾追加，不做 SLA 主张。

## 3. 顺序与出口

- 首个实现 STEP（建议 0487 起）= codegen 骨架：固定 preamble/类型段/内存段/导出段的发射，与 Rust 输出对拍最小 fixture。
- 波次出口 = M22 门 3 全部要件（B 组件字节、A==B==C、`.sapp` 链、语料双跑）+ 复审计。
- 任何一步 typed 拒绝或 trap 阻断后续 canary 项的完成声明（ADR-0016 决策 6）。

## 4. 与既有轨的关系

- 不触碰 0484 treadmill 合同（RFC-0029 amendment）；codegen guest 同样运行在 treadmill  arena 纪律之上。
- 门 2 收尾（compiler.sico 驱动段逐源全等）与本波次并行不悖：codegen 面的 IR 输入已由 census 保证。
- 门 6（提交快照 CI）在所有实现波次之后统一执行。
