# Sico examples

这些示例用于反推语言语义、语法和 AI 可理解性。目前全部属于设计样本，不代表已经稳定的语言规范。

## `hello.sico`

第一个候选程序表达以下语义：

- 显式导入控制台能力；
- `main` 是程序入口；
- 入口没有参数，返回 `Unit`；
- 程序向标准输出写入一行文本；
- 函数块使用显式 `end` 结束；
- 不使用分号和花括号。

候选源码：

```sico
use sico.console

fn main() -> Unit
  console.write_line("Hello, world!")
end
```

预期输出：

```text
Hello, world!
```

编译器应能从该程序生成类似的低 token 语义概要：

```text
entry: main() -> Unit
effects: console.write
capabilities: console
output: "Hello, world!"
```

在语法定稿前，需要将该程序改写为其他候选语法，并比较 AI 的生成正确率、阅读 token、错误恢复和单轮修复表现。

## `calculator/`

第一个非平凡代表性程序，覆盖类型、枚举、完整匹配、`Result`、错误传播、列表模式以及控制台能力：

- [行为规格](./calculator/SPEC.md)
- [候选语法 A](./calculator/candidate-a.sico)

## `order-state/`

订单状态机样本，覆盖封闭状态、事件、显式转换、非法路径、状态图和测试语法：

- [行为规格](./order-state/SPEC.md)
- [候选语法 A](./order-state/candidate-a.sico)
