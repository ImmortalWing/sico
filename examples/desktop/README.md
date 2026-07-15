# Hello Desktop

M5 的代表性应用由 `hello-desktop.sico` 与 `hello-ui.json` 组成。前者通过正式 compiler/package/签名链生成 `.sapp` 并在 Windows Desktop Host 中返回 `42`；后者通过 RFC-0020 typed UI validator 和原生 Windows preview adapter。

M5 尚未把 UI WIT 绑定加入 compiler，因此 UI model 作为同一应用的受审计 companion fixture，而不是伪装成已由 guest Component 动态发出的模型。STEP-0053 validator 同时验证两个入口。

