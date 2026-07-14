# Sico examples

这些示例用于反推语言语义、语法和 AI 可理解性。目前全部属于设计样本，不代表已经稳定的语言规范。

- [代表性程序计划与覆盖矩阵](./PLAN.md)
- [跨样本问题矩阵](./ISSUES.md)
- [候选 A 语义审计](./SEMANTICS-AUDIT.md)
- [P0 语义正反例集](../semantic-cases/README.md)

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

## `account-transfer/`

- [行为规格](./account-transfer/SPEC.md)
- [候选语法 A](./account-transfer/candidate-a.sico)

## `todo-store/`

- [行为规格](./todo-store/SPEC.md)
- [候选语法 A](./todo-store/candidate-a.sico)

## `word-count/`

- [行为规格](./word-count/SPEC.md)
- [候选语法 A](./word-count/candidate-a.sico)

## `http-service/`

- [行为规格](./http-service/SPEC.md)
- [候选语法 A](./http-service/candidate-a.sico)

## `concurrent-fetch/`

- [行为规格](./concurrent-fetch/SPEC.md)
- [候选语法 A](./concurrent-fetch/candidate-a.sico)

## `component-plugin/`

- [行为规格](./component-plugin/SPEC.md)
- [候选语法 A](./component-plugin/candidate-a.sico)

## `notes-ui/`

- [行为规格](./notes-ui/SPEC.md)
- [候选语法 A](./notes-ui/candidate-a.sico)
