# Technical prototypes

本目录保存 M0 技术风险原型。原型用于验证表示、边界和工具链，不是正式 Sico 编译器或稳定标准库。

| Prototype | Step | Purpose |
|---|---|---|
| [`numeric`](./numeric/README.md) | STEP-0008 | `Int`、Decimal、限额与 WIT 边界 |
| [`resource-async`](./resource-async/README.md) | STEP-0009 | affine resource、Task/Future/Stream 与 WASI 0.3 WIT |
| [`component-host-call`](./component-host-call/README.md) | STEP-0010 | 真实 Component host call、resource、数值记录与原生 async ABI |
| [`script-profile`](./script-profile/README.md) | STEP-0076 | Script rich-value direct-runner fallback and bounded channel cases |
