# RFC-0008: typed Sico IR contract v0

> - status: accepted
> - date: 2026-07-15
> - authors: autonomous-agent
> - target phase: M3
> - supersedes: -
> - superseded-by: -

## Summary

Sico IR v0 是确定性、强类型、显式控制流的编译器内部协议。只有通过 M2 的 source 能进入 IR builder；任何 IR 必须先由独立 verifier 接受，才能进入 codegen。canonical JSON schema marker 为 `sico.ir.v0`。

## Decisions

- module/function/block/value ID 按 source declaration 与结构顺序分配，不包含绝对路径或哈希随机盐；
- source range 使用原 source 的半开 UTF-8 byte range，所有 range 必须在 `source_len` 内；
- function 由参数、返回类型、有序 effect set、entry block 和有序 blocks 组成；
- instruction 明确 result ID/type/operation/range；terminator 明确 return/jump/branch/unreachable；
- block 内 operand 必须是函数参数或同 block 先前定义值。跨 block 值必须由后续版本引入显式 block parameter，v0 不允许隐式 SSA phi；
- verifier 独立检查 schema、限额、唯一/规范 ID、range、def-before-use、基本操作类型、call signature、effect declaration、return 和 CFG target；最多保留 100 个根因；
- invalid IR 不序列化、不进入 codegen。canonical JSON 使用固定字段顺序与声明顺序，不依赖 map/hash iteration。

## Type surface

v0 IR 区分 Unit、Bool、Int、Float64、String、nominal、Option、Result、owned/borrowed resource、Task、Future 和 Stream。存在类型不代表后端已经支持该类型。`Unknown` 不属于 verified IR。

`Int` constant 是规范十进制数学整数文本：零仅为 `0`，禁止 `-0` 和前导零。其 runtime/library 表示仍受 RFC-0003 acceptance gate 约束；本 RFC 不把任意精度 Int 偷换成 i64。Decimal 仍是名义库类型。

## Operations and phase gates

core constant/copy/add/call/construct/project/variant 可由 STEP-0031 lowering。effect/resource/revision/await/stream operations 先作为 typed contract 存在，具体 flow verifier 与 lowering 属于 STEP-0032/0035。Wasm opcode、canonical ABI、trap mapping、async scheduling 和 resource table 表示不属于 IR v0 的隐含语义。

任何新增 operation 必须同时给出 operand/result type rule、source-map rule、negative verifier case 和确定性 serialization case。

## Limits

- functions/module：10,000；
- blocks/function：100,000；
- instructions/function：1,000,000；
- verifier diagnostics：100。

这些是 compiler input/IR 防御限额，不是语言可观察业务错误。更改限额需更新测试与审查文档。

## Rejected alternatives

- 直接把 HIR token tree 交给 codegen：无法独立证明类型与 ownership invariant；
- 未验证的 builder-only IR：builder bug 会进入 Runtime artifact；
- host-native enum layout/bincode：跨版本与跨平台不稳定；
- 隐式跨 block value capture：无法局部验证 dominance；
- 在本 RFC 决定 Wasm ABI/runtime representation：会提前固定 RFC-0003/0004 未接受部分。

## Acceptance evidence

- valid identity、branch、declared effect shapes 由独立 verifier 接受；
- schema/ID/range/value/type/constant/CFG mutation 被拒绝；
- canonical JSON 重复生成 byte-identical 并可反序列化；
- syntax/semantic invalid source 在 IR construction 前被 compiler-produced error 阻止；
- workspace/M2/M1/M0 regression 继续通过。

## Links

- [`M3 plan`](../plans/M3-sico-ir-component.md)
- [`M2 exit audit`](../reports/m2-exit-audit.md)
- [`RFC-0003`](./RFC-0003-numeric-representation-v0.md)
- [`RFC-0004`](./RFC-0004-resource-async-mapping-v0.md)
