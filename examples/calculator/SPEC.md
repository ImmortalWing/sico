# Command-line calculator specification

> 状态：代表性程序草案
> 目的：验证 Sico 的基础类型、错误、模式匹配、能力与 AI 可理解性

## 1. 功能

程序接收三个命令行参数：

1. 左操作数；
2. 运算符；
3. 右操作数。

支持四种运算：

- `+`：加法；
- `-`：减法；
- `*`：乘法；
- `/`：除法。

成功时向标准输出写入计算结果。失败时向标准错误写入一条简短错误信息。

## 2. 输入约定

- Runtime 传给 `main` 的参数列表不包含程序文件名；
- 操作数使用十进制文本；
- 数字的精确语义尚未确定，候选程序暂时使用 `Number`；
- 参数数量必须恰好为三个；
- 不自动忽略额外参数。

## 3. 结果

### 3.1 成功

```text
input:  ["10", "/", "2"]
stdout: "5"
stderr: ""
```

### 3.2 参数数量错误

```text
input:  ["10", "+"]
stdout: ""
stderr: "usage: calculator <left> <operator> <right>"
```

### 3.3 数字错误

```text
input:  ["ten", "+", "2"]
stdout: ""
stderr: "invalid number: ten"
```

### 3.4 运算符错误

```text
input:  ["10", "^", "2"]
stdout: ""
stderr: "unknown operator: ^"
```

### 3.5 除零错误

```text
input:  ["10", "/", "0"]
stdout: ""
stderr: "division by zero"
```

## 4. 正式错误集合

```text
InvalidArguments
InvalidNumber(value: Text)
UnknownOperator(value: Text)
DivisionByZero
```

所有可预期失败都必须由 `CalculatorError` 表达，不使用异常或隐藏控制流。

## 5. 逻辑分层

```text
main
  ├── run
  │   ├── parse_number
  │   ├── parse_operator
  │   └── calculate
  └── error_message
```

- `parse_number`：把文本转换为数字，并映射解析错误；
- `parse_operator`：把运算符文本转换为封闭枚举；
- `calculate`：执行纯计算并检查除零；
- `run`：解构参数并组织纯业务流程；
- `error_message`：把正式错误映射为用户文本；
- `main`：唯一执行控制台副作用的函数。

## 6. 类型、效果与能力

| 定义 | 返回 | 效果 | 能力 |
|---|---|---|---|
| `parse_number` | `Result<Number, CalculatorError>` | 无 | 无 |
| `parse_operator` | `Result<Operator, CalculatorError>` | 无 | 无 |
| `calculate` | `Result<Number, CalculatorError>` | 无 | 无 |
| `run` | `Result<Text, CalculatorError>` | 无 | 无 |
| `error_message` | `Text` | 无 | 无 |
| `main` | `Unit` | `console.write` | `console` |

程序采用“纯逻辑核心 + 副作用外壳”。AI 理解计算逻辑时不需要展开 Runtime 或控制台实现。

## 7. 必须验证的语言能力

- 模块能力导入；
- 函数和显式返回类型；
- `enum` 及携带数据的变体；
- `List<Text>`；
- 固定长度列表模式；
- 字符串模式；
- 完整 `match`；
- `Result<T, E>`；
- 同错误类型的显式传播；
- 局部类型推导；
- 不可变局部绑定；
- 方法或命名空间调用；
- 控制台标准输出与标准错误。

## 8. 测试矩阵

| 参数 | stdout | stderr |
|---|---|---|
| `10 + 2` | `12` | 空 |
| `10 - 2` | `8` | 空 |
| `10 * 2` | `20` | 空 |
| `10 / 2` | `5` | 空 |
| `10 / 0` | 空 | `division by zero` |
| `ten + 2` | 空 | `invalid number: ten` |
| `10 ^ 2` | 空 | `unknown operator: ^` |
| `10 +` | 空 | usage |
| `10 + 2 extra` | 空 | usage |

数字格式化、负数、小数、溢出、`NaN` 和无穷大的语义需要在数字类型设计中补充测试。

## 9. AI 语义概要目标

编译器应能在不输出函数体的情况下生成类似概要：

```text
program: calculator
entry: main(List<Text>) -> Unit
purpose: evaluate one binary arithmetic expression
operators: +, -, *, /
errors: InvalidArguments, InvalidNumber, UnknownOperator, DivisionByZero
effects: console.write
capabilities: console
state writes: none
```

AI 应能仅根据概要和符号签名回答：

- 程序支持哪些运算；
- 什么情况下失败；
- 哪些函数是纯函数；
- 哪个函数产生副作用；
- 修改除零行为会影响哪些测试。

## 10. 候选 A 暴露的开放问题

- `Number` 表示整数、浮点数、十进制数还是数值类型族；
- 枚举和变体的最终语法；
- `ok`、`error` 和 `try` 是语言结构还是标准库类型操作；
- `try` 是否只能传播相同错误类型；
- 列表模式的语法与穷尽性规则；
- `match` 分支块如何明确结束；
- 函数尾部是否必须显式 `return`；
- `main` 如何表达失败和进程退出码；
- 控制台能力是显式导入、入口参数还是效果声明；
- 字符串拼接是否使用 `+`、插值或专用格式化；
- 错误消息是程序职责还是 Runtime 的结构化展示职责。

这些问题由后续代表性程序和候选语法共同决定。
