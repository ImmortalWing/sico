# Execution plans

阶段计划把 ROADMAP 的里程碑拆成有依赖、退出证据和稳定 STEP 编号的可执行序列。计划可以因新证据调整，但调整必须更新状态、理由和后续编号，不能让实现隐式改变 RFC/ADR。

| Plan | Status | Scope |
|---|---|---|
| [`M1 compiler frontend`](./M1-compiler-frontend.md) | complete | Rust source/span、lossless lexer/parser、恢复、formatter、`sico check`、outline 与 M1 audit |
| [`M2 static semantics`](./M2-static-semantics.md) | ready after STEP-0021 go | full HIR、names/types/control/effects/resources、semantic diagnostics/index 与 M2 audit |
