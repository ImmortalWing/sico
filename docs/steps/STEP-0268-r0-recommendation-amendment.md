# STEP-0268: R0 recommendation amendment — owner criteria evaluation

> - status: complete
> - phase: M22 R-phase (R0)
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent (criteria directive by owner, same-day)
> - evidence class: planning-only (decision-criteria evaluation)

## 1. Objective

记录 owner 决策标准指令（2026-09-24「我只看最终效果和稳定性、长期可维护
性」）对 ADR-0017 推荐的修订，保证推荐翻转留有审计痕迹（不静默改草案）。

## 2. Criteria evaluation

按 owner 三维度重评 R0 两选项（完整论证见 ADR-0017 §Considered options）：

- **最终效果**：Option A 完胜——终点是带真实结构数据模型的语言，自举源
  层大幅缩水，M14 应用基线与 AI 生成人机工学解锁；B 的终点保留全部仪式
  税与 12.7k 行 SOA 基础设施。
- **稳定性**：分 horizons——短期 B 稳（现有产物虽脆但在工作），长期归
  A；A 动 Rust 内核语言面但有 RFC-0033 全套证据门与冻结语料兜着，且
  M22 现为 NO-GO，延迟 S6 不损失工作状态。
- **长期可维护性**：A 决定性胜出，且 **B 并不避免重写**——M14/M23 对
  records 的需求压力不受 B 影响（M23 四项未含 records 是未排期，不是不
  需要），B 的"永久"大概率是"维护数月 SOA 后照样 re-baseline"；B = SOA
  维护 + 迟到的重写，A = 只重写一次 + 终局单一数据模型。

原推荐 B 的前提（成本最小化默认值）被 owner 价值函数取代；R1 数字
（燃料无悬崖）关闭了性能紧迫性，把决策完全交还产品维度——结论翻转
为 **Option A（records RFC 先行）**。

## 3. Changes

- `docs/adr/ADR-0017` 全面修订：status 行记修正案、Context 补 owner 指令、
  Decision drivers 改为 owner 加权、推荐翻转为 Option A（B 降为 fallback
  并写明其被支配的理由）、Consequences/Validation/Revisit conditions 对
  齐；Option 分析正文保留，修订可追溯。
- ADR README 登记行同步；STATUS next-step 与 M22 计划状态头指针更新。

## 4. Validation

- 修订不触碰已接受合同（ADR-0015/0016 原样引用）；ADR-0017 仍处
  proposed，等 owner 显式接受 Option A（或改选 B）。
- `tools/validate-step-0124.ps1` 通过；`git diff --check` 通过。

## 5. Metrics

无执行指标。

## 6. Risks and follow-ups

- A 被接受后的顺序：records RFC 起草（RFC-0033 四证据 + 双实测消费者：
  selfhost 语料 + 应用 profile 语料）→ RFC 接受 → 语言面实现（Rust 内核
  全链）→ M22 Route-B 式 re-baseline。SOA 不变式在 re-baseline 落地前
  持续强制，W1 合并债在 RFC 期间照还（与数据模型无关的项：sha 门、
  指纹规则、双 lexer 合并）。
- 下一步：owner 对 ADR-0017（Option A 推荐）拍板；接受即开始 records
  RFC 起草。
