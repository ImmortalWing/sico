# RFC-0009: core lowering and evaluation order v0

> - status: accepted
> - date: 2026-07-15
> - authors: autonomous-agent
> - target phase: M3
> - supersedes: -
> - superseded-by: -

## Summary

冻结 Sico core lowering v0 的 source-order、single-evaluation 规则，以及 M3 首个可执行子集进入 typed IR 的边界。该规则不改变 M2 类型/效果判断，也不把 unsupported construct 伪装成成功 IR。

## Evaluation order

- binary operand 按 left 后 right 求值；
- function/intrinsic/constructor/variant arguments 按 source order、每项恰好一次求值；
- named record fields 按源码出现顺序求值，完成所有字段后构造值并检查 invariant；字段名不改变求值顺序；
- function callee 在 M2 已静态解析，runtime 不重复动态查找；arguments 全部求值后调用；
- `try expression` 先恰好一次求值 expression；`ok` 解包继续，`error` 原值提前返回；不得重新执行 expression；
- match scrutinees 从左到右各求值一次，arms 按 source order 测试；M2 已证明 exhaustiveness，IR 不增加 default success；
- return expression 完成后才执行 return；
- optimizer/codegen 不得跨 effect、resource、trap、try 或 call observable boundary 重排。

## Supported lowering v0

STEP-0031 支持参数/local、Int/Bool/Text constants、Int add、direct function call、`Float64.from_int`、newtype/record construction、field projection、payload-free variant、`ok`/same-error `try`，以及不需要 payload binding 的 exhaustive match。

payload-binding match、closure/map_error、if/revision、effect/capability/component call、resource/using、async/task/stream 返回 typed `Unsupported`。这些 construct 已由 M2 接受不代表本步后端支持；不得输出 partial IR。

## IR additions

- `intrinsic` operation 目前只接受 `Float64.from_int(Int) -> Float64`；
- `try` operation 只接受 `Result[Ok, Error]` 并返回 `Ok`；early error edge 是其固定 control semantics；
- `match` terminator 包含一次求值后的 values、有序 pattern arms 和显式 target blocks；
- Pattern v0 区分 wildcard、binding、Bool 与 named variant。STEP-0031 拒绝需要把 binding 传入 arm block 的 source，直到显式 block parameter contract 存在。

## Rejected alternatives

- 依赖 Rust/Wasm 未指定 operand order：拒绝，source semantics 必须独立；
- record fields 按声明或名称排序求值：拒绝，会改变 source-observable call/effect order；
- 为 unsupported source 生成 `unreachable`：拒绝，会把 compiler capability gap 变成 runtime trap；
- 重复执行 `try`/match scrutinee：拒绝，会重复 effect 或资源消费；
- 在本步为 match binding 引入隐式跨 block capture：拒绝，违反 RFC-0008 verifier contract。

## Acceptance evidence

- 12 个现有 valid B core cases 生成 verified deterministic IR snapshot；
- 其余 13 个 valid cases typed-refuse，29 个 invalid cases仍由精确 M2 diagnostic 先拒绝；
- constructor fields 与 binary operands 的 instruction 顺序证明 left-to-right single evaluation；
- intrinsic/try/match verifier mutation 被独立拒绝；
- workspace/M2/M1/M0 regression。

## Links

- [`RFC-0008`](./RFC-0008-typed-sico-ir-contract-v0.md)
- [`STEP-0031`](../steps/STEP-0031-core-lowering-evaluation-order.md)
