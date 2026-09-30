# STEP-0328: M23 item 1 — RFC-0048 中缀比较条件与 `&&`

> - status: implementation complete locally / independent CI pending（M23 为并行轨；owner 2026-09-30 指令「现在有个代理在完成M22，你来完成M23」授权与 M22 并行推进）
> - phase: M23 language v1 batch 3 — item 1（M23 plan §8.1；已接受 RFC-0048）
> - completed: 2026-09-30
> - evidence: internal-fixture，Windows x64 真实 runner（`1.98.0-x86_64-pc-windows-gnu`）

## Objective / Changes

- **Lexer**（`crates/sico-lexer`）：按 RFC-0048 §4 冻结集追加 token
  `LessThan`/`GreaterThan`/`GreaterEqual`/`BangEqual`/`AmpAmp`（append-only，
  既有判别式不动）；`punctuation` 二元组扫描先行（`<=`/`>=`/`!=`/`&&`/`==`
  longest-match），`|`、裸 `!`、裸 `&`、裸 `<`、裸 `>` 中除 `<`/`>` 新单 token
  外全部保持 `UnexpectedCharacter` 词法拒绝（`<`/`>` 单 token 本 RFC 新增）。
- **Semantics**（`crates/sico-semantics`）：`infer_expression` 先按顶层 `&&`
  切分（两侧经 `require_type(Bool)`），再以六元比较 kind 表类型化为 Bool
  （RFC-0044 的 `==`/`<=` 面按 RFC §7 泛化：E2024 消息携带实际运算符文本）；
  新增条件位检查 `check_condition_atoms`——swap 形 `>`/`<=` 操作数限原子
  （字面量/绑定/字段链，括号包裹不改变表达式），首个越界操作数触发
  **E2025 INFIX_SWAP_OPERAND_NOT_ATOM**（append-only 新码，span 指向操作数，
  消息内联 `b < a` 改写指引），每个条件只报一处。
- **Lowering**（`crates/sico-ir`）：新增共享 `lower_condition_branch`（条件 →
  Branch 终结路由到 then/else 目标）：顶层 `&&` 反糖为嵌套分支（右侧条件仅在
  左侧为真时求值——短路是定义行为；每个 `&&` 操作数必须是含顶层比较的条件形，
  否则 named build 拒绝）；比较经 RFC §3 反糖表落到既有 `EqualFixed`/`LessFixed`
  ——`!=`/`<=`/`>=` 分支反转、`>`/`<=` 操作数交换且按反糖序求值（swap 形
  原子限制在 lowering 侧镜像拒绝，E2025 文案一致）；链式比较
  （`a < b < c`）named 拒绝；其余形状回退既有 Bool 表达式面。
  `if` 处理器：batch-3 形状先于冻结 `==` 路径（裸 `==` 字节路径原样保留，
  `is_revision_if_shape` 在 batch-3 形状上让位至通用 CFG）；`while` 处理器：
  batch-3 形状经同一反糖（普查点名需求：`while i < n` 范围扫描头），既有的
  中缀算符 while 拒绝面按 RFC §5 加宽为接受；值位 `&&` 在 `expression` 中
  named 拒绝（`&& outside a condition position`）。
- **Formatter**：新 token 走 `needs_space` 默认表——RFC §6 的两侧各一空格
  规范由既有规则自动满足，`a<b` 规范化为 `a < b`，两次格式化幂等。
- **冻结面不变**：既有语料的 IR 字节路径未动（裸 `==` if 冻结路径、
  revision-if 快速路径、strict-match、straight-line 均在 batch-3 形状之外）。

## Evidence（本机实测）

- e2e 语料 `tests/end-to-end/infix-comparisons.sico`（+ `.test.json`）：六算符
  × I64/U64、`<=`/`>=` 边界等式、swap 形原子操作数、`!=` 反转、`&&` 三层嵌套
  与 while 头 `&&`（短路跳过的迭代可由退出状态 i=3/hits=2 观测）——经真实
  runner 字节精确，`expect_stdout` 91 字节，重复运行确定。
- 拒绝语料（`runner/sico-runner/tests/language_batch3.rs`，7 测试）：
  - check 通道：E2025（`>` 与 `<=` 的 call 操作数）；`||`/裸 `&` 词法拒绝。
  - build 通道：混合宽度/裸 Int（NUM-001 不变）/Text/Bool 操作数 →
    `infix comparison condition operand types`；链式比较；`&&` 值位与
    `&&` 非条件操作数。
  - limit+1：255 层嵌套 batch-3 条件块恰在 MAX_PARSE_DEPTH=256 内可构建，
    256 层 `NestingLimitExceeded { limit: 256 }` 拒绝。
- 矩阵：`application-profile-v0.json` 新增 executable 行
  `infix-comparison-conditions`（RFC-0048/STEP-0328）+ 三条 refused 行；
  `exit-corpus` 登记 fixture 与 runner 测试。
- formatter：紧凑输入规范化为 `a < b && b >= I64.literal(7)`，两次格式化
  幂等。
- validator：`tools/validate-step-0328.ps1` 从零复现以上全部检查，exit 0。
- runner 套件：`language_batch3` 绿（STEP-0328 落地时 7 测试；STEP-0329 扩
  至 14 测试后复跑保持全绿）；根工作区 92 套件 + runner 工作区
  selfhost 差分全套（compiler 58/58、parser 2/2、checker 9/9、formatter
  1/1）绿；冻结语料回归见同日登记。

## Honest residuals

- M23 exit gate 4 的 action-hint 注册表已为 E2025 注册（与 E2026/E2027 一同
  落地，`sico explain` 可查，带 crate/CLI 测试）；E2025 的可行动改写亦内联
  在消息文本中。`diagnostics/catalog.json` / semantic-case-map 的全量协议
  扩展仍欠（E2023/E2024/E2031 同批先例，独立收口项）。
- `diagnostics/catalog.json` / semantic-case-map 未扩展（同上先例：
  E2023/E2024/E2031 均未进入该协议；扩展将触发 33 案例集全量重构，超出本
  STEP 范围）。
- 自托管（selfhost）侧不消费新表面（M23 §9 / RFC-0048 §9 的显式 re-baseline
  决定，另行登记）：M22 差分不受影响（selfhost 源 0 个 ` == ` 出现）。
- M23 整体出口审计未做（待 item 2 落地后一并进行）。
