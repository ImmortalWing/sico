# M4 exit audit

> - status: complete
> - date: 2026-07-16
> - phase: M4
> - conclusion: GO

## Conclusion

GO: M4 complete; M5 entry gate satisfied; next STEP-0046.

M4 已把 M3 raw Component 升级为 canonical、strict、可签名、可授权、受存储与 Runtime 限额约束的 `.sapp`，并提供正式 build/run/inspect 与 fail-closed source cache。Desktop/Android Host、production publisher/registry/update 未被错误计入完成范围。

## Requirement audit

| Gate | Evidence | Result |
|---|---|---|
| deterministic canonical package | same input/config/resource order permutations byte-identical；RFC-0015 | pass |
| strict hostile parser | 18-class threat matrix、hard ceilings、path/version/unknown/hash rejection | pass |
| development signature/trust | domain-separated Ed25519、wrong key/strip/replay/single-byte mutation refusal | pass |
| capability closure | source = manifest = imports；host required subset；unknown default deny | pass |
| isolated storage/WASI | app+trust hashed root、real Windows junction refusal、quota/cross-app/default-off flags | pass |
| Runtime limits/faults | 11 effective dimensions、7 classes、real infinite loop termination、healthy post-fault run | pass |
| package CLI/cache | `.sapp` build/run/inspect、explicit trust、cache reverify/corrupt refusal、args/stdio/exits | pass |
| security properties | 2,048 signed mutations + 1,024 Component mutations + 256 resource permutations + explicit empty trust | pass |
| performance | 3 × 1,000 build/verify/signature-verify；non-SLA median recorded | pass |
| regression | workspace strict Clippy/tests and M0–M3 exit validators | pass |

## Performance baseline

Windows x86_64 GNU release、1,000 iterations/run、3 runs：median package build 5.459 ms，structural verify 8.571 ms，development signature verify 48.177 ms。该数据是固定微型 corpus 的 non-SLA regression baseline，不代表 Desktop startup 或大包吞吐承诺。

## Residual risks and deferrals

- development signature 不等于 production publisher identity；key agent、revocation、registry/update 留 M7；
- process adapter fault classification 依赖固定 Wasmtime 46.0.1 stderr，升级需重跑 corpus；
- persistent storage quota 是 execution 前后 audit，长期 writable in-process adapter 尚未实现；
- scalar `main()` 不支持 guest stdin/args、domain result transport 或 async cancellation；
- Windows junction 已真实验证，非 Windows symlink code path 只有编译/单元 contract，需对应 runner；
- Desktop lifecycle、permission UI、file association 与 renderer isolation 从 STEP-0046 开始。

## Entry authorization

M5 可开始，但第一步必须是 Desktop Host threat/lifecycle/platform contract；不得直接以 GUI shell 绕过 M4 loader/trust/capability/limit gates。
