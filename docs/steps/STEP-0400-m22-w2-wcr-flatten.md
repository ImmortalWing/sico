# STEP-0400 — M22: 函数级巨大 if 顶层化——wcr 翻绿连锁 19 户，RETURN-COUNT 族灭，census 104→107/130、0 DIFF

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，深嵌容量攻坚首片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 交付内容

**wcr 巨大 if 顶层化三步**（0399 阈值标定的首应用）：
1. 函数级 `if args==close: <嵌套 if 快速路径> else: <300 行主体> end if`
   反守卫主体顶层化（`if cond: 嵌套块 end if` + 主体反缩进）。
2. 嵌套快速路径抽辅助 `while_call_rhs_empty_args`（0343 纪律：then 臂
   内任何带 return 的 if 都是顶层深度 2 禁形，唯一正解是抽函数）。
3. while 体内 `set scl_scan = next_index(matching_close(...), ...)` 嵌套
   调用拆 let+set（aic 模板）。

**gic_n 插曲（回退净）**：`bytes.length` 两形（提升 cell / 嵌套 encode）
在 gw 上下文双拒；match 主语形会把 gic 路由进 checked_add 主语族
（红户）——两害相权维持基态，登记待 checked_add 主语族翻绿后重试。

## 2. 前沿更新

- census **104/129 → 107/130、0 DIFF**（+1 = 新 helper 入分母）；
  **RETURN-COUNT 19→0 族灭**（19 户 typed 门全清）；**wcr +
  gw_text_trim_user_call_packed 翻绿**；级联：NON-DOMAIN 2→17（15 户
  走进 0365 登记 trap 类——新前沿主体）、CALL-SHAPE 1→3、GWPACK-OTHER
  2→3；无回归。
- **前沿主体已收敛为单一缺陷类**：0365 wasm trap（上游值损坏族）。
  下片 = casm trap 专项攻坚（0365 登记正体）。

## 3. 事故与登记

- 手工正则两次误切巨大 if（小守卫误中/臂边界错配）——块结构变换
  必须先打印结构再动刀；缩进异常体（flatten 遗留的同级缩进）以
  sed -n 实况为准，勿凭假设。
- wcr 截体扫描被 gw_instr_count 的 STATEMENT 遮蔽（组合探测首错语义）
  ——先修/查闭包内其他红户再截体。

## 4. 验证与预算

- census 终值：flipped=107 diffs=0 total=130；HIST NON-DOMAIN 17 /
  CALL-SHAPE 3 / GWPACK-OTHER 3；探针副本按纪律删除。
- 固件：新测试 `sico_compiler_lowers_while_call_rhs_packed_closure_byte_exactly`
  （35 函数闭包字节全等，簇金丝雀；清单 extract 生成）。
- `--test selfhost_compiler`：**81/81 绿**（含新固件）。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （81/81 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
