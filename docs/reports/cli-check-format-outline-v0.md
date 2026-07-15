# Report: CLI check, format and outline v0

> - status: complete
> - date: 2026-07-15
> - related-step: STEP-0020
> - environment: Windows, Rust 1.97.0, clap 4.6.1

## 1. Question

已验证的 M1 frontend 能否通过一个真实 `sico` binary 提供稳定 file/stdin、text/JSON、stdout/stderr 和退出码行为，同时不把 29 个 M2 语义负例或未来 `run`/REPL 伪装成已实现能力？

## 2. Method

clap 只注册 `check`、`format`、`outline`。进程级 integration tests 启动真实 binary，覆盖成功 B 文件、M2 semantic reject、B mutation、stdin、临时文件原地格式化、outline 顺序/range、usage、I/O、source contract 和未登记 lexical error。命令契约单独冻结为文本资产。

## 3. Results

```text
STEP_0020_OK commands=3 input_modes=file,stdin exits=0,1,2 check=text,json format=stdout,check,write outline=text,json integrations=4 type_checker=unavailable run_build_repl=unavailable
```

- `check`：syntax success exit 0；text 和 RFC-0001 JSON 均明确 `type checker: unavailable` / `semantic_checks_performed=false`；
- semantic reject：代表性 M2 negative case 仍 exit 0 syntax success，不执行语义判断；
- mutation：text stderr 或 JSON stdout，E1001 与 exit 1；
- `format`：stdout、`--check`、`--write` 与 error-tree no-partial-output 全部通过；
- `outline`：text/JSON 保持顶层 declaration source order、kind/name 和 byte range；
- exit 2：usage、I/O、source contract、未登记 lexical/parser failure；
- `run`、`build`、`-c`、REPL、type checker、IR、codegen 均不可用且不伪成功。

## 4. Interpretation and limits

`check` 的成功只证明 source/lex/parse。RFC-0001 稳定 language diagnostics 当前只包含 E1001–E1012；未登记的 lexical/source failure 暂按 tool error 退出，等待有根因与协议证据的后续步骤，CLI 不临时制造 code。outline 不是 semantic index。

## 5. Links

- [`STEP-0020`](../steps/STEP-0020-cli-check-format-outline.md)
- [`CLI contract`](../../tests/cli/contract.txt)
- [`CLI implementation`](../../crates/sico-cli/src/lib.rs)
- [`integration tests`](../../crates/sico-cli/tests/cli.rs)
