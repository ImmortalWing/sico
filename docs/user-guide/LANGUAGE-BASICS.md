# 语言基础与可运行子集

Sico `main`（`0.1.0`）使用 B Labeled Blocks 语法。语言前端已经能检查比 Runtime codegen 更丰富的类型和控制流，因此必须区分“检查通过”和“可以构建运行”。

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

顶层只允许声明。可执行入口必须写成显式 `function main(...)`；直接书写顶层语句或 `script:` block 会产生 E1013，编译器不会静默忽略这些行。

入口必须名为 `main`。经 Wasmtime 端到端验证的入口返回类型取决于 profile：

- `component-v0`（`sico build` 默认）：`Int`、`Bool`、`Unit`；
- `script-v0`（`sico run`/`sico build --profile script-v0`）：除上述类型外，还支持 `Result[T, NumericError]` 返回——`?` 错误传播要求所在函数返回该类型，否则在 check 阶段报 E3101。

## 函数形式

```sico
function identity(value: Int) returns Int:
  return value
end function
```

参数使用 `name: Type`，返回类型写在 `returns` 后。函数声明与一般调用已全链路可执行（跨函数边界的用户类型、all-return match 均已冻结）；一般递归也可运行，耗尽时得到 typed 的 `RunOutcome::StackLimit`（退出码 125），runner 保持可复用。精确的逐项拒绝以 [application-profile-v0](../../tests/language-matrix/application-profile-v0.json) 矩阵为准；不要仅凭 `sico check` 成功就假定可运行。

## 变量与控制流

前端和静态语义覆盖 `let`、`if`、`match`、record/enum 与名义类型、`Result`/`Option`/`try`、capability/effect、resource、Task/Future/Stream 与 revision dataflow。其中已具有 Runtime codegen、可直接运行的子集包括：

- `let` 局部绑定、`if`、`match`（all-return 形态）、`while`、`for` 迭代（List、Map keys、Set[Text]，STEP-0175）、`break`/`continue`、`set` 可变单元（STEP-0130）；
- 条件中的中缀比较（`<` `<=` `>` `>=` `!=` `==`）与逻辑与 `&&`（RFC-0048，STEP-0328），以及 let 绑定的语句形 `if`/`match`——`let x = if …:` … `end if`、`let x = match …:` … `end match`（RFC-0049，STEP-0329）；端到端样本 `tests/end-to-end/infix-comparisons.sico`、`tests/end-to-end/let-bind.sico`；
- record/enum：构造、dot 字段访问（含链式）与 `set` 整记录替换（RFC-0047，STEP-0271..0273；typed 拒绝面 E2010–E2024）；条件中的中缀比较与逻辑与、let 绑定语句形另见上一条（E2025–E2027）；`List[record]` 仍 gated（E2031）；
- `Result`/`Option`、checked 算术（STEP-0134）、位运算（STEP-0132）与 `?` 错误传播（STEP-0175）；`try` 表达式仍仅 check 阶段；
- `Map`/`Set`（STEP-0131，实例化集合受限）与 `List[Text|I64|U64]`；
- task scopes 的顺序执行（parallelism-1）；一等 Task/Future/Stream 值仍为 typed 拒绝；
- module/use（STEP-0143）与 `use pkg` 包导入（STEP-0147）、字符串转义（STEP-0139）。

完整与唯一权威的逐项清单见 [application-profile-v0](../../tests/language-matrix/application-profile-v0.json)。不支持的形态得到明确的 typed backend refusal，而不是不可验证产物。

## Application profile（M14 可运行清单）

M14 出口审计（STEP-0141）确认应用 profile（`script-v0`）无未声明的 check/build/run 缺口。经真实 runner 端到端验证的能力包括：

- 通用控制流 `while`/`if`/`break`/`continue`/`set`（STEP-0130）；
- `Map`/`Set`（STEP-0131）、位运算（STEP-0132）、`checked_mul`/`checked_div`（STEP-0134）；
- 纯 Sico 源码 xorshift64 PRNG（STEP-0135，前 8 个输出对 Python 参考向量钉死）；
- 有界递归与 typed `StackLimit`（STEP-0137）、离线方块求解器（STEP-0138，与冻结 Python oracle 字节级一致）；
- http@0.2.0 源码级 `sico.http2.request`（STEP-0136，含未授权端点的 typed 拒绝）；
- 流式 stdin/stdout 转换、capability 状态机与字符串转义（STEP-0139）；
- `for` 循环与 `?` 错误传播（STEP-0175）、module/use（STEP-0143）、`use pkg` 包导入（STEP-0147）、record 类型（STEP-0271..0273）。

M14 之后继续扩面且已具备真实 runner 证据的语言项：中缀比较条件与 `&&`（STEP-0328）、let 绑定的语句形 `if`/`match`（STEP-0329）——见 [M23 出口审计](../reports/m23-exit-audit-2026-09-30.md)，其 item 3 仍为 deferred-external，不代表 M23 整体收口。

逐项的 check/build/run 结果、实例化集合与拒绝面以 [tests/language-matrix/application-profile-v0.json](../../tests/language-matrix/application-profile-v0.json) 为唯一权威来源；可执行端到端样本位于 `tests/end-to-end/`。

最小可运行示例：

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

建议先用 `sico check`，再用 `sico run`（或 `sico build --profile script-v0`）验证 backend 支持。构建失败时不会创建或覆盖目标产物。

## 示例目录的含义

根目录 `examples/` 保存语言设计历史和编译器回归样本，其中一些文件是旧候选语法，不应直接复制为当前应用。当前 B 语法的正反例位于 `syntax-candidates/b/`，可执行端到端最小样本位于 `tests/end-to-end/`，完整发布工作流位于 `pilots/third-party-component/`。

语义和语法的设计级说明见根目录 `SEMANTICS.md` 与 `SYNTAX.md`；它们不是稳定 1.0 规范。
