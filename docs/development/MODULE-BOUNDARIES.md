# 模块边界

Sico 采用与 OpenJDK 相同方向的“单仓库、强模块、分命令、分发布物”模型。crate 路径保持稳定，模块归属与允许依赖由 [`module-boundaries.json`](../../tests/architecture/module-boundaries.json) 定义，并由 [`validate-module-boundaries.ps1`](../../tools/validate-module-boundaries.ps1) 强制检查。

## 模块职责

| 模块 | 主要 crate | 职责 |
|---|---|---|
| language | `sico-source`、`sico-syntax`、`sico-lexer`、`sico-parser`、`sico-hir`、`sico-semantics` | 源码、语法和静态语义 |
| compiler | `sico-ir`、`sico-codegen-wasm`、`sico-diagnostics`、`sico-format`、`sico-index` | 编译、诊断和查询 |
| observability | `sico-observability` | 无执行权的版本化 debug identity/map/fault/event 数据合同与严格验证 |
| tooling | `sico-cli`、LSP、AI tools | 语言开发工具 |
| application | `sico-package` | `.sapp` 格式、签名和授权对象 |
| runtime | `sico-runtime`、`sico-app-cli` | Runtime、安全执行和应用命令 |
| host | Host Core、Desktop Host、Mobile Host Core | 安装、权限、生命周期和平台适配 |
| ecosystem | `sico-ecosystem`、`sico-registry-server` | 发布者、registry、更新、依赖协议和只读网络 origin |
| integration | third-party pilot | 端到端集成证明 |

## 命令边界

```text
source.sico
    │ sico build
    ▼
app.component.wasm
    │ sico-app pack
    ▼
app.sapp
    │ sico-app run 或 sico-desktop-host open
    ▼
Wasmtime / platform Host
```

`sico` 不得依赖 package、Runtime 或 Host。`sico-app` 不得通过正常依赖引入编译器；`sico-app dev` 只能通过显式进程边界调用独立的 `sico build`，之后必须回到相同的 package/trust/Runtime gate。测试可以使用编译器生成确定性 Component fixture，但这类依赖必须保持为 dev-dependency。

`sico-observability` 只拥有严格、版本化、bounded 的数据结构、canonical serialization 与 identity 验证。Compiler、Runtime 和 tooling 可以依赖它，但它不得反向依赖这些模块，也不得获得进程启动、Store、Host provider、文件或网络 authority。

## 修改规则

1. 新 crate 必须先加入机器可读模块清单；
2. 新正常依赖必须符合模块允许边；
3. ABI、`.sapp` 或 Host WIT 变更必须提供跨模块 conformance 测试；
4. 平台专用代码不得进入 language/compiler；
5. 发布包可以组合多个命令，但源码所有权不因此合并。
