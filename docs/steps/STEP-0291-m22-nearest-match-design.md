# STEP-0291: M22 22-C 前置设计——nearest_match 前沿形状实测与三片降低方案

> - status: complete / design + probe evidence frozen; no product source changed, no gate moved, not counted toward R3
> - phase: M22 compiler self-host — W2 S3/S4 preparation (execution card 22-C pre-design; STEP-0251 precedent)
> - date: 2026-09-25
> - evidence class: internal-fixture, real runner probes on the STEP-0282 baseline tree

## 1. Scope and decision

卡 22-C 的入口（W1 GO）仍待 owner 裁决；本 STEP 做**不越权的前置**：
把前沿函数 `nearest_match`（canary 22/30 的第一个未覆盖函数）的失败
形状逐层钉定，形成可立即执行的三片降低设计（STEP-0251 先例：设计
先行，实现另起 STEP）。全程零产品源码改动；探针为临时文件。

## 2. Probe evidence（真实 runner，fuel 5e9，对 STEP-0282 基线树）

探针框架：现建 guest compiler component（`compiler.sico` → script-v0），
runner 喂源码看 exit/stderr。前缀边界 = 下一函数 `m.start()`（首轮框架
曾用 `m.end()` 截断下一函数签名行，产生 `ERR:E-SH-IR-RETURN-COUNT`
伪影，已修正并复核）。

| 探针 | 形状 | 结果 |
|---|---|---|
| prefix through `repeat_indent`（f22） | 已覆盖前缀 | **exit 0**（与 canary 22/30 零矛盾） |
| prefix through `nearest_match`（f23） | 前沿 | `ERR:E-SH-IR-GWPACK-OTHER` |
| A1：while 体 `set cursor = sub(...)`（user-call） | 已支持面 | exit 0 |
| A2：while 体 `let key = sico.u64.to_text(cursor)` | **内建 RHS in while-body let** | `GWPACK-OTHER` |
| B2：while 体 match，主语 `sico.map.get[Text,U64]`，字面键 | **map.get 为 match 主语** | `ERR:E-SH-IR-CALL-TARGET` |
| C2：B2 + ok 臂内嵌 if + `return` | 多层退出 | `ERR:E-SH-IR-CALL-TARGET`（被主语层遮蔽） |

## 3. 三片降低方案（每片一个实现 STEP，按序）

- **S1：while 体 let/set 的内建调用 RHS——`sico.u64.to_text` 首先入
  面。** `gw_rhs_packed` 已支持 concat（STEP-0262）；扩 `gw_intrinsic_
  call_packed` 的内建集加 `u64.to_text`（及其后同片需要 `bytes.length`
  /`text.encode`——`set_nearest_match` 用到）。出口 = A2 形状 exit 0，
  前沿移到 nearest_match 的下一拒绝（match 主语层）。
- **S2：match 主语内建集加 `sico.map.get[Text,U64]`。** 现有 match
  机制只认 `U64/I64.checked_*`（`checked_arithmetic_match_ir`）与
  `sico.list.get`（`list_get_match_ir`）；加 map.get 的 ok/error 形状
  （value 臂绑定为 U64）。出口 = B2 exit 0。
- **S3：match-in-while 框架 + 臂内 if/return 退出。** ok/error 臂走
  既有 if-frame 类似的两分支区域 + 汇合；error 臂自赋值
  （`set cursor = cursor`）与 ok 臂内 `return`（多层退绕到函数尾）
  复用既有 return 机制。出口 = C2 exit 0，随后整前缀
  （through f23）byte-exact 差分过，canary 重钉于 f24
  `set_nearest_match` 的新拒绝。
- **R3 记账（预先声明，防争议）**：每片的"前沿移动"以拒绝码/位置
  变化认定（如 GWPACK-OTHER → CALL-TARGET → 框架层）；canary 函数
  数在 nearest_match 整体过前不动（22/30），由**前沿移动**供给 R3
  进展，与 22-B 基线 JSON 的前沿字段对照记录。
- 回归纪律不变：每片跑 215+5 冻结差分、`gw` 局部 bounds
  （`general_while_function_ir` 240 上限查 top-5）、拒绝面 3 例、
  formatter 幂等；独立 CI 未决时该片保持待裁决。

## 4. Honest limits

- 探针只测当前树；W1 未 GO 前不进入实现（卡 22-C 入口不变）。
- `set_nearest_match`/`match_arm_levels` 的完整形状（bytes.slice/
  utf8_decode/map.put 等）在 S1–S3 逐片到位后仍可能有残余拒绝；
  每片出口以**实测**重钉，不预告。

## 5. Gate accounting

无 gate 变化；M22 = NO-GO；W1 待裁决；R3 未起算（本 STEP 为设计/
测量）。W1 GO 后：S1 即为第一个 22-C 实现 STEP。
