# STEP-0292: M22 W1 independent CI dispatch

> - status: complete; W1 GO on the isolated `codex/m22-w1-ci` snapshot
> - phase: M22 W1 consolidation, execution card 22-A
> - date: 2026-09-25
> - evidence class: independent fresh-runner CI, Windows 2025 GNU

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

## Exit evidence

- Pushed commit: `e8495f921eae16927edd71bc87c6c226d7bd88a5` on GitHub branch
  `codex/m22-w1-ci`.
- [Independent run 36087222284](https://github.com/ImmortalWing/sico/actions/runs/36087222284):
  Windows 2025 `windows-gnu` job **success**, 2026-09-25 02:40:21–02:57:51 UTC.
  Checkout, GNU setup, and the repository CI command all completed successfully.
- The command is the full `tools/run-ci.ps1`, whose exit code is nonzero if any
  of its 11 stages fails. Its runner test stage includes the frozen 215-case
  checker partition and five SHA-frozen W1 additions. The prior local
  STEP-0283 run independently recorded 11/11 stage-level output. The public
  Actions API exposes the remote job verdict but not its live stage log to an
  unauthenticated reader; the workflow step's successful exit is the remote
  aggregate evidence.

**W1 GO for this pushed snapshot.** This resolves execution card 22-A and
permits 22-C on this branch. It does not resolve the conflicting
`origin/dev` history and does not make M22 GO. Each subsequent implementation
STEP needs its own real-runner evidence and R3 accounting.
