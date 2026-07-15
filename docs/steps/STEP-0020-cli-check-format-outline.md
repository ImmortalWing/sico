# STEP-0020: 实现 `sico check`、`format` 与 `outline`

> - status: complete
> - phase: M1
> - started: 2026-07-15
> - completed: 2026-07-15
> - owners: autonomous-agent

## 1. Objective

交付第一个可执行 `sico`：为文件和 stdin 暴露 syntax check、canonical format 与 top-level outline，冻结 stdout/stderr、退出码和 RFC-0001 JSON 行为，并明确说明 M2 type checker 尚不可用。

## 2. Command contract

```text
sico check [--json] <file|->
sico format [--check|--write] <file|->
sico outline [--json] <file|->
```

- exit 0：命令成功；`check` 只表示 syntax success；
- exit 1：已登记的 source language diagnostic，或 `format --check` 发现差异；
- exit 2：usage、I/O、source contract、尚未登记的 lexical/parser/internal failure；
- text diagnostic 写 stderr；JSON diagnostic envelope 写 stdout；正常数据写 stdout；
- `format --write` 只接受文件，默认 format 输出 stdout，任何 error tree 都无格式化输出；
- outline 只列 parser 已有的顶层 kind/name/byte range，不声称名称解析或类型检查。

## 3. Boundaries

- 不实现 `run`、`build`、`-c`、REPL、type checker、IR 或 codegen；
- 29 个 M2 semantic reject 必须仍是 syntax success，并显式返回 `type_checker=unavailable`；
- 不为尚未登记的 lexer/source failure 临时分配诊断 code；
- 不改变 STEP-0018 RFC-0001 renderer 或 STEP-0019 formatter policy。

## 4. Acceptance

- file/stdin check success、mutation text/JSON failure与退出码集成测试；
- format stdout/check/write、幂等、syntax refusal 集成测试；
- outline text/JSON 顶层顺序与 byte range 集成测试；
- usage/I/O 和未实现命令不伪成功；
- workspace、STEP-0016–0019 与 M0 regression 保持绿色。

## 5. Commit

`feat(cli): [STEP-0020] expose check format and outline`

## 6. Changes and validation

- 新增真实 `sico` binary 与 clap command surface，只注册 `check`、`format`、`outline`；
- exit 0/1/2、file/stdin、text/JSON 和 stdout/stderr 行为已冻结；
- `check` JSON 复用 RFC-0001 envelope，并显式附加 syntax/type-checker 状态；
- `format` 复用 STEP-0019 API，支持 stdout/`--check`/`--write` 且拒绝 error tree；
- `outline` 仅输出 parser top-level declaration source order、kind/name/byte range；
- 4 组真实进程 integration tests、workspace Clippy/tests、STEP-0016–0019 与 M0 regression 全部通过。

```text
STEP_0020_OK commands=3 input_modes=file,stdin exits=0,1,2 check=text,json format=stdout,check,write outline=text,json integrations=4 type_checker=unavailable run_build_repl=unavailable
```

## 7. Risks and next

source contract 与未登记 lexical/parser failure 仍按 exit 2，不临时分配 code。STEP-0021 必须验证任意 bytes、深嵌套、长 token 和随机 delimiter 下 CLI 所依赖的 frontend 不 panic/hang，并做 M1 requirement-by-requirement exit audit。
