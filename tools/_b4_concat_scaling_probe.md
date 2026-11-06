# B4 拆账：最小复现设计（文本拼接平方 vs JSON 成员访问平方）

目标：用**秒级**实验把「fuel 随组件大小超线性放大」的根因钉到"文本累加拼接"还是"JSON 成员重扫"，不烧 2h ledger。

## 已知数据（STEP-0555 §10.3）

| 站点 | IR 字节 | 最小 fuel |
|---|---|---|
| codegen.sico:cgs_add | 3,457 | 1.0e9…1.2e9 |
| parser.sico:fixed_literal_const_segment | 92,055 | >800e9（耗尽） |

fuel/IR字节：347k → >8.7M（下限）。**超线性确认**（线性本应只涨 26×）。

## 假设 A：文本累加拼接平方

`bytes_hex`（compiler.sico:301）等用 `set output = sico.text.concat(output, hex_byte(byte))` 逐字节累加。若 concat 每次重拷整个 output，则拼 L 字节串烧 O(L²)。

**判定量**：构造只含 1 个大 const_string 的最小源（让 bytes_hex/const_string 路径主导），测 IR 大小 vs fuel。若 fuel ∝ L²，假设 A 成立。

## 假设 B：JSON 成员重扫平方

`cgs_cell_slot`/`cgs_dispatcher_slot`/`cgs_cell_slots` 每次 `cgj_member(bytes, run, field)` 都从 JSON 线性扫。发射器 per-instruction 频繁查它们 ⇒ 指令数 × run 大小 = O(n²)。

**判定量**：构造指令数多但字符串少的源（让 cell_slot/dispatcher_slot 查询主导），测指令数 vs fuel。若 fuel ∝ n²，假设 B 成立。

## 最小复现构造

guest 源 = `HOST_CALL_SHAPE_PRELUDE` + 一个含 N 条指令的 run + `HOST_CALL_SHAPE_RUN`。

- **测 A**：N 条指令全是 `const_string`（大字面量）⇒ bytes_hex/文本主导。
- **测 B**：N 条指令全是 `checked_add`/`read_local`（触发 cell_slot 查询）⇒ JSON 重扫主导。

每个 N 用 fuel_bracket 测最小 fuel，看 fuel(N) 曲线是线性还是平方。

## 判定

- 若 A 主导：B4 = 累加缓冲改写（中等工作量）。
- 若 B 主导：B4 = cell_slot/dispatcher_slot 索引化（B2 同类，风险低）。
- 若两者都平方：先改杠杆大的。

## 状态

未实施——等 owner 确认走这条最小复现，或直接选一个假设开工。
