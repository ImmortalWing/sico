# Records diagnostics — implementation corpus

RFC-0047 records 拒绝面在**真实编译器**上的逐码身份凭据：每个 case 由
`tools/validate-step-0489.ps1` 用 `sico check --json` 实跑，证明目录模板、
编译器 `arguments` 与默认短消息三者逐字节一致。

与 `semantic-cases/` 的区别：P0 语料是**设计态**语法（`fn main() -> T`），
只承载 code/key 的设计映射；本语料用**当前可执行语法**，承载目录 ↔ 编译器的
实现级对拍证据。

## 必须拒绝

| Case | 诊断码 | 默认短消息 |
|---|---|---|
| [`REC-001`](./invalid/duplicate-field.sico) | `DUPLICATE_FIELD` | duplicate field: x |
| [`REC-002`](./invalid/empty-record.sico) | `EMPTY_RECORD` | record Empty declares no fields |
| [`REC-003`](./invalid/field-immutable.sico) | `FIELD_IMMUTABLE` | field p.x is immutable; replace the whole record with set |
| [`REC-004`](./invalid/record-comparison.sico) | `RECORD_COMPARISON` | records of type Point do not support ==; compare fields explicitly |
