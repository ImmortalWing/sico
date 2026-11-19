# STEP-0629 — language v1 batch 4 前沿探针（Rust oracle 实测）

状态：measurement complete locally（2026-11-19，本机 `target/debug/sico.exe`，script-v0 profile）
性质：**纯测量片，零生产代码改动**；不翻任何 gate、未 commit/push。
计划：[`docs/plans/language-v1-batch4-option-variant-match.md`](../plans/language-v1-batch4-option-variant-match.md)。
探针脚本（版本化）：[`tools/probe-language-v1-batch4-frontier.ps1`](../../tools/probe-language-v1-batch4-frontier.ps1)，
fixture 由脚本自身生成到 `target/tmp/step0629/`，退出码 0 = 前沿与冻结表一致。

## 1. 目的

在 RFC-0056 起草前，用生产工具链（Rust oracle，非 M22 guest 前端）实测
enum payload match 与 Option 形状的真前沿，把计划 §2 的静态复核表升级为
实测引文，并发现静态复核未能覆盖的缺口。

## 2. 实测结果（2026-11-19 冻结，逐字引用）

| 探针 | 形状 | 实测判定（逐字） |
|---|---|---|
| p1-payload-construct | `function mk() returns Notice: return Notice.Message("hi")` | exit 2：`cannot generate Script Component for …: function mk: unsupported non-scalar result` |
| p2-wildcard-arm | enum 形参 + `case Notice.Message(_)` 通配臂 | exit 2：`function is_message: unsupported non-scalar parameter` |
| p3-binding-arm | `case Notice.Message(t)` 单元数绑定臂 | exit 2（lower）：`unsupported match payload binding at bytes 504..527` |
| p4-multi-payload-binding | `case PairBox.Pair(a, b)` 二元数绑定臂 | exit 2（lower）：`unsupported match payload binding at bytes 506..530` |
| p4b-nested-pattern-arm | `case Wrap.Box(Notice.Stop)` 嵌套（无绑定）臂 | exit 1（semantics）：`E3001 …:34:3 missing case: Wrap.Box\|Notice.Message, Wrap.Nothing\|Notice.Message, Wrap.Nothing\|Notice.Stop` |
| p5-option-some-construct | `return some(I64.literal(1))` | exit 1（semantics）：`E2031 …:24:10 unresolved call target some`（与 STEP-0494 fact-fix 一致） |
| p6-result-binding-anchor | `case ok(h)` Result 绑定（对照锚） | exit 0，组件 1170 B 构建成功 |
| p7-inline-transient | `match Notice.Message("x"):` 内联构造+通配臂，enum 值不跨函数边界 | exit 0，组件 1294 B 构建成功 |

复跑：`powershell -NoProfile -ExecutionPolicy Bypass -File tools/probe-language-v1-batch4-frontier.ps1`
→ 8/8 OK，`FRONTIER_UNCHANGED`，exit 0。

## 3. 对批次方案的三点修正（全部实证驱动）

1. **codegen 缺口比「哨兵一处」大，但比「enum 值整体不支持」小**：p1/p2 实测
   Script 发射器**拒绝 enum 类型的函数形参/返回**（`unsupported non-scalar parameter/result`，
   两个 profile 同形），而 p7 实测**瞬态 enum 值**（内联构造、立即 match、不跨函数边界）
   完整走通构建。⇒ G1/G3 的发射器工作可收窄为「match 判别 + payload 投影的
   局部布局」，不需要把用户 enum 提升为函数边界上的 first-class；
   D5/D7 的 `match sico.ui.poll_event():` 正是瞬态形状。
2. **嵌套模式不是「已可用」**：p4b 实测语义层把嵌套臂展开为**叉积**
   （`Wrap.Box|Notice.Message` 等）并按叉积做 E3001 完备性检查。G1 的 D1 若
   要支持 `some(click(handle))` 这类嵌套绑定，必须先裁决嵌套语义
   （叉积展开 vs 分层解构），不能只放开 lowering 哨兵。
3. **拒绝身份分层清晰**：lower 哨兵（p3/p4，exit 2）→ 语义 E3001（p4b）/
   E2031（p5，exit 1）→ 发射器 non-scalar（p1/p2，exit 2）。RFC-0056 的
   拒绝矩阵按这三层登记，G1 的 typed-refusal 回归测试直接引用本表逐字文本。

## 4. 诚实收窄

- 只测 **Rust oracle**；M22 guest 前端对同形状的拒绝面由
  `tools/probe-m22-frontend-frontier.ps1` 负责，本片不复测。
- p6/p7 的「构建成功」只证明**发射**成功；运行期判别字取值的正确性
  （`emit_match_condition` 路径）仍未实测——这是 G1 实现片的第一道测试任务，
  本片按零代码纪律不越界。
- p3/p4 的 byte span 随 fixture 行尾（`\n` vs `\r\n`）漂移 ±1，身份以
  消息文本为准、span 以脚本生成的 fixture 为准。
