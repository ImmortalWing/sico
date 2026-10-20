# STEP-0310: M22 CI 裁决与出口复审

> - status: documentation/evidence audit complete; M22 NO-GO
> - phase: M22 S7 evidence audit (不关闭里程碑)
> - date: 2026-09-28
> - evidence class: internal-fixture; GitHub Windows GNU exact-SHA CI for committed snapshots

## 入口与裁决

在不改动 STEP-0306–0309 实现的条件下，核对 GitHub Actions 公开运行及其 SHA、job 结论：STEP-0300 的 `f2fd7fc` 对应运行 36212509946 成功；包含 STEP-0301–0305 的 `8bee75b` 对应运行 36366432845 成功。两者仅裁决相应提交；0306–0309 尚未提交，独立 CI 待执行。

逐项复审 M22 计划 §4 七个出口门，结论及剩余工程工作见 [M22 出口复审](../reports/m22-exit-audit-2026-09-28.md)：**NO-GO**。本次审计进入 S7 并给出裁决；它不把未满足的 S3/S4/S5/S6 变成完成。R3 连续停滞数仍为 0，Route B 尚未触发。

## 验证

- GitHub Actions runs 36212509946、36366432845：`completed/success`，Windows GNU job `success`，SHA 与本地提交一致。
- 本机 `tools/run-ci.ps1`（完整、无 `-Fast`）：**CI GREEN 11/11，exit 0**，包括 root/runner fmt、两工作区 clippy、runner build、root/runner 全测试、module boundaries、planning contract、application-profile matrix、cross-host/UI corpus 和 `git diff --check`。原始控制台记录在本机 `target/evidence/m22-ci-adjudication-local.log`（被 `.gitignore` 忽略，不作为远端 CI 证据）。
- `tools/validate-step-0124.ps1 -SelfTest`：exit 0，5/5 negative cases；`git diff --check`：exit 0。
- `tools/validate-step-0309.ps1`（单独新命令）：exit 0，43/43 compiler、2/2 parser、1/1 local-bounds 与继承 STEP-0262/0302–0308 门全部绿。
- `tools/report-m22-canary.ps1`（单独新命令）：exit 0，29/30，`main` / `ERR:E-SH-IR-SCICOND`，full-source exit 122，fuel cap 5,000,000,000；该 cap 不是实际 consumed fuel。

本次审计时 STEP-0306–0309 尚无对应精确快照的独立 CI，因此无权将上述旧 SHA 的结果写成它们的 GO。后续提交触发的运行须另行核对 SHA 与结论。

## 裁决增补（2026-09-29）

本 STEP 与 STEP-0306–0309 的快照 `9c94e7d` 已取得独立 CI：GitHub Actions 运行 [36406396074](https://github.com/ImmortalWing/sico/actions/runs/36406396074)（workflow `m22-w1-ci.yml`，`windows-2025` runner）`completed/success`，`windows-gnu` job `success`，head SHA 与本地提交一致。按本文裁决规则，该运行覆盖 STEP-0306–0309 的实现与本文档快照：0306–0309 自此获得正式进展信用，canary 29/30（`main` / `ERR:E-SH-IR-SCICOND`）、全源 exit 122 与 R3 连续停滞数 0 正式化。七个出口门的 NO-GO 裁决不变（门 2/3/4 未满足，门 6 的完整候选集成审计仍待复跑）。同日 owner 指令：`codex/m22-w1-ci` 为 M22 权威工作线，M22 完成后合并至 `dev`；该线 STEP 的提交/推送已获授权。
