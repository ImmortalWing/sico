# STEP-0292: M22 W1 independent CI dispatch

> - status: in progress; remote result and W1 verdict pending
> - phase: M22 W1 consolidation, execution card 22-A
> - date: 2026-09-25
> - evidence class: CI infrastructure only until a fresh runner result exists

## Scope

The owner explicitly authorized pushing the pending W1 work to obtain an
independent CI verdict. The local `dev` worktree starts at `0e4cbb9`; during
pre-push inspection, `origin/dev` had advanced to `550059a` with a different
STEP-0279–0283 W1/W2 implementation and conflicting STEP numbers. This STEP
uses the isolated `codex/m22-w1-ci` branch, preserving both histories and all
pending local work. It does not merge or overwrite `origin/dev`.

The repository has no existing remote workflow. Add a branch-scoped GitHub
Actions Windows GNU job that fetches the two Cargo workspaces, prepares the
GNU build tools, then runs the existing full `tools/run-ci.ps1`. The workflow
must be evaluated against the pushed commit, not the earlier local worktree.

## Exit evidence to record

- Exact pushed commit SHA and workflow run URL.
- Fresh-runner result for all 11 `run-ci.ps1` stages, including the W1 215+5
  checker differential in the runner suite.
- Any environment/setup failure distinct from a product test failure.
- An explicit W1 GO/NO-GO decision. A push alone is not a GO.

Until the run completes and its output is inspected, W1 remains pending and
M22 remains NO-GO. The W2 implementation card 22-C does not start.
