# STEP-0050: Minimal UI WIT/SDK and renderer boundary

> - status: complete
> - phase: M5
> - started: 2026-07-16
> - completed: 2026-07-16
> - owners: autonomous-agent

## 1. Objective

实现 RFC-0020 typed no-script UI tree、renderer-neutral plan、accessibility order 与 bounded event gate。

## 2. Context and evidence

Desktop Host 必须显示真实应用但不能直接执行 guest HTML/JS/shell。RFC-0020 已冻结 7 node variants 与 node/depth/text/event ceilings。

## 3. Scope

包含 strict serde model、validation、escaping、preorder/accessibility plan、typed events、queue/rate gate、WIT package。不包含 Windows native renderer 或 guest compiler binding。

## 4. Decision

Host 完整验证模型后一次性产出 `RenderPlan`；renderer 不能看到未验证树。EventGate 从 plan 的 interactive node map 创建，button/input 事件严格匹配。

## 5. Changes

- `UiModel/UiNode/UiNodeKind` 7 variants；
- 1,024 node、depth 32、64 KiB total/4 KiB field limits；
- ASCII stable IDs、control refusal、escaped text；
- accessibility preorder；
- 256 queued events、120 accepted events/rolling second；
- `sico:ui@0.1.0` WIT world；
- malicious model/event and WIT parse tests。

## 6. Validation

```text
cargo clippy -p sico-host-core --all-targets -- -D warnings
cargo test -p sico-host-core
STEP_0050_OK ui=typed-no-script nodes=7 node_limit=1024 depth=32 text_total=65536 text_field=4096 events_queue=256 events_rate=120-per-second accessibility=preorder wit=0.253-parse ui_tests=4 host_tests=13 next=STEP-0051
```

## 7. Metrics

4 UI tests；7 node variants；6 structural/text/event ceilings；2 event variants；1 parse-verified WIT world。

## 8. Risks and follow-ups

No native renderer has been claimed. Event timestamps come from trusted Host monotonic time; caller-supplied guest time is forbidden. Compiler-facing Sico UI SDK remains a WIT/manual model boundary in M5 v0。

## 9. Audit links

- [`RFC-0020`](../rfc/RFC-0020-desktop-ui-permission-contract-v0.md)
- [`review report`](../reports/minimal-ui-boundary-v0.md)
- [`STEP-0049`](./STEP-0049-lifecycle-process-supervision.md)
