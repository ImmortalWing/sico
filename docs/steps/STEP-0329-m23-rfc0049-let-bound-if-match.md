# STEP-0329: M23 item 2 — RFC-0049 let 绑定 if/match 语句形

> - status: implementation complete locally / independent CI pending（M23 为并行轨；owner 2026-09-30 指令「现在有个代理在完成M22，你来完成M23」授权与 M22 并行推进）
> - phase: M23 language v1 batch 3 — item 2（M23 plan §8.2；已接受 RFC-0049）
> - completed: 2026-09-30
> - evidence: internal-fixture，Windows x64 真实 runner（`1.98.0-x86_64-pc-windows-gnu`）

## Objective / Changes

- **Parser**（`crates/sico-parser`）：`opener` 识别两种 let 绑定块头
  ——`let x = if …:` 关 `end if`、`let x = match …:` 关 `end match`（位置化
  检测：第 4 个有效 token 为 If/Match 且行尾为冒号，其他 let 行不受影响）；
  RFC-0049 §2 探针记录的 `MismatchedClose { expected: Function, actual:
  If|Match }` 在最小合法形上消失为真实下沉。顶层 `let x = if` 仍被
  UnexpectedTopLevel 拒绝（块开启不改变顶层拒绝面）。
- **Semantics**（`crates/sico-semantics`）：walker 新增 let 绑定块处理臂
  （置于既有 let 臂之前）——
  - `check_let_bound_if`：条件走 statement 语法（RFC-0044/0048，含
    `check_condition_atoms` 的 E2025 原子检查）；`else:` 必备
    （**E2027 LET_BIND_MISSING_ELSE**，span = let 头）；每臂恰一条表达式
    语句（**E2026 LET_BIND_ARM_NOT_EXPRESSION**，span = 越界语句）；
    双臂类型统一（既有 E2001，指向第二臂）。
  - `check_let_bound_match`：主语照 statement match 推断；复用
    `check_match` 的 E3xxx 穷尽性/封印规则（行类型为 Let 不影响其按行号+
    深度扫描 case 臂）；每臂单表达式（E2026）；模式变量按既有口径推断为
    unknown。
  - 绑定可见性：绑定类型在区域 `end` 行入账（pending 表按 end 行号索引），
    臂表达式无法读到正在定义的名字；name 随既有 open_scopes 机制在封闭
    区域关闭时移除。
- **Lowering**（`crates/sico-ir`）：`body_needs_general_cfg`（由原
  has_general_control 内联式提取为具名函数，语义逐字保持）新增 let 绑定块
  分支 → 通用 CFG；`run` 的 Let 臂优先识别块形：
  - `lower_let_bound_if`：`find_region_close`（allow_else）+ 两侧
    `single_arm_expression` 单表达式校验（E2026 文案）+ 缺 else named 拒绝
    （E2027 文案）；条件经 STEP-0328 的 `lower_condition_branch` 路由到
    then/else 臂块；每臂求值后 `WriteLocal` 写入 join cell；双臂类型不等
    named 拒绝；cells 表在双臂写完后的 join 处插入——臂内自引用在 build
    期必然失败（cell 未入表）。
  - `lower_let_bound_match`：主语求值 + 臂区间扫描与 statement match 同构；
    单表达式臂校验；payload 绑定（`case ok(v)`/`case error(e)`）的 spill 与
    statement match 逐字一致（Project + `#match<v>` cell）；join cell 在
    全部臂写完后入表；`Terminator::Match` 路由各臂。
  - 无新 IR 操作、无新 intrinsic（RFC §1 反糖承诺）；既有 let/`?`/match
    路径逐字节未动。
- **Formatter**（`crates/sico-format`）：`BlockState` 增 `let_bound` 标记；
  `opener_close` 按同一位置化规则把 let 绑定块头压栈（close = If/Match）；
  RFC §5 规范形状——臂缩进一层、`else:`/`case …:` 在 header 深度（statement
  形的既有缩进约定不变）。

## Evidence（本机实测）

- e2e 语料 `tests/end-to-end/let-bind.sico`（+ `.test.json`，格式化规范形
  即提交形）：标识符臂/内建调用臂的 `let x = if`（`<` 与 `>` swap 原子、
  `&&` 条件、`I64.equal` 回退面）；`let x = match` 的 Result 双臂
  （payload `sum` 用于臂表达式）与用户 enum 四臂；join 后值经 concat 汇入
  stdout 且 let 绑定的 `exit_code` 进入返回构造——真实 runner 字节精确
  `vif-taken4eqand7limit|bound`（exit 0），重复运行确定。
- 拒绝语料（`runner/sico-runner/tests/language_batch3.rs` 扩至 14 测试）：
  - E2026：臂内 `set` + 表达式两语句；臂内 `return`；match 臂两语句。
  - E2027：缺 `else:`。
  - E2001：双臂 `Text` vs `I64`（span 指向第二臂）。
  - 注解形 `let x: I64 = if …`：parse 拒绝
    （`MismatchedClose { expected: Function, actual: If }`——注解移位后不
    满足位置化识别，落入既有拒绝面，typed 不变）。
  - 嵌套 if 表达式条件：check 绿 + build named 拒绝（`core expression`
    通道，RFC §3 允许）。
  - limit+1：254 层「语句 if（保持开启）+ let 绑定 if（层内闭合）」恰在
    MAX_PARSE_DEPTH=256 内可构建；255 层 `NestingLimitExceeded { limit:
    256 }` 拒绝。
- 矩阵：`application-profile-v0.json` 新增 executable 行
  `let-bound-if-match`（RFC-0049/STEP-0329）+ refused 行
  `let-bind-arm-shape-violations`、`expression-position-conditions`；
  `exit-corpus` 登记 `let-bind-fixture`。
- validator：`tools/validate-step-0329.ps1` 复现以上全部检查 +
  let-bind 语料的格式化规范形/幂等断言，exit 0。

## Shared-tree note（并行工作树事实）

回归期间 `runner/sico-runner/tests/selfhost_compiler` 曾出现 14 例 prefix
字节差分失败；经逐 crate 暂存二分定位，根因是并行 M22 代理当时工作树中
未提交的 `selfhost/parser.sico` 中间态（其 STEP-0327 实现进行中），非本
STEP 改动——该代理提交后，本树以全部 STEP-0328/0329 改动重跑
selfhost_compiler **58/58 绿**。此记录如实留档，说明共享树并行执行的两类
瞬时假阳性（其一为 `bootstrap_bundle` 冻结清单需按 RFC-0047 record 先例
显式登记 post-freeze 语料，已在 `language_batch3` 落地时一并处理）。

## Honest residuals

- `diagnostics/catalog.json` / semantic-case-map 未扩展（E2023/E2024/E2031
  同批先例，独立收口项）；action-hint 注册表已随本批落地
  E2025/E2026/E2027（`sico explain` 可查 + crate/CLI 测试），M23 出口门 4
  对新码完整闭合。
- 自托管源不消费 let 绑定形（248 处手工 cell 模式不自动改写）——M23 §9 /
  RFC-0049 §8 的显式 re-baseline 决定另行登记；M22 差分不受影响。
- item 3（裸字面量复称）保持 deferred（owner 凭据外部门）；M23 整体出口
  审计见同日报告。
