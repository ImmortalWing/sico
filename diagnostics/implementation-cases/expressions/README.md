# Expression diagnostics — implementation corpus

RFC-0048 / RFC-0049 / RFC-0039 拒绝面在**真实编译器**上的逐码身份凭据：每个
case 由 `tools/validate-step-0489.ps1` 用 `sico check --json` 实跑，证明目录
模板、编译器 `arguments` 与默认短消息三者逐字节一致。

与 `semantic-cases/` 的区别：P0 语料是**设计态**语法，只承载 code/key 的设计
映射；本语料用**当前可执行语法**，承载目录 ↔ 编译器的实现级对拍证据。

## 必须拒绝

| Case | 诊断码 | 默认短消息 |
|---|---|---|
| [`EXP-001`](./invalid/swap-form-non-atom.sico) | `INFIX_SWAP_OPERAND_NOT_ATOM` | non-atom operand in a swap-form > condition; evaluate the operand into a binding first or rewrite as b < a |
| [`EXP-002`](./invalid/let-bind-arm-not-expression.sico) | `LET_BIND_ARM_NOT_EXPRESSION` | let-bound arm must be exactly one expression |
| [`EXP-003`](./invalid/let-bind-missing-else.sico) | `LET_BIND_MISSING_ELSE` | let-bound if requires an else arm |
| [`EXP-004`](./invalid/unresolved-call-target.sico) | `UNRESOLVED_CALL_TARGET` | unresolved call target missing_helper |
