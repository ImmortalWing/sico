# 语言基础与可运行子集

Sico `main`（`0.0.2-dev`）使用 B Labeled Blocks 语法。语言前端已经能检查比 Runtime codegen 更丰富的类型和控制流，因此必须区分“检查通过”和“可以构建运行”。

## 源文件规则

- 扩展名：`.sico`；
- 编码：严格 UTF-8，无 BOM；
- 换行：LF 或 CRLF；裸 CR 被拒绝；
- 标识符：Unicode XID，并要求 NFC 规范化；
- 块通过冒号开始，通过对应的 `end ...` 明确结束；
- 当前规范格式由 `sico format` 唯一确定。

## 最小程序

```sico
function main() returns Int:
  return 42
end function
```

入口必须名为 `main`。当前真正经过 Component/Wasmtime 端到端验证的入口返回类型是：

- `Int`；
- `Bool`；
- `Unit`。

## 函数形式

```sico
function identity(value: Int) returns Int:
  return value
end function
```

参数使用 `name: Type`，返回类型写在 `returns` 后。语义检查支持函数声明和调用，但当前 scalar codegen 仍会对部分函数调用或复杂 operation 返回明确的 `Unsupported`；不要仅凭 `sico check` 成功就假定可运行。

## 变量与控制流

前端和静态语义已覆盖：

- `let` 局部绑定；
- `if`；
- `match`；
- `record`、`enum` 和名义类型；
- `Result`、`Option` 和 `try`；
- capability/effect；
- resource、Task、Future 和 Stream；
- revision dataflow。

这些能力并非全部具有 Runtime codegen。异步 Task/Future/Stream 源码目前会得到专用 backend refusal，而不是生成不可验证产物。

## 当前可靠的可运行示例

```sico
function main() returns Int:
  return 40 + 2
end function
```

```sico
function main() returns Bool:
  return true
end function
```

```sico
function main() returns Unit:
  return Unit
end function
```

建议先用 `sico check`，再用 `sico build` 验证 backend 支持。构建失败时不会创建或覆盖目标产物。

## 示例目录的含义

根目录 `examples/` 保存语言设计历史和编译器回归样本，其中一些文件是旧候选语法，不应直接复制为当前应用。当前 B 语法的正反例位于 `syntax-candidates/b/`，可执行端到端最小样本位于 `tests/end-to-end/`，完整发布工作流位于 `pilots/third-party-component/`。

语义和语法的设计级说明见根目录 `SEMANTICS.md` 与 `SYNTAX.md`；它们不是稳定 1.0 规范。
