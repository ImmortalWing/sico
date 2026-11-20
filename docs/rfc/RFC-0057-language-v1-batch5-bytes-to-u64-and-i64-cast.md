# RFC-0057: Language v1 batch 5 — `bytes`→`U64` read + `I64`→`U64` cast

> - status: accepted（owner 2026-11-19「接受」）
> - date: 2026-11-19
> - authors: autonomous-agent
> - target language/platform version: draft
> - supersedes: -
> - superseded-by: -

## Summary

提案为 Sico v0 补两个最小数值/字节面能力——**从 `Bytes` 读取一个无符号 64 位整数**（`bytes`→`U64`）与**把 `I64` 显式转换为 `U64`**——以解锁 guest 侧「解码负载 → 取尺寸 → 重编码」这类真实数据管道。二者是 M24 G6 转换器 pilot（STEP-0635）实测的**决定性 v0 缺口**：没有它们，guest 无法把 `image-codec@1` 解码出的 16 字节 LE u64 尺寸头回喂给 `encode-png`，PNG→PNG 转换在纯 Sico 中不可表达。

> **状态纪律**：本 RFC 仅提案，**未实现、不翻任何 gate、不代表任何特性已支持**。落地前须 owner 裁决 status → accepted，并另开实现 STEP（共享序列顺取，不预占号）。M24 overall 不因本 RFC 改变。

## Problem

STEP-0635 实测确认的代表性场景与当前缺口：

- `sico.bytes.at(bytes: Bytes, index: U64, fallback: I64) -> I64`（semantics `lib.rs:4159-4162`）**只回 `I64`**，guest 无法直接从字节缓冲得到 `U64`。
- 探针实测：`U64.shl(U64.literal(1), <I64>)` → **E2001 expected U64, found I64**；`U64.literal(<I64 var>)` → **E2001 expected Int, found I64** + 范围校验拒。即 v0 **无 `I64`→`U64` 的任何转换路径**（显式或隐式）。
- `image-codec@1` 的 `decode_png` 返回 16 字节 LE u64 头（宽度 w、高度 h 各 8 字节），而 `encode_png(pixels, width: U64, height: U64)` 的宽高是 `U64`。
- 包接口戳记门 **E8016** 禁止把接口参数声明为 `I64`（与冻结戳 `u64` 冲突 → 拒绝），故**不能**靠把接口改成 `I64` 绕过。
- **后果**：`encode-png` 的 `U64` 宽高无法从解码负载取得 ⇒ re-encode 在纯 Sico 中不可表达 ⇒ G6 pilot 的 re-encode 止于 typed refusal（STEP-0635 `g6_converter_pilot` 13/13 绿，但 save-path-failure / byte-stable repeat / batch 采样三组 §8.5 corpus 因需 re-encode 而 blocked）。

这是**纯 v0 表达力缺口**，不是 bug：现有设计从未提供跨 `I64`/`U64` 的读取或转换原语。

## Goals and non-goals

**Goals**

1. 提供**显式、typed、fail-closed**的 `Bytes`→`U64` 读取原语（小端，定长 8 字节）。
2. 提供**显式** `I64`→`U64` 转换原语，负值行为有界且 typed（不静默、不 trap 逃逸）。
3. 不破坏现有冻结序列化/字节输出（纯新增 intrinsic，不改既有 emission）。
4. 使 G6 pilot 的 re-encode 可表达，从而解锁 §8.5 的 save-path-failure / byte-stable repeat / batch 采样三组 corpus。

**Non-goals**

- 不引入隐式数值转换/拓宽（`I64`↔`U64` 保持显式，避免静默语义）。
- 不改变 `image-codec@1` 冻结接口戳记或 `E8016` 门。
- 不新增 `Bytes` 的可变写入或切片原语（只读读取足够本场景）。
- 不处理 `U64`→`I64` 反向转换（当前无需求，留待有实测需求再提）。

## Semantics

**A. `Bytes`→`U64` 读取（小端，定长 8 字节）**

- 候选签名：`sico.bytes.le_u64(bytes: Bytes, offset: U64) -> Result[U64, Text]`。
- 动态语义：取 `bytes[offset..offset+8]` 小端组装为 `U64`。
- 边界（fail-closed）：
  - `offset + 8 > len(bytes)` → `error("out-of-bounds")`（**不**用 fallback 值静默填充）。
  - `offset` 越界本身由 `U64` 无符号性排除负值。
- 效果：无（纯函数）；资源：O(1) 时间/空间。

**B. `I64`→`U64` 显式转换**

- 候选签名：`sico.conv.u64_from_i64(value: I64) -> Result[U64, Text]`。
- 动态语义：`value >= 0` → `ok(value as U64)`；`value < 0` → `error("negative")`（**不用 wraparound 静默回绕**）。
- 理由：`Bytes`→`U64` 已覆盖「从数据得到 U64」的主路径；`I64`→`U64` 仅作受控辅助，负值拒绝比回绕更不易隐藏 bug。
- 效果：无；资源：O(1)。

**错误面**：两者均返回 `Result`，错误值属 closed set（`out-of-bounds` / `negative`），纳入既有 `ScriptErrorCode::DomainError` 归因；不新增 trap 路径。

**为什么返回 `Result` 而非裸值**：与仓库「typed refusal over hidden fallback / silent trap」纪律一致（AGENTS §5）。裸 `U64` + 越界 trap 会引入新 trap 面；`Result` 让调用方显式处理。

## Syntax candidates

**方案 1（推荐）：内建 intrinsic 函数，挂在既有 namespace**

```
sico.bytes.le_u64(bytes, offset) -> Result[U64, Text]
sico.conv.u64_from_i64(value) -> Result[U64, Text]
```

- 规范格式：与 `sico.bytes.at`、`sico.text.concat` 等既有 intrinsic 同构；前端作为具名 intrinsic 调用解析，发射到 codegen 新增 `cgs_*` 臂。
- 优点：最小新增、复用既有 intrinsic 分发/预算机制、无新类型/无新运算符。

**方案 2：`U64` namespace 上的构造/转换方法**

```
U64.from_le_bytes(bytes, offset) -> Result[U64, Text]
U64.from_i64(value) -> Result[U64, Text]
```

- 优点：类型内聚（`U64` 的构造集中在 `U64.` 下）。
- 缺点：`U64.from_le_bytes` 把 `Bytes` 依赖带进 `U64` namespace，职责略混；与既有 `sico.bytes.*` 分裂。

**方案 3（拒绝）：泛化 `sico.bytes.at` 增加返回类型重载**

- `sico.bytes.at_u64(bytes, offset) -> U64`（裸值 + 越界 trap）。
- 拒绝原因：引入新 trap 面、调用方无法 typed 处理越界，违反 fail-closed 纪律。

## Positive and negative cases

**成功**

```
let header: Bytes = decode_png(data)?          # 16B LE u64 头
let w = sico.bytes.le_u64(header, 0)?          # ok(U64)
let h = sico.bytes.le_u64(header, 8)?          # ok(U64)
encode_png(pixels, w, h)?                       # 可表达
```

**失败（期望 typed 短诊断）**

- `le_u64(header, 9)`（9+8 > 16）→ `error("out-of-bounds")`；期望诊断钉 `sico.bytes.le_u64` 越界。
- `u64_from_i64(sico.bytes.at(b, 0, -1))`（fallback 触发得 -1）→ `error("negative")`；期望诊断钉负值。
- 畸形：`le_u64` 传非 `Bytes` / `u64_from_i64` 传非 `I64` → 前端 typed refusal（E200x 形状错）。

## AST and IR

- 语义 AST：两个新 intrinsic 调用节点（复用既有 intrinsic 调用形状，无新 AST 类型）。
- Sico IR：新增两条 intrinsic id（如 `bytes_le_u64`、`conv_u64_from_i64`），进入 intrinsic 表；verifier 校验参数形状（`Bytes,U64` / `I64`）与结果 `Result[U64,Text]`。
- 验证要求：参数计数/类型不匹配 → typed refusal（不静默近似）；越界/负值在运行期走 `Result::error`，不新增 trap。

## Component/WIT mapping

- 二者均为**纯 guest 内计算**，不跨 host 边界 ⇒ **无 WIT 改动、无 hostcall**、不进 `script-ui-v1`/任何 package 接口。
- 可表示性：canonical IR→Wasm 直接映射（`le_u64` = 8×`i32.load8_u` + 移位或 `i64.load` + `i64.or` 组合；`u64_from_i64` = 有符号性检查 + `i64`→`u64` 位保持）。
- 不支持情况：不暴露为 package 接口函数（避免 E8016 戳记问题）；仅作语言内建。

## AI evaluation

- 生成：模型能否在需从字节缓冲取数值时正确选用 `le_u64` 并处理 `Result`。
- 理解：能否区分 `I64`/`U64` 且不尝试隐式转换。
- 诊断：越界/负值用例能否给出可定位短诊断。
- 修复：把「`bytes.at` + 手工移位」的冗长写法重构为 `le_u64` 的能力。
- 实验：沿 STEP-0635 的 G6 pilot 语料（decode→re-encode）作 acceptance oracle。

## Compatibility

- 版本：纯新增 intrinsic，不改既有 intrinsic id/字节输出 ⇒ 既有冻结语料字节不变。
- 序列化：无新序列化形式。
- 包/接口：不动 `image-codec@1` 戳记；不新增 package 接口。
- 迁移：无破坏性；既有程序不受影响。

## Security and privacy

- 权限：无新权限（纯计算，无 hostcall、无 IO）。
- 信息泄漏：`le_u64` 只读调用方自有 `Bytes`；无跨边界读取。
- 资源限额：O(1)，受既有 fuel/内存预算约束，不引入无界循环/分配。
- 滥用风险：负值 `I64`→`U64` 拒绝（不回绕）避免符号混淆类 bug 被静默利用。

## Alternatives

- **隐式 `I64`→`U64` 拓宽**：拒绝——静默语义、负值回绕隐患，违反「typed refusal over hidden fallback」。
- **裸值 + 越界 trap**（方案 3）：拒绝——新增 trap 面，调用方无法 typed 处理。
- **把 `image-codec@1` 接口宽高改 `I64`**：拒绝——E8016 戳记门禁止（与冻结 `u64` 冲突），且把类型缺口推给包契约而非语言。
- **只做 `bytes`→`U64`、不做 `I64`→`U64`**：可选收窄——若 owner 认为主路径仅需 `le_u64`，可先落 A、B 延后。**倾向两件一起落**（G6 的 fallback 路径 `bytes.at(...,-1)` 仍产 `I64`，B 使其可受控转换）。

## Validation and acceptance criteria

- 会失败-先行差分测试：`le_u64` 越界、`u64_from_i64` 负值，先写期望 typed 拒绝的测试再实现。
- 逐字节等同：新增 intrinsic 不改变既有任何 `*_byte_exactly` pin 的输出。
- G6 pilot 解锁：`g6_converter_pilot` 在 re-encode 可表达后，save-path-failure / byte-stable repeat / batch 采样三组由 blocked 转 green，且 byte-stable repeat 真正产出字节恒等 PNG 文件。
- 正式接受门槛：owner 裁决本 RFC status → accepted；实现片另开 STEP，跑绿上述测试 + workspace clippy/fmt + `validate-step-0131.ps1`（若登记矩阵行）。

## Links

- 实测来源：[`STEP-0635`](../steps/STEP-0635-m24-g6-converter-pilot-png-only.md) §3.1（v0 语言缺口实测）。
- 受益项目：[`案例项目/gui-converter/converter.sico`](../../案例项目/gui-converter/converter.sico)（PRJ-0001；原 `pilots/g6-converter/`，依 owner 2026-11-19 治理政策迁为外部应用项目）。
- 相关：M24 计划 §8.5（转换器 fail-closed corpora）；`image-codec@1` 包契约（`guests/image-codec/`）；语义 `sico.bytes.at`（`crates/sico-runtime/src/.../lib.rs:4159`）。
- 纪律：AGENTS §5（前端支持面/typed refusal）、§10.2（合同先 RFC 冻结）。
