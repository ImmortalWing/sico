# M13 plan: AI tooling closure

> - status: in-progress (STEP-0119 complete 2026-08-04)
> - created: 2026-08-04
> - phase: M13 (parallel support track; M11/M12 mainline unchanged)
> - reserved steps: STEP-0119–0123
> - entry requirement: none beyond current repository state; runs in parallel with M11 implementation the way STEP-0074 ran during M7
> - decision record: repository owner approved this track and STEP-0119 start on 2026-08-04; live-model credentials (DeepSeek API) promised by owner, pending delivery

## 1. Outcome

Close the gap between "AI can inspect/understand/repair Sico code" and the AI-workflow completion criteria in `AGENT_GOAL.md` §13, using only repository-local evidence plus one externally gated authoritative live-model evaluation.

The measured baseline today (see `target/ai-eval-experiments/report-hy3.md`) is:

- overall score 0.8846 over 96 fixed tasks × 30 attempts (2,880 tries, subagent-as-model, `official_comparison=true` only under the subagent substitution approved at the time);
- repair 1.000, understanding 0.982–1.000;
- generation byte-exact match A0=0.0 / B=0.2 / C=0.3, dominated by `canonical-source-mismatch` (~750 failures).

M13 exists because the remaining AI gaps are specific and closable: generation quality has one dominant, measured failure class; no ADR fixes the numeric quality budgets required by `AGENT_GOAL.md` §11/§13; no agent-framework integration layer (MCP / function-calling tool schemas) exists; semantic-index accuracy/latency and the error taxonomy's `observed_ai_frequency` remain unmeasured.

## 2. Relationship to M11/M12 and external gates

M11 (STEP-0104–0110) and M12 (STEP-0111–0118) proceed unchanged. M13 steps use numbers starting at STEP-0119, the first free block after the M12 reservation. M13 work must not modify M10-frozen claims (bounded events, redaction, exact DAP) and must not alter semantic/IR contracts under M11 implementation; where a guide or scorer change touches those surfaces, the change is additive and separately validated.

External gates are rechecked at every M13 step:

- **Live-model credentials and cost authorization** (DeepSeek API promised by the repository owner): when delivered, an authoritative live-model evaluation step is numbered at that time and runs under the existing `sico-ai-eval-v1` protocol. Until then, subagent runs remain engineering feedback only and produce no official model scores, per `docs/STATUS.md` §6.
- Public deployment, third-party pilots and mobile runners remain deferred and are not M13 scope.

## 3. Scope and non-goals

### 3.1 In scope

- failure attribution for the measured generation gap and one scored, re-run baseline;
- scorer and/or guide changes to make generation scoring honest about canonical formatting;
- a numeric quality-budget ADR derived from the re-measured baseline;
- an agent-framework integration layer exposing the existing `sico.ai-tool.v0` operations;
- measurement of semantic-index query accuracy/latency and real observed error frequencies;
- a closing audit against the four AI-workflow completion criteria.

### 3.2 Non-goals

- no change to the 96-task protocol's task set or scoring identities (normalization is a scorer option, not a task rewrite);
- no model training, fine-tuning or provider-specific prompt tuning beyond what the guides already document;
- no multi-location or >256-byte AI edits; `validate_fix` bounds stay frozen;
- no MCP server with filesystem/process/network side effects beyond the existing tool boundary;
- no claim of live-model scores before authorized credentials produce them;
- no interruption or re-scoping of M11/M12.

## 4. Execution sequence

### STEP-0119: generation-quality attribution, scorer decision and re-measured baseline

Deliver:

- an attribution report over `target/ai-eval-experiments/score-hy3-full.json` quantifying `canonical-source-mismatch` by candidate (A0/B/C), by task, and by nature (formatting-only vs substantive token difference), including a formatter-normalization experiment on representative failures;
- a decision, recorded in the step doc: add canonical normalization to the scorer (e.g. format both sides before byte comparison where a formatter exists for the candidate), and/or state the canonical-output requirement explicitly in the three `ai-eval/guides/` documents and prompt packets;
- implementation of the chosen change with validator updates;
- a re-run of the 96×30 subagent evaluation producing a new measured baseline under the updated scorer, stored under `target/ai-eval-experiments/` with the same run-format discipline as the hy3 run.

Exit evidence:

- every reclassified failure is reproducible from stored run artifacts;
- scorer changes are fail-closed (a task that was byte-exact before remains scored correct);
- the new baseline is reported with per-candidate generation/repair/understanding scores comparable to the hy3 report.

### STEP-0120: AI quality-budget ADR (ADR-0011)

Deliver:

- an ADR fixing numeric budgets for the AI workflow metrics, as `AGENT_GOAL.md` §11 requires once a real baseline exists: per-category minimum scores (generation/understanding/repair), evaluation scope and repetition count, and which scorer mode is authoritative.

Exit evidence:

- budgets reference the STEP-0119 measured baseline and are stated as pass/fail thresholds usable by STEP-0123;
- any budget the subagent baseline cannot justify is deferred to the live-model run rather than asserted.

### STEP-0121: agent-framework integration layer

Deliver:

- a new crate (provisional name `sico-mcp-server`, package count and module boundaries updated per ADR-0007 discipline) that exposes the four frozen `sico.ai-tool.v0` operations (inspect / validate_fix / plan_execution / summarize_execution) as MCP tools with JSON Schema registrations;
- fail-closed behavior identical to the existing tool: no filesystem writes, no process launches, no network, no model calls from the tool itself;
- contract cases added to `ai-eval/tooling-cases.json` (AIT-025+) and validator coverage.

Exit evidence:

- all existing 512 invalid-input mutations still fail closed through the new transport;
- a real MCP client session performs inspect→edit→validate_fix roundtrips against repository fixtures;
- module-boundary validator covers the new package.

### STEP-0122: measurement completion

Deliver:

- semantic-index query accuracy and latency benchmark over the existing 10-module fixture corpus plus compiler-produced indexes, recorded under `semantic-index/` or `target/evidence/`;
- `observed_ai_frequency` in `ai-eval/error-taxonomy.json` replaced with measured frequencies from the STEP-0119 run artifacts (and later refreshed by the live-model run).

Exit evidence:

- numbers are reproducible from committed fixtures and recorded commands;
- taxonomy change is data-only and validated by the existing ai-eval validators.

### STEP-0123: AI tooling closure audit

Deliver:

- an audit report scoring the four `AGENT_GOAL.md` §13 AI-workflow criteria: (a) query protocol stability, (b) structured stable diagnostics, (c) reproducible benchmarks, (d) numeric quality budgets met;
- GO/NO-GO per criterion and a residual register.

Exit evidence:

- (d) can only be fully GO with budgets from ADR-0011 met by a measured run; if live-model credentials are still unavailable, (d) is reported as `blocked-external-evidence` rather than inferred from subagent runs.

## 5. Live-model evaluation insertion rule

When the owner delivers DeepSeek API credentials and cost authorization:

- a new step (numbered at insertion time) runs the authoritative evaluation under `sico-ai-eval-v1` with recorded provider, model, cost and raw responses;
- its results supersede the subagent baseline for ADR-0011 budget checks;
- no M13 step before that point may report live-model scores.

## 6. M13 exit gate

M13 is GO when:

1. generation scoring is honest about canonical formatting and a re-measured baseline exists;
2. ADR-0011 fixes numeric budgets from measured data;
3. an agent-framework integration layer exposes the AI tool operations fail-closed;
4. semantic-index accuracy/latency and observed error frequencies are measured;
5. the STEP-0123 audit records GO/blocked status per §13 criterion with external gates rechecked;

with the standing rule that subagent evidence never substitutes for live-model credentials, and M11/M12 claims remain untouched.
