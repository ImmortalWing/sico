# STEP-0401 — M22: casm trap 根因实锤——arena 耗尽（非值损坏），0365 诊断更正；正式修复 = RFC-0029 预算表修订（下片）

> - 状态：diagnostic complete locally（实验改动全部回退净，源码 = 0400 终态；independent CI pending）
> - 里程碑：M22 compiler self-host（S4 lowering 片，0365 缺陷根因定位片）
> - 证据级别：internal-fixture（函数索引映射 + 内存矩阵实验）

## 1. 根因定位链

1. **15 户 trap 同一 backtrace**：`main(34) → scalar_ir(215) →
   general_while_function_ir(197) → [glue 229] → [glue 216] 陷阱`，
   仅 197 帧地址随户变（= 各户在 general_while 内不同调用点）——
   陷阱在**共享 import 包装层**，非各户特有逻辑。
2. **函数索引映射**（临时 scratch 测试 lower 同名源包）：wasm 索引 ==
   host 模块索引（34=main/197=general_while/215=scalar_ir 精确对上），
   216/229 超出核心函数数 = canonical ABI 胶水帧。
3. **内存矩阵实验**（临时 256MiB arena + 512MiB runner 内存）：
   gw_compare 闭包 trap → **STATEMENT（typed 拒绝）**——arena 不再是
   瓶颈后编译走到真实门。
4. **结论：0365 登记的"wasm trap = 上游 Result 判别式损坏的下游症状"
   诊断更正——真根因 = guest 编译机器函数闭包时 arena（128 MiB）
   耗尽触 realloc 裸 unreachable（0351 登记的同一 trap 机制）**。

## 2. 修复路径（下片 STEP-0402，RFC 级）

按 0351 模式做正式预算表修订：RFC-0029 预算表 guest 线性内存
128→256 MiB（ARENA_LIMIT/MEMORY_PAGES 2048→4096）、宿主授权同步、
确定性组件快照重生成；runner 默认 memory_bytes 128→512 MiB 配套
（census 探针已 512）。本片实验已验证该修订能立即解锁 15 户 typed
门（预期大片级联，类似 0351 的 64→128 解锁全源降低）。

## 3. 验证

- 本片源码净（两个实验常数回退，git diff 确认 canonical.rs /
  runner lib.rs 与 HEAD 无差异）；census 维持 107/130、0 DIFF。
- 实验证据：guest256.wasm（256MiB 构建）+ 512MiB runner 下
  gw_compare 闭包 STATEMENT（对照 128MiB 构建 = trap）。
- 临时 scratch_fnmap.rs 按纪律删除。

## 4. 预算

- R3 连续停滞数保持 0；M22 整体 NO-GO 不变（S6/S7 未进入）。
- 下片 = RFC-0029 预算表修订（正式 arena 加倍）+ 重跑 census 量
  级联。
