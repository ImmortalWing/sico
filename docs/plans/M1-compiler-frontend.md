# M1 plan: compiler frontend and diagnostics

> - status: in progress (STEP-0019 complete)
> - created: 2026-07-15
> - phase: M1
> - language baseline: [`RFC-0005`](../rfc/RFC-0005-labeled-block-syntax-baseline.md)
> - diagnostic contract: [`RFC-0001`](../rfc/RFC-0001-diagnostics-protocol-v0.md)

## 1. Outcome

交付第一个正式 Sico Rust 前端：读取 UTF-8 `.sico`，对 B Labeled Blocks 建立无损语法树和语义 AST，局部恢复语法错误，提供唯一 formatter、`sico check` 文本/JSON 诊断和基础 outline。M1 不做类型检查、Sico IR 或 Component codegen。

## 2. Entry conditions

- STEP-0014 结论允许进入 M1；
- B 的 54 个 canonical case、12 个 B mutation 和 RFC-0005 不漂移；
- Unicode/identifier、token 和换行规则已在 lexer 实现前由 [`RFC-0006`](../rfc/RFC-0006-lexical-source-contract-v0.md) 接受；
- RFC-0001 的 E1xxx syntax partition 从 `reserved` 升级前有真实 parser 根因、span 和 recovery 证据。

## 3. Engineering boundaries

- Rust stable，锁定依赖；不引入 JavaScript/TypeScript；
- source/span 坐标以 UTF-8 byte 为事实源，同时提供 line/column 映射；
- lexer/parser 不做类型、效果、能力或所有权判断；
- lossless syntax tree 保留 token、空白、注释和 error/missing node；
- semantic AST 不保留纯格式差异，任何 error node 禁止进入 M2；
- formatter 是 canonical source 的唯一生产者，二次格式化逐字不变；
- 每一步先写 STEP 文档、case/golden，再实现；提交和推送前运行相关全套回归。

## 4. Proposed workspace shape

```text
Cargo.toml
crates/
  sico-source/       UTF-8 source、TextSize/TextRange、line index
  sico-syntax/       SyntaxKind、lossless tree、AST wrappers
  sico-lexer/        tokenization、trivia、lexical diagnostics
  sico-parser/       B grammar、event/tree construction、recovery
  sico-format/       canonical formatter 与 idempotence
  sico-diagnostics/  RFC-0001 text/JSON、排序、span/related info
  sico-cli/          sico check/format/outline
tests/
  parser/            54 canonical cases、12 mutation goldens
  formatter/         roundtrip/idempotence
  diagnostics/       text/JSON snapshots
fuzz/                lexer/parser no-panic/no-hang targets
```

具体 tree 库、自写/生成 parser 和 CLI 库在 STEP-0015 做有界比较；选择只影响实现，不得改变 RFC-0005 grammar。

## 5. Execution sequence

| Step | Deliverable | Exit evidence |
|---|---|---|
| STEP-0015 | M1 workspace + lexical/source RFC | workspace 可构建；UTF-8、identifier、newline、trivia、token、span 与限额有正反例和 accepted/proposed 状态 |
| STEP-0016 | source/span + lossless lexer | token golden、Unicode/invalid-byte diagnostics、line index property tests、formatter-safe trivia preservation |
| STEP-0017 | B grammar + lossless parser happy path | 54/54 B cases parse；AST shape snapshots；不执行 M2 语义拒绝 |
| STEP-0018 | recovery + E1xxx syntax diagnostics | 12/12 B mutation 命中主要根因与 anchor；construct 外最多 1 cascade；text/JSON span verified |
| STEP-0019 | canonical formatter | 54/54 format→parse AST stable；第二次 format byte-identical；comments/trivia policy verified |
| STEP-0020 | `sico check`, `format`, `outline` | CLI exit codes、stdout/stderr、RFC-0001 JSON、基础 top-level outline integration tests |
| STEP-0021 | fuzz/performance + M1 exit audit | no panic/hang corpus；size/depth/token limits；全部 M1 gate requirement-by-requirement proven |

编号在 STEP-0014 完成后可立即使用。若某步发现必须先做语言决定，则暂停实现、新建 RFC，并在本表插入新 STEP；不得把语法选择藏进 Rust enum 或 parser 分支。

进度：STEP-0015–0019 已完成；54/54 AST-stable/idempotent formatter 证据见 [`STEP-0019`](../steps/STEP-0019-canonical-formatter.md) 和 [`review report`](../reports/canonical-formatter-v0.md)。下一执行项为 STEP-0020。

### 5.1 后续命令行体验目标（M3/M4）

Sico 最终应具有接近 Python 的终端使用体验，但底层仍采用“编译到 WebAssembly Component，再由 Sico Runtime 执行”，不改为动态解释器，也不引入 JavaScript：

```text
sico app.sico              # sico run app.sico 的便捷形式
sico run app.sico          # 编译、复用本地缓存并运行源码
sico -c '<source>'         # 执行短源码片段
sico                       # 进入交互式 REPL
sico build app.sico        # 显式产生可分发构件
```

- M1 的 STEP-0020 只实现 `check`、`format`、`outline`，不得伪装已经具备执行、构建或 REPL 能力；
- M3 在 Component codegen 与最小 Runtime 链路真实通过后，定义最小端到端 `run`/源码直跑行为；
- M4 随 `.sapp`、Runtime、缓存和权限模型固定 `build/run/inspect` 的稳定契约；
- `-c`、无参数 REPL、参数透传、缓存失效、退出码和 stdin/stdout/stderr 规则须由后续独立 RFC/STEP 验证后接受；
- 源码直跑必须先执行与正常构建相同的语法、类型、能力和资源检查，不能为了脚本体验绕过静态保证。

## 6. Test matrix

- Unit：source range、line index、token、parser productions、formatter primitives；
- Golden：54 canonical B trees、12 mutation diagnostics、text/JSON envelopes；
- Properties：no token loss、parse(format(parse(x))) AST stable、format idempotent、range within source；
- Fuzz：任意 bytes/UTF-8、深嵌套、长 token、缺失 close、随机 delimiter；
- Integration：`sico check/format/outline` 文件、stdin、机器 JSON 和退出码；
- Regression：M0 semantic/mutation/AI/diagnostic/query validators 与 Rust prototypes 保持绿色。

## 7. M1 exit gate

- B canonical corpus 54/54 稳定解析；
- B mutation 12/12 给出预期主要诊断和有界恢复；
- formatter 唯一且幂等；
- syntax tree 无损，semantic AST 不含可 lowering error node；
- `sico check` text/JSON 诊断符合 RFC-0001；
- outline 能稳定列出顶层声明；
- fuzz/property/limit tests 无 panic、无限循环或不受限资源增长；
- 未实现的 M2 规则明确返回“尚无 type checker”工程状态，不把语义负例误报为语法错误。

## 8. Parallel but non-blocking evidence

- 获得模型、凭据和成本授权后执行 `sico-ai-eval-v1` full runs；
- 非 Windows CI 重现 numeric/resource/Component probes；
- Future/Stream 真实 Component Runtime 往返；
- Android Pulley minimum probe。

这些工作可以触发 RFC/ADR 复审，但不属于 M1 parser happy-path 的隐式功能。
