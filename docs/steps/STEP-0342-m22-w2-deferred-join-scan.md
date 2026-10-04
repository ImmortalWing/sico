# STEP-0342 — M22 W2 S3/S4: 延迟 join 扫描变量独立，`find_body_end`/`count_until_end_function` 翻绿

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

上一片（STEP-0341）的 match 栈落地时，延迟 join（if/else 双臂都 return 时的不可达收口块）
的发射循环**复用 `gmt_len` 作为扫描变量**。`gmt_len` 同时是 match 栈长度——终装配的
终结器循环（`while gmti < gmt_len`）与嵌套 match 的栈深判断都依赖它。真实函数的更深
嵌套在延迟块发射之后仍需读栈长，读到被扫描循环破坏的值即沿 STATEMENT 拒绝。

本片把该扫描变量独立为 `gdefer_scan`（三处循环：deferred-else 目标重排、延迟块排序
发射、块 JSON 组装），`gmt_len` 全程只表达 match 栈长度：

- **A/B 证据**（同一探针、同一语料）：HEAD（a12fa76）上 `find_body_end` /
  `count_until_end_function` / D2 深嵌套形状均拒 `ERR:E-SH-IR-STATEMENT`；本片后全部
  字节全等。简化形状 A（match 臂内 if/else 双臂 return、无后续嵌套）在 HEAD 即通——
  破坏只发生在"延迟 join 发射之后仍要读 match 栈"的更深形状，与二分结论一致。
- **翻绿（parser 35→37/120，canary +2，R3 0）**：`find_body_end`、
  `count_until_end_function`（while + list.get match + 臂内 if/else 双臂 return +
  嵌套 match，line_end 被调前缀字节全等）。
- 普查（prefix28 不动点）重跑：pass1 新翻 5（含上述两函数与 count_params 复证、
  binding_parameter_kind/parameter_kind_by_id 在该前缀下复证），pass2 零新翻——
  组合不动点未移动，机器面无回归。

## 2. 剩余缺口（前沿更新）

- `fn_entry`：被调 `count_until_end_function` 已翻绿后**仍拒 CALL-TARGET**——独立缺口
  （自身调用形状），需单独二分。
- 无 while 函数的顶层 match（header 区 STATEMENT）不动。
- `has_return_at` EXPRESSION、`json_string_token` while+match、`u64_list_at`
  MATCH-SHAPE、`gw_*_packed` 机器自宿主层（GWPACK-OTHER）不动。
- R3 0/4；M22 整体 NO-GO 不变。

## 3. 验证与预算

- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler`：
  **58/58 绿**（新增 find_body_end/count_until_end_function 冻结语料条目，line_end 前缀）。
- 完整 runner 工作区套件复跑两遍全绿（35 个测试目标 ok；首轮一次单测失败位于
  `--test runner` 集成二进制、单跑与两轮复跑均 37/37 绿，按并行负载抖动登记，非本片路由漂移）。
- **本地预算**：`gdefer_scan` 使 `general_while_function_ir` +1 local（244→245）；
  `selfhost_local_bounds` 头寸地板 12→11（登记成本；硬顶 `MAX_LOCALS_PER_FUNCTION`=256 不变，
  后续 gw 切片再加 locals 即触发守卫）。
- `cargo fmt --all -- --check` 绿；`git diff --check` 净；bisect 标记（32 处行号后缀）
  全部清除；`selfhost_probe_tmp.rs` 按纪律删除。
