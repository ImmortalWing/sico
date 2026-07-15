# M3 plan: Sico IR and Component

> - status: in progress (STEP-0030 complete)
> - created: 2026-07-15
> - phase: M3
> - entry evidence: [`M2 exit audit`](../reports/m2-exit-audit.md)
> - semantic baseline: STEP-0022–0029 compiler facts and diagnostics

## 1. Outcome

交付第一条可验证的 Sico semantics → typed IR → deterministic Core Wasm/Component → real Runtime 链路，并让最小端到端 CLI 程序与现有 semantic oracle 一致。M3 不设计 `.sapp` 包、安全签名、sandbox、Desktop/Android host 或完整 REPL。

## 2. Entry conditions and decision gates

- M2 exit gate 必须保持通过；invalid semantic input 不得进入 lowering；
- IR type/effect/resource invariants 必须由 verifier 独立检查，不能只依赖 builder 正确；
- evaluation order、trap/error mapping、numeric representation、resource ownership、canonical ABI 或 async lowering 若不能从已接受 RFC/case 唯一推出，必须先新增 RFC/case；
- RFC-0003/0004 的 proposed 部分不会因 codegen 方便自动升级；
- 每一步必须有 deterministic artifact、negative verifier/codegen tests、真实工具链验证和前序 regression；
- 在真实 codegen/runtime 证据前，CLI 不得注册伪 `run`/`build` success。

## 3. Proposed workspace additions

```text
crates/
  sico-ir/           typed IR, source maps, verifier, canonical serialization
  sico-codegen-wasm/ verified IR to deterministic Core Wasm/Component artifacts
  sico-runtime/      minimal selected-runtime execution boundary
tests/
  ir/                lowering/verifier goldens and invalid fixtures
  components/        deterministic artifacts, WIT and runtime integration
```

名称可由 STEP-0030 工程审查微调，但语言、ABI 与 Runtime contract 不能隐藏在 crate API 中。

## 4. Execution sequence

| Step | Deliverable | Exit evidence |
|---|---|---|
| STEP-0030 | typed Sico IR contract、source maps、canonical serializer 与 verifier | accepted RFC/case gate；representative valid lowering shapes；malformed IR 被 verifier 拒绝；stable IDs/ranges；semantic invalid blocked |
| STEP-0031 | core expression/control/data lowering | numeric/local/call/record/variant/match/Result subset；deterministic snapshots；evaluation/error order 有明确 contract |
| STEP-0032 | effect/resource/revision IR lowering | capability tokens、affine resource state、revision guards 的 verifier invariants；无隐式复制/drop 或不可见 runtime 语义 |
| STEP-0033 | deterministic Core Wasm backend | verified IR only；validator 通过；重复构建 byte-identical；numeric/control programs 在真实 engine 执行 |
| STEP-0034 | Component/WIT boundary codegen | accepted WIT/canonical ABI mapping；Component validator + host call；Result/resource/value records 往返 |
| STEP-0035 | async/task/stream backend decision and implementation | 先审计 RFC-0004/runtime support；仅实现证据支持子集；unsupported path 明确诊断，禁止模拟成功 |
| STEP-0036 | minimal end-to-end CLI build/run chain | source→semantics→IR→Component→Runtime；stdout/stderr/exit contract；invalid input 不产物；命令 surface 经审查冻结 |
| STEP-0037 | determinism、fuzz/limits/performance 与 M3 exit audit | IR/verifier/codegen fuzz；artifact determinism；runtime corpus；non-SLA baseline；M0–M2 regression；下一阶段计划 |

进度：STEP-0030 已完成；typed IR v0、canonical JSON、source maps、独立 verifier 与 semantic gate 见 [`RFC-0008`](../rfc/RFC-0008-typed-sico-ir-contract-v0.md) 和 [`review report`](../reports/typed-sico-ir-contract-v0.md)。下一执行项为 STEP-0031。

## 5. Test matrix

- Unit：IR builders/verifier、type/effect/resource invariants、source maps、canonical serialization；
- Golden：semantic facts→IR、IR→Wasm/Component、diagnostic/source mapping；
- Negative：malformed IR、invalid ownership/effect/branch signature、unsupported ABI/async feature；
- Integration：real Wasm/Component validators、selected Runtime、WIT host call、CLI file/stdin/text/JSON；
- Properties：deterministic IDs/order/bytes、verifier rejects mutations、range within source、no invalid semantic lowering；
- Limits/performance：large IR/CFG/type/resource graph、diagnostic cap、compile/runtime corpus；
- Regression：STEP-0015–0029、M2/M1/M0 validators remain green。

## 6. M3 exit gate

- representative P0 valid programs 经过真实 IR/verifier/codegen/Runtime，结果与 semantic expectation 一致；
- invalid semantic source 在 IR 前拒绝，invalid IR 在 codegen 前拒绝；
- 相同输入/配置生成确定性 IR 与 Component artifact；
- Core Wasm 与 Component 通过真实 validator，最小 WIT host call 在选定 Runtime 执行；
- resource/Result/numeric/async 只声明已由真实链路证明的子集；
- CLI build/run contract、错误退出、产物策略和 capability boundary 有审计证据；
- fuzz/property/limit 与非 SLA 性能基线通过；
- `.sapp`、安全/缓存/host/REPL 等 M4+ 能力仍明确 unavailable。

## 7. Deferred beyond M3

`.sapp` manifest/package/hash/signature、增量缓存、sandbox/storage/permission enforcement 属于 M4；Desktop/Android host 属于 M5/M6；package registry、LSP 与生态发布属于 M7。M3 的最小 Runtime 链路不得提前固化这些产品协议。
