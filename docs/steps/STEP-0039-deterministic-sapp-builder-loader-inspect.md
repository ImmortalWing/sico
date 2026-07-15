# STEP-0039: deterministic `.sapp` builder、loader 与 inspect core

> - status: complete
> - phase: M4
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

实现 RFC-0015 的 deterministic builder 与 strict verification/inspection core，使 malformed、non-canonical、invalid Component、hash/length/resource-set tamper 在执行前被拒绝。

## 2. Context and evidence

M3 `sico-codegen-wasm` 已生成 wasmparser-valid raw Component。STEP-0038 冻结 fixed framing、canonical JSON 和 18 项 threat matrix；本步只建立 package library，CLI surface 在 STEP-0044 冻结。

## 3. Scope

新增 `sico-package` workspace crate：manifest/identity/artifact/limit model、path normalization、checked framing parser、canonical builder、SHA-256、Component validation/import inspection 和 verified extraction。不在本步接受未验证签名或授予 capability。

## 4. Options and decision

选择单独 crate，避免 CLI 或 Runtime 绕过同一 parser。loader 返回拥有内容的 `VerifiedPackage`，只有该类型可进入后续 trust/capability/runtime gate；不向调用者暴露“部分成功”包。

## 5. Changes

- package ceiling：64 MiB/1,024 entries/240-byte path/8 levels/64 KiB manifest/32 MiB Component/8 MiB resource；
- strict NFC relative paths，拒绝 parent、absolute、backslash、colon、NUL、device name 与 ASCII case collision；
- canonical manifest `deny_unknown_fields`，resource/effect/capability 排序；
- wasmparser 完整验证 Component，并独立读取 top-level imports；
- content length 与 SHA-256 全量核对，truncation/trailing/tamper 不产生 verified object。

## 6. Validation

```text
cargo clippy -p sico-package --all-targets --all-features -- -D warnings
cargo test -p sico-package
STEP_0039_OK package_crate=sico-package deterministic=byte-identical parser=strict component=wasmparser hashes=sha256 package_tests=4 threat_classes=framing,path,limit,manifest,integrity next=STEP-0040
```

4 个集成测试覆盖 byte-identical build、verified inspection、tamper/truncate/trailing/version、unknown manifest field、父目录/绝对/反斜线/设备名/ADS/大小写冲突。

## 7. Metrics

最小测试 package 含 1 Component + 1 resource；性能留到 STEP-0045，当前不建立 SLA。

## 8. Risks and follow-ups

签名 entry 当前只由 framing 保留为 opaque bytes，不能形成 trust；STEP-0040 必须 strict parse/verify 后才允许执行。capability closure 在 STEP-0041 完成。

## 9. Audit links

- [`RFC-0015`](../rfc/RFC-0015-sapp-package-format-v0.md)
- [`STEP-0038`](./STEP-0038-sapp-threat-model-package-contract.md)
