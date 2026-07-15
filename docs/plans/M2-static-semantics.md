# M2 plan: static semantics

> - status: in progress (STEP-0023 complete)
> - created: 2026-07-15
> - phase: M2
> - entry evidence: [`M1 exit audit`](../reports/m1-exit-audit.md)
> - language baseline: RFC-0005 + RFC-0006
> - semantic oracle: 25 valid + 29 invalid P0 cases

## 1. Outcome

交付第一个真实 Sico static semantics pipeline：从无错误 B syntax tree 建立完整结构 AST/HIR，解析名称，检查核心/名义类型、Option/Result、match、效果/能力、affine resource、Future/Task/Stream、Component 与 revision 规则，产生 E2xxx–E7xxx text/JSON diagnostics 和初步 Semantic Index。M2 不生成 Sico IR/Wasm，不运行程序。

## 2. Entry conditions and decision gates

- M1 exit gate 已通过，error tree 不能进入 semantic lowering；
- 54 个 B case 与 `diagnostics/semantic-case-map.json` 是最低可执行 oracle；
- 只实现现有 P0 case 和已确认/明确草案语义所需子集；
- prelude 类型/函数签名、名称空间、泛型约束或控制流规则若无法从现有文档唯一推出，STEP-0022 先补 RFC/case；
- RFC-0003/0004 的 proposed 部分不因实现方便自动升级；Future/Stream Component Runtime 证据仍不属于 M2 type checking；
- 新 diagnostic 必须先有最小 case、root cause、span 和 cascade 证据。

## 3. Proposed workspace additions

```text
crates/
  sico-hir/          完整结构 AST/HIR、稳定 IDs、source maps
  sico-semantics/    names、types、control flow、effects、resources
  sico-index/        RFC-0002 semantic facts/query producer
tests/
  semantics/         25 pass + 29 compile-fail goldens
  semantic-index/    compiler-produced query snapshots
```

crate 名称和内部数据结构是 STEP-0022 工程决定；不得改变 language oracle 或公开协议。

## 4. Execution sequence

| Step | Deliverable | Exit evidence |
|---|---|---|
| STEP-0022 | full B AST/HIR + source maps + name/prelude contract | 54/54 lowering snapshots；stable IDs/ranges；error tree blocked；名称空间/prelude 不确定项已 RFC 化 |
| STEP-0023 | core/nominal types、fields、local inference、invariants | numbers + nominal：7 valid pass；9 invalid 命中 E2001/E2002/E2010/E2011/E2020；无 implicit conversion |
| STEP-0024 | match/control flow + Option/Result/error mapping | match + result：6 valid pass；8 invalid 命中 E3001–E3003/E3101–E3104；root-cause cascade bounded |
| STEP-0025 | effects/capabilities + Component boundary semantics | capability + component：4 valid pass；4 invalid 命中 E4001/E4002/E6001/E6002 |
| STEP-0026 | affine resources + Future/Task/Stream checks | resource/task/stream：6 valid pass；6 invalid 命中 E5001/E5002/E5101/E5102/E5201/E5202 |
| STEP-0027 | revision/contract dataflow | revision：2 valid pass；2 invalid 命中 E7001/E7002；不引入不可见 runtime conflict handling |
| STEP-0028 | compiler-produced Semantic Index/query v0 | 10-module facts、5 operations、completeness/blocking diagnostics、stable IDs/ranges 与 RFC-0002 schema verified |
| STEP-0029 | semantic CLI integration、fuzz/performance 与 M2 exit audit | 25/25 valid，29/29 invalid exact primary code；`sico check` 不再报告 unavailable；全部 M2 gate proven |

编号在 STEP-0021 推送后可用。任何需要语言决定的发现必须插入独立 RFC/STEP，并更新本表；不能通过“先让测试过”提前固定语义。

进度：STEP-0023 已完成；core/nominal types 的 7/7 valid 与 9/9 invalid exact primary diagnostics 见 [`review report`](../reports/core-nominal-types-v0.md)。下一执行项为 STEP-0024。

## 5. Test matrix

- Unit：HIR IDs/source maps、scope/binding、type relation、flow lattice、effect/resource state；
- Golden：54 HIR shapes、29 primary diagnostics text/JSON、25 success summaries；
- Properties：deterministic IDs/order、range within source、unknown/error type suppresses dependent cascades、no error HIR lowering；
- Integration：CLI file/stdin/text/JSON、semantic index queries、syntax error remains E1xxx；
- Fuzz/limits：deep expressions/patterns、large scopes/types/control-flow graphs、diagnostic cap；
- Regression：M1 STEP-0015–0021 与 M0 validators remain green。

## 6. M2 exit gate

- 25/25 P0 valid case 通过完整 semantics；
- 29/29 invalid case 在 backend 前拒绝，主要 code/key 与 catalog map 一致；
- name/type/control/effect/resource facts 有统一 span 和 stable ID；
- dependent/cascade diagnostics 有界，不用 unknown type 制造噪音；
- `sico check` text/JSON 明确执行 semantic checks，syntax-only 状态不再冒充完整成功；
- Semantic Index 由真实 compiler facts 生成，错误时标记 completeness/blocking diagnostics；
- deterministic fuzz/property/limit 与性能基线通过；
- Sico IR、Wasm、Runtime、`run`/REPL 仍明确 unavailable。

## 7. Deferred beyond M2

Sico IR/validator、Core Wasm/Component/WIT codegen、Runtime 执行与源码直跑属于 M3；`.sapp`、缓存、签名和 sandbox 属于 M4；不得在 M2 CLI 中伪实现。
