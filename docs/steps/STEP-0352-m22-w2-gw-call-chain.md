# STEP-0352 — M22: gw 调用链机器 `gw_call_chain_packed`（checked_add RHS 调用链），census 54→57、canary 30/30 收官

> - 状态：implementation complete locally / independent CI pending（本机 runner clippy 缺 gcc 环境阻塞，CI 裁决）
> - 里程碑：M22 compiler self-host（S6 全源差分推进）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**在途 WIP 吸收**：接手上一会话未完成的 `gw_call_chain_packed`（+233 行）与
`gw_checked_add_subject_packed` RHS 调用链分支，补完三处缺陷后整片落地：

1. **cell 重声明（build 拒绝）**：`gw_call_chain_packed` 在 checked_add 用户调用臂
   与内建臂的 if/else 双臂各声明一次 `let gcc_out`——`sico check` 通过但
   script-v0 lowering 拒绝（"unsupported cell redeclared"）。修复 = 声明提升到
   分派前（`let gcc_out = ""`），双臂改 `set`。
   **新发现（登记，不扩大化）**：check 接受双臂同名 let、build 拒绝——又一例
   check/build 声明缝隙（与 STEP-0279 关键字参数名缝隙同类，M14
   「check/build/run 无暗隙」gate 候修项）。
2. **`(` 定位 bug（本片根因）**：两处 call 分支假设 `"("` 紧跟被调首词（+1，
   仅单词用户调用成立）；内建点路径 `sico.bytes.length` 等的 `(` 在 +5。
   修复 = 从 expr_idx 向 bound 扫描第一个 `"("` token（`gw_call_chain_packed`
   与 `gw_checked_add_subject_packed` 的 is_call 门同步）。修复后
   `checked_add(o, sico.bytes.length(sico.text.encode(word_at(words, i))))`
   形状（word_span_offset 主循环）可下降。
3. **pieces 合并 bug（字节差）**：`gw_checked_add_subject_packed` 共享尾仅当
   `gca_r_count == 1` 才追加 RHS pieces；调用链 count>1 时链指令整体丢失
   （guest IR 缺 read_local/call/intrinsic×2，实证 diff 于 word_span_offset）。
   修复 = 条件改 `0 < gca_r_count`。

**机器能力**（`gw_call_chain_packed`，parser.sico +~250 行）：

- 用户调用：复用 `while_call_rhs_packed` 打包 + `call_instruction_json` 发射
  （结果被调存在性与返回类型检查沿用）；
- 4 个内建：`sico.bytes.length`/`sico.text.length`（u64）、
  `sico.text.encode`（bytes）、`sico.bytes.utf8_decode`（string），
  单参 = param/cell/字面量三形态，多 token 实参递归自打包（嵌套调用链）；
- typed 拒绝面：CALL-SHAPE（尾不匹配）/CALL-TARGET（非上述被调）/
  CALL-ARITY（内建多参）/CALL-TYPE（链返回类型 ≠ checked_add 宽度）/RANGE；
- `gw_checked_add_subject_packed` RHS fallback：字面量判定为空且为调用时走
  新机器，链返回 kind 与 checked_add kind 不匹配如实拒 CALL-TYPE
  （`checked_add(u64, text.encode(...))` 类型错误如实拒绝，实测验证）。

## 2. 前沿更新

- **word_span_offset 翻绿**（0351 登记的 S6 守门人之一）：54 前缀 + wso
  整源字节全等；依赖闭包 census **57/123 翻绿、0 DIFF**（54→57：
  word_span_offset + fixed_literal_const_instruction +
  fixed_literal_operand_json——后两者自身形状本已支持，其闭包内调用链体的
  pieces 丢失是差分根因，pieces 修复连锁治愈）。
- **canary 30/30 FORMATTER-CLOSED**，full_source_exit 0，前沿空——
  formatter 金丝雀单位全覆盖收官，R3 止损计数 = 0。
- **分母修正**：parser.sico 实测 **123** 函数（0351 记 122 有误，本片以
  `(?m)^function` 全量扫描钉死，123 个唯一名）。
- 剩余 66 refuse 地图（census）：RETURN-COUNT（gw_* packed 机器族自身
  lowering 的多返回形状）、MATCH-SHAPE（gw 机器族）、CALL-ARGUMENT
  （fixed_operation 族）、CALL-TARGET（gw_decimal_value 缺 is_digit_token
  被调链）等 + 缺被调连锁——机器族自降是下一波次主线。

## 3. 验证与预算

- `sico check selfhost/checker.sico`：ok。
- 普查探针（临时，按纪律已删）：逐函数传递被调闭包 + oracle 字节对比，
  **flipped=57 refused=66 diffs=0 total=123**（8 路并行，PreparedPool）。
- `report-m22-canary.ps1`：30/30，full_source_exit 0。
- `validate-step-0262.ps1`：`STEP_0262_OK`（call-left/format-code/
  repeat-indent/normalize-source byte-exact，canary=FORMATTER-CLOSED）。
- `validate-step-0245.ps1`：root fmt、语义 oracle、runner fmt、
  M22 回归（bootstrap_bundle + selfhost_checker + selfhost_compiler
  **63/63**）全绿；**runner clippy 本机环境阻塞**（缺 gcc.exe，
  wasmtime-internal-fiber 的 windows.c 需 cc；非代码问题，CI 裁决）。
- 卫生清理：runner 工作区 `selfhost_compiler.rs` HEAD 预存 rustfmt 漂移
  （154 行纯重排，无逻辑）随片清理使 fmt 门恢复可执行。
- 探针按纪律删除；`git diff --check` 绿。
