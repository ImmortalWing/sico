# STEP-0341 — M22 W2 S3/S4: gw 机器 match 栈 + `count_params` 翻绿

> - 状态：implementation complete locally / independent CI pending（gitcode 已推，GitHub 待连通）
> - 里程碑：M22 compiler self-host（W2 S3/S4 lowering 片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

general-while 机器的 match 状态从**单槽**改为**栈**，嵌套 match（match 臂内再开 match，即 `count_params`/`find_body_end`/`count_until_end_function` 三个计数器的核心形状）由此可下降：

1. **梯级门改路由**：`scalar_ir` 中 match>1 且 while==0 的 `ERR:E-SH-IR-CONTROL` 拒绝门删除；general_route 计算新增 `match_count > 1 → general_route_count = 1`——多 match 函数一律路由 gw 机器（match==1 仍走既有 lgm/casm 单 match 机器）。
2. **`gw_match_open_pack`**：删除单槽 `match_open==1 → CONTROL` 检查（嵌套打开放行；entry/after/header 区域与 cur_term 约束不变）。
3. **match 栈**：gw 机器新增 `gmt_stack`（`split_lines("")` Text 列表）+ `gmt_depth`；match 打开时若已有开 match 则 push 当前 `gmt_p_pack`、depth+1；`end match` 时 depth>0 则 pop 恢复外层 pack（open 保持 1）、depth==0 才清 open。块列表（gblk_*）全局追加无需栈化——嵌套打开天然闭合当前块；终结器汇编在终装配按 entry 查表（per-entry 自包含，完成序 = 嵌套逆序无影响）。

**翻绿（parser 34→35/120，canary +1，R3 保持 0）**：`count_params`（while 体内 `list.get` match + if-then 内嵌套 `checked_sub` match，line_end 被调前缀字节全等）。探针矩阵同时证实：while 内 match 臂 return（R2）、嵌套 match+臂内 return（R4）均已通。

## 2. 剩余缺口（本轮二分钉死，下片目标）

- **R3 类**：match 臂内 if/else **双臂都 return**（find_body_end/count_until_end_function 的真实形状）→ `ERR:E-SH-IR-STATEMENT`——if 关闭后的 join 块在双臂终结时不可达，gfr 机制未建模"后支配 return 的 if 收口"，case 处理器需接受 gcur_term=true 的臂尾。翻绿这两个计数器后 `fn_entry` 等消费者连锁解锁。
- **header 区域 match**：无 while 函数的顶层 match（N1/N2）→ STATEMENT（region 门）；需要 entry/header 区域的 match 支持。
- R3 0/4；M22 整体 NO-GO 不变。

## 3. 验证与预算

- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler`：**58/58 绿**（含 count_params 冻结语料条目）。
- **完整 runner 套件全绿**（34 个测试目标，路由变更漂移检查通过）。
- **本地预算**：match 栈使 `general_while_function_ir` 增加 4 locals（240→244）；`selfhost_local_bounds` 守卫头寸 16→12（注释登记成本；硬顶 `MAX_LOCALS_PER_FUNCTION`=256 不变，后续 gw 切片再加 locals 即触发守卫——预算有界，下片须复核）。
- `git diff --check` 净；`scratch_matchstack_probe.rs` 按纪律删除。
- oracle 转储钉死嵌套 match 的块/终结器结构（`#match<local>`/`#match<bind>` 双 cell、match 终结器 arms/双 range）。
