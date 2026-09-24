# STEP-0270: RFC-0047 acceptance registration and EC-4 census freeze

> - status: complete (RFC-0047 accepted by owner directive; EC-4 census frozen)
> - phase: M22 R0 outcome (ADR-0017 Option A)
> - started: 2026-09-24
> - completed: 2026-09-24
> - owners: autonomous-agent
> - evidence class: planning-only + internal-fixture (pure text measurement over frozen corpora; zero grammar or source changes)

## 1. Objective

owner 于 2026-09-24 会话指令「接受」，接受
[`RFC-0047`](../rfc/RFC-0047-record-types-v0.md)（record 类型最小封闭集：
声明 / 具名字面量 / 点字段访问 / `List[record]`，词法表零增长，
List 边界降级到 ADR-0016 并行数组）。接受后按 STEP-0269 §6 的既定顺序，
第一个实现前 STEP = **EC-4 双普查冻结**（纯测量、零语法改动），把
RFC-0047 承诺的两个实测消费者落成冻结数字，作为 R0 决议的验证基线。

本 STEP 同时完成 RFC README 状态登记（draft → accepted）与全仓指针同步。

## 2. EC-4 census — frozen numbers (schema `sico.m22.ec4.v0`)

测量工具：[`tools/census-ec4-records.ps1`](../../tools/census-ec4-records.ps1)
（`(?m)` 逐行 + `Singleline` 签名正则，函数签名跨行安全；参数计数为
括号深度感知逗号计数）。原始冻结输出：
[`docs/reports/m22-ec4-census-2026-09-24.json`](../reports/m22-ec4-census-2026-09-24.json)。

### Corpus A — self-host（SOA 授权缩减上限代理）

| 指标 | 冻结值 |
| --- | --- |
| 文件数 | 11 |
| 总行数 | 16,931（与 STEP-0265 §2 盘点一致 ✓） |
| list 仪式行（`sico.list.init/append/get/length`） | 360 |
| **仪式行占比** | **2.13%** |
| 函数总数 | 263 |
| ≥3 个 `List[` 形参的函数（SOA 表穿线） | 16（6.1%） |
| `List[` 形参出现总次数 | 145 |

解读：records 能把 SOA 表穿线收口到名义类型，但仪式行直接缩减的
上限约 2%——**R0 选 A 的依据从来不靠行数，靠的是「只重写一次 +
终局单一数据模型」**（STEP-0268）；仪式行占比只承诺授权表面收窄，
不承诺行数大降。`compiler_semantics.sico` 仪式行占比 8.8% 为单文件
最高，是 re-baseline 时第一个该重写的文件。

### Corpus B — application profile（M14 e2e，records 需求侧实测）

| 指标 | 冻结值 |
| --- | --- |
| 文件数 | 39 |
| 总行数 | 3,450 |
| `sico.map.put[...]` 调用点 | 16 |
| 其中字面量键调用点 | 14（87.5%） |
| 去重字面量键 | 14 |
| `Text` 键类型的 map 操作（put/empty/get/contains） | 24 |
| 函数总数 | 115 |
| ≥5 形参宽签名函数 | 5（4.3%） |

解读：应用语料里 map-as-struct 的全部字面量键只有 **14 个去重键**——
`Map[Text, T]` 在应用代码里实际承担的是伪 record 角色，且规模极小；
records 的 M14 消费者压力真实存在但当下用量小，验证 RFC-0047 把
`Map`/`Set` record 键值排除在 v0 之外的判断（需求侧没有倒逼扩大封闭集）。
5 个宽签名函数是记录类型字段收口的直接落点。

## 3. Changes

- `docs/rfc/RFC-0047-record-types-v0.md` 状态头：draft → accepted
  （owner directive 2026-09-24「接受」, same-day draft）；
- `docs/rfc/README.md` 0047 行：draft → accepted，注记接受指令与
  STEP-0270 冻结 gate；
- 新增 `tools/census-ec4-records.ps1`（EC-4 双普查，schema
  `sico.m22.ec4.v0`）；
- 新增冻结证据 `docs/reports/m22-ec4-census-2026-09-24.json`；
- 本记录 + steps 索引；STATUS / M22 计划头指针更新。

## 4. Validation

- 普查可重跑：`powershell -ExecutionPolicy Bypass -File tools/census-ec4-records.ps1`
  输出与冻结 JSON 一致（合计行 16,931 与 STEP-0265 盘点交叉核对通过）；
- `tools/validate-step-0124.ps1` 通过；`git diff --check` 通过；
- 未运行：编译/测试（纯文本测量 + 文档登记）。

## 5. Metrics

见 §2 两表。仪式行占比 2.13% 是「SOA 模拟行数缩减」的冻结上限；
应用侧 14 个去重字面量键是「map-as-struct 真实需求」的冻结实测。

## 6. Risks and follow-ups

- **接受后实现顺序**（ADR-0017 §Validation，重申）：Rust 内核全链实现
  （parser/HIR/semantics/IR/verifier/codegen/formatter）前，先冻结
  EC-1..EC-3 出口语料（语法/格式化幂等/typed 诊断）；W1 合并债照还——
  在途 WIP（`parser.sico` Map 参数面 +228 行 + `dump_nm_region_ir` 探针）
  在 W1/W2 动 parser.sico 前落地为独立 STEP 或吸收，本 STEP 未触碰它。
- **不预留实现 STEP 编号**：records 各实现 STEP 在 EC-1..EC-3 语料冻结
  后按下一空闲号登记。
- 与 M23 关系不变：RFC-0047 独立于 M23 四项，可并行；M23 开工盘点
  （四探针）不受影响。
- 下一步：EC-1 语法语料冻结（声明块 / 具名字面量 / 点访问的
  append-only 语料，RFC-0038 corpus 登记），或先偿还在途 WIP 的
  STEP 落地（两者不冲突）。
