# RFC-0012: Component and WIT boundary v0

> - status: accepted
> - date: 2026-07-15
> - authors: autonomous-agent
> - target phase: M3
> - supersedes: -
> - superseded-by: -

## Summary

冻结 M3 第一条 deterministic Core Wasm → WebAssembly Component canonical lift，以及 fixed scalar、WIT `result`、value record 和 owned/borrowed resource 的结构映射。只接受已由真实 validator 与 Wasmtime 46.0.1 证明的子集；arbitrary `Int`/Decimal 字段语义、通用 aggregate adapter 和 compiler-generated resource imports 仍保持未接受。

## Compiler-generated Component subset

- `compile_component` 必须先走 STEP-0033 的 verified-IR-only Core Wasm gate；
- 每个受支持 Core export 按 canonical IR 顺序 alias、定义 Component function type、canon lift 并 root export；
- `Bool` 使用 Component `bool`/Core `i32`，compile-time proven fitting `Int` probe 使用 Component `s64`/Core `i64`，`Unit` 无 result；
- generated Component 不需要 memory、realloc 或 post-return，因为当前仅支持 flat scalar ABI；
- aggregate、string、general arbitrary `Int`、effect/resource/revision 与 async function 必须继续 typed-refuse，不能缺少 adapter 仍宣称成功。

## WIT structural mapping

锁定的 `sico:boundary-probe@0.1.0` 只是一条审计 probe package，不是稳定应用 ABI：

- nominal value record 映射到 WIT `record`，字段顺序就是 ABI 顺序；
- Sico `Result<T, E>` 映射到 WIT `result<T, E>`，Ok/Err 不转成 trap 或整数状态码；
- owned resource 映射到 WIT `resource` owned handle，普通 method receiver 是 call-scoped borrow，drop 消费 owned handle；
- trap、domain `Result`、resource drop 与普通返回保持不同通道；
- package/interface/world 名和版本显式，不依赖 Rust layout 或 generated binding spelling。

`big-int`/`decimal-value` records 继续作为 RFC-0003 proposed encoding 的真实 roundtrip probe。该 roundtrip 只证明 WIT/Canonical ABI 保真，不接受 RFC-0003 尚未满足的跨平台限额与诊断决定。

## Runtime evidence

- compiler 生成的 scalar Component 通过 `wasmparser 0.253` Component validator；
- 官方 Wasmtime CLI 46.0.1 加载 compiler artifact，调用 `main()` 返回 `42`；
- Rust guest/Wasmtime host 使用同一正式 probe WIT，Result Ok/Err、4096-bit record、Decimal record 与 resource new/borrow/drop 均真实往返；
- 两次 Component build hash 相同，host event 顺序固定。

## Tooling and stale-artifact rule

- Wasmtime CLI archive固定官方 release URL 与 SHA-256；下载物只进入 ignored `target/tooling`；
- prototype validator 必须检查每次 Cargo build/run exit code；任何失败立即终止，禁止继续读取旧 artifact 后打印 PASS；
- nested prototypes 显式排除于正式 workspace，仍使用各自 lockfile。

## Rejected alternatives

- 把 WIT `result` 映射为 trap：丢失领域错误通道；
- 把 resource 映射为可复制 `u32`：破坏 affine ownership；
- 对 aggregate 省略 memory/realloc adapter：会生成不可验证或错误 ABI；
- 因 record roundtrip 成功就接受 arbitrary `Int`/Decimal 全部语义：越过 RFC-0003 gate；
- 使用旧缓存 artifact 掩盖 build failure：不可审计。

## Acceptance evidence

- 两个 compiler-generated Components deterministic 且通过真实 Component validator；
- selected Wasmtime Runtime 执行 compiler artifact 返回 42；
- Result Ok/Err、big-int/Decimal records、owned/borrowed/drop resource 与 host call 往返；
- malformed IR、unbounded Int 与 unsupported aggregate path 保持拒绝；
- workspace/M2/M1/M0 regression。

## Links

- [`boundary probe WIT`](../../wit/boundary-probe-v0/world.wit)
- [`RFC-0003`](./RFC-0003-numeric-representation-v0.md)
- [`RFC-0004`](./RFC-0004-resource-async-mapping-v0.md)
- [`RFC-0011`](./RFC-0011-deterministic-core-wasm-backend-v0.md)
- [`STEP-0034`](../steps/STEP-0034-component-wit-boundary.md)
