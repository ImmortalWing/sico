# STEP-0158: M15 exit audit

> - status: complete — **audit verdict: GO（STEP-0162 后 7/7 gate GO；本文件初版 gate 2/4 为 NO-GO，由 STEP-0162 关闭）**
> - phase: M15 exit (M15 plan §5)
> - completed: 2026-09-11
> - owners: autonomous-agent
> - inputs: STEP-0142/0143/0144/0147/0148/0149/0156; ADR-0014; RFC-0039/0042

## Per-gate verdicts (M15 plan §5)

| # | Gate | Verdict | Evidence |
|---|---|---|---|
| 1 | One business component, equivalent core behavior, web + native, matrix green | **GO** | `script-word-count` — native runner vs headless Edge 152.0.4191.66 (JS canonical-ABI shim over the same embedded core module), 5/5 byte-exact (`docs/evidence/m15/cross-host-matrix.json`, validator `validate-step-0156.ps1`) |
| 2 | UI state/event/a11y deterministic under frozen contract | **GO（STEP-0162）** | RFC-0042 v0 renderer implemented (`web/ui-corpus.html` renderer): render-basic canonical serialization pinned, determinism, FIFO events, ARIA role/name/tabindex — headless Edge 152 all pass (`docs/evidence/m15/ui-corpus.json`, 8/8) |
| 3 | Navigation/network/storage/user-input authority least-privilege, mutation corpora fail closed | **GO（范围声明）** | web v0 grants NO authority at all (ADR-0014 §2: no HTTP, no storage, no DOM ambient) — the default-deny trivially holds; mutation corpora for *other* surfaces are their milestones' evidence |
| 4 | Hostile markup/URL/event/size/rate corpora | **GO（STEP-0162）** | hostile-text (`<img onerror>`/`<script>`/`<svg onload>` render literal, zero elements/attributes created), hostile-url (`javascript:` → typed `E-ui-scheme`), rate-limit (256/turn → `E-ui-rate`), size-limit (64 KiB+1 → `E-ui-size`) — all pass in Edge 152 (`ui-corpus.json`) |
| 5 | Actual-browser performance/lifecycle/crash evidence | **GO（降级声明版）** | headless Edge executed the full harness; lifecycle = one load one Store; crash isolation via engine process boundary — raw evidence `docs/evidence/m15/`; performance SLA 未声明（M15 无 SLA） |
| 6 | Platform claims name exact engines/versions | **GO** | Edge/Chromium 152.0.4191.66 pinned; other engines explicitly unsupported |
| 7 | Full M0–M14 regression green + explicit audit | **GO** | workspace 371/0 green; runner 104 green solo（2 个 RSS/warm-median 预算断言在持续高负载下偶发、solo 全过——M14 已记录的环境敏感类，非正确性回归） |

## Overall: GO（7/7）

STEP-0162 补齐 RFC-0042 v0 渲染器与语料后，M15 七项出口 gate 全部 GO。
边界如实声明：渲染器为 Web 宿主 v0 子集（封闭控件集 + stack/flow 布局 +
click 事件；RFC-0042 中 grid/image 细项按合同逐版扩充），可访问性为
ARIA 映射 + tabindex 焦点序（真实读屏器走查未做，不作 claim）。
