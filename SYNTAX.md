# Sico syntax candidates

> - 状态：第一轮候选设计
> - 语义基线：[`SEMANTICS.md`](./SEMANTICS.md)
> - 判定集：[`semantic-cases/`](./semantic-cases/README.md)
> - 原则：只比较写法，不改变程序含义

## 1. 目的

本文档定义 Sico 表层语法候选的比较方法。任何候选都必须先满足同一语义判定集，然后才能比较简洁度、AI 生成正确率和错误恢复能力。

当前三套候选分别强调：

| 候选 | 核心思路 | 主要假设 |
|---|---|---|
| A / A0 | 缩进 + 通用 `end` | 少标点比带名称的结构更容易生成 |
| B | 关键字 + 带名称的结束标记 | 冗余的结构标签能减少错配和级联错误 |
| C | 花括号 + 表达式式 match | 模型熟悉的紧凑结构能降低 token 和生成成本 |

候选源码位于 [`syntax-candidates/`](./syntax-candidates/README.md)。第一轮只覆盖前 30 个 P0 语义案例，不代表完整语言语法。

## 2. 所有候选必须满足的约束

### SYN-001：语义结果相同

同一个 case ID 在 A、B、C 中必须得到相同的 `accept` 或 `reject(KEY)`。候选不得通过隐式转换、自动错误映射、默认分支或隐藏能力改变判定。

### SYN-002：一种结构只有一种规范写法

规范格式化后，同一个 AST 不应同时保留多种等价格式。可选分号、可选逗号、同义关键字和可省略结束形式都会扩大 AI 的生成空间，应默认避免。

### SYN-003：不同语义优先使用不同形式

以下概念不得继续共用难以区分的单一形式：

- 导入与获得能力；
- 忽略载荷与接住未来变体；
- record 更新与资源作用域；
- Result 领域错误与 Component trap；
- 泛型参数与接口版本；
- match 分支与 lambda。

### SYN-004：错误必须局部恢复

缺少一个结束符、分隔符或分支时，解析器应在当前定义内恢复。单个根因不应使后续所有定义失去结构。

### SYN-005：签名可以独立阅读

公共函数的参数、返回、错误、异步性质、效果和能力必须能从签名或紧邻的机器可读声明中获得，不能要求扫描整个函数体。

### SYN-006：不依赖自然语言补全含义

注释可以解释目的，不能决定类型、错误、能力、匹配完整性和资源生命周期。

## 3. 候选 A / A0

示意：

```sico
enum Color
  Red
  Green
end

fn name(color: Color) -> Text
  match color
    Color.Red => return "red"
    Color.Green => return "green"
  end
end
```

特点：

- 标点少；
- 一个 `end` 关闭多种结构；
- `=>` 同时容易被用于 match 和 lambda；
- `_` 容易同时表示忽略载荷与通配分支；
- 多层嵌套时，结束标记需要向上寻找对应结构。

A0 是语义案例的当前记法，不是已胜出的候选。

## 4. 候选 B：Labeled Blocks

示意：

```sico
enum Color:
  case Red
  case Green
end enum

function name(color: Color) returns Text:
  match color:
    case Color.Red:
      return "red"
    case Color.Green:
      return "green"
  end match
end function
```

规则：

- 所有多行结构都有 `:` 头和带名称的结束标记；
- enum 变体和 match 分支都写 `case`，但一个位于类型声明、一个位于 `match` 内；
- 使用 `ignore` 忽略已知变体载荷；
- 使用 `else` 表达通配，语义检查仍可拒绝封闭 enum 的通配；
- 泛型类型使用 `Type[A, B]`，不使用 `<...>`；
- `function`、`returns` 强化签名边界；
- 表达式语句、`return` 和 `try` 含义保持显式。

预期优势：结构恢复强、结束错配诊断直接、对长函数更友好。预期代价：token 更多、视觉密度较低。

## 5. 候选 C：Compact Delimited

示意：

```sico
enum Color { Red, Green }

fn name(color: Color) -> Text {
  return match color {
    Color.Red: "red",
    Color.Green: "green",
  }
}
```

规则：

- `{}` 明确结构范围；
- enum 变体、record 字段和 match arm 使用必需逗号；
- match 是表达式，分支使用 `pattern: value`；
- `ignore` 只忽略已知载荷，`else` 才是通配；
- 泛型使用 `Type[A, B]`；
- Result 构造为 `Ok`/`Err`，同错误传播使用 postfix `?`；
- 函数仍要求显式返回类型，非 Unit 路径不能隐式落空。

预期优势：token 少、训练语料熟悉度高、match 数据流紧凑。预期代价：标点遗漏可能增加，嵌套括号错误恢复需要实测。

## 6. 第一轮评测指标

### 6.1 静态结构指标

| 指标 | 计算方式 | 越低/越高越好 |
|---|---|---|
| 源码字符数 | 去除案例元数据后的字符数 | 越低越好 |
| 词法 token | 使用候选独立词法器计数 | 越低越好 |
| 结构标记数 | 块开始、结束、分隔符数量 | 结合错误率判断 |
| 最大结束距离 | 块开始到对应结束的行数 | 越低越好 |
| 符号过载数 | 同一符号承担的不同语义数量 | 越低越好 |
| 唯一规范格式率 | 同一 AST 格式化后是否唯一 | 100% |

### 6.2 AI 生成与修复指标

对每个案例使用相同提示、相同模型版本和相同采样参数，至少重复 30 次：

- 首次解析成功率；
- 首次类型检查成功率；
- 目标 accept/reject 命中率；
- 修复一个注入错误所需轮数；
- 单轮修复率；
- 诊断输入与修复输出 token；
- 修复时意外改变语义的比例。

### 6.3 AI 理解指标

仅提供目标函数及编译器语义切片，要求回答：

- 输入、输出和正式错误；
- 匹配是否完整；
- 哪些类型不能混用；
- 哪些 Result 被传播或映射；
- 修改 enum 后哪些分支受影响。

记录回答正确率、输入 token 和引用错误符号的比例。

### 6.4 错误恢复指标

对合法程序机械注入以下单一错误：

- 删除一个块结束；
- 删除一个逗号或冒号；
- 拼错一个类型名；
- 删除一个 match arm；
- 将一个 `Ok` 改为 `Err` 或反向；
- 把一个领域类型替换成同底层的其他类型。

记录主要诊断位置、级联诊断数量、解析器恢复到下一个定义所需距离以及 AI 单轮修复率。

## 7. 通过门槛

候选进入下一轮前必须满足：

1. 30 个 case ID 一一对应；
2. accept/reject 矩阵完全一致；
3. 每个 reject 的主要诊断根因一致；
4. 不引入语义文档未允许的隐式行为；
5. 可以唯一格式化；
6. 删除单个结构符号后可以在当前定义内恢复；
7. AI 生成和修复评测不显著劣于其他候选。

未达到门槛的候选可以吸收其他候选的局部设计，但必须保留独立历史，不能只保留最终版本而丢失比较证据。

## 8. 当前不评测的内容

第一轮暂不比较：

- 模块、import、effect 和 capability 的最终写法；
- borrow、move、share 和资源作用域；
- async、Future、Task 和 Stream；
- interface、WIT world 和版本；
- UI SDK；
- 文档注释和属性系统。

这些内容在相应 P0 语义案例完成后加入第二轮语法评测。

## 9. 下一步

1. 对 A、B、C 进行结构计数；第一轮结果已记录在 [`syntax-candidates/METRICS.md`](./syntax-candidates/METRICS.md)；
2. 定义最小词法器或独立 token 计数规则；
3. 维护已建立的 [`syntax-mutations/`](./syntax-mutations/README.md) 单点错误注入清单；
4. 人工复核三套候选是否保持相同语义；
5. 按 [`ai-eval/`](./ai-eval/README.md) 协议运行 AI 生成、理解和修复实验；协议与离线工具已建立，真实模型数据尚未测量。
