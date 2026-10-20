# STEP-0412 — M22: gw_rhs_multi_pack split_lines 分支抽辅助——census 117→117/136、0 DIFF（分母 +1，绿维持）

> - 状态：implementation complete locally（本机全量回归见 §4；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，深嵌手术片 5）
> - 证据级别：internal-fixture / runtime-verified

## 1. 交付内容

**split_lines 复杂分支体抽辅助 `gw_rhs_split_lines_emit`**（0411 方法的
复杂分支版）：44 行分支体（含 inline 发射、raw-pack 直返路径）移入
helper，累加器 set 重定向为 helper 局部（gws_sl_kind/gws_sl_pieces），
尾装配四行 pack 返回；caller 臂内 = 单调用 + 单守卫 + 解包 9 行。
关键语义验证：分支的 raw-pack 直返（`return gw_intrinsic_call_packed(...)`
原样传播 callee 结果）与四行 pack 同构（pack 格式本身就是
count\nvalue\nkind\npieces）→ 可统一走单守卫协议。

## 2. 前沿更新

- census **117/135 → 117/136、0 DIFF**（+1 分母 = 新 helper；绿维持；
  CALL-SHAPE 1→2 一户换挡）；无回归。
- 剩余手术面：multi_pack encode/length 均匀对（intrin4_direct 辅助可
  复用 0411 模板）+ final-else 字面量路径 + GWPACK-OTHER 13 + trap 4。

## 3. 事故与登记

- 首跑 splice 保留 `else:` 致 MismatchedClose（替换须含头含尾整链或
  只换体保头）；二跑累加器重定向漏变体（`concat(gws_pieces, ...)` 与
  `concat(gws_sl_pieces, ...)` 两种形态并存）——set 重定向必须枚举
  实测全部形态，不得只匹配预期。

## 4. 验证与预算

- census 终值：flipped=117 diffs=0 total=136；HIST GWPACK-OTHER 13 /
  CALL-SHAPE 2 / NON-DOMAIN 4；探针副本按纪律删除。
- 固件：无新测试；81 项既有固件原位保持。
- 本片全量电池重跑全绿：fmt（根 + runner）、validate-step-0262
  （81/81 + parser 2/2 + bounds 1/1）、validate-step-0245、
  report-m22-canary（FORMATTER-CLOSED 30/30、full_source_exit 0）、
  validate-step-0124 -SelfTest、git diff --check。
- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。

## 6. STEP-0413 方法定型（同日，源码净 = 0412 终态）

嵌套消解的完整模板库已定型（下片起批量执行）：
1. **early-return 替 else**（then 恒 return 时）：`if c: T[ret] else: E end if`
   = `if c: T[ret] end if + E`——无否定需求；
2. **pack-helper 代分支对**（互斥可报错 callee）：每臂一 helper 各自
   call+guard+consume+尾装配，helper 返回四行 pack 或 ERR，dispatch 层
   `if path: return helper(ctx) end if` 全 flat（0411/0412 已验）；
3. **操作数析取 helper**（param/cell 二元）：helper 内 early-return
   形式（param 臂直接 return pack4，非 param 体紧随其后），天然 flat；
4. **识别级联**（gwsdf 型 5 深层）：逐级 helper（level-k: `if tok:
   return level-k+1(ctx) end if / return ERR`）。

剩余量：multi_pack 内 ~8-10 个模板实例（encode/length/fallback/
gwf_l/gwf_r/gwsdf×N）→ GWPACK-OTHER 13 → trap 4。每片 1-2 实例的节奏
≈ 每片 +1 绿。
