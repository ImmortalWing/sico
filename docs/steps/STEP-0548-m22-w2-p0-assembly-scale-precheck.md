# STEP-0548 m22-w2 P0 **装配尺度预检片**：分片 `list.append` + 一次 `join` 在 42.5 M fuel / 38 ms 内产出 1 MiB ⇒ **D1 绿**

| 项 | 值 |
|---|---|
| 片型 | P0 尺度预检：纯测量 + 4 条常驻钉。`selfhost/`、`crates/`、`wit/` **零改动**；唯一代码改动是 `runner/sico-runner/tests/m22_scale_probe.rs` 新增策略 S4 与其钉 |
| 决策门 | **D1 = 绿**：装配墙（输出字节自身拼装）有 shipped 线性解法，P1 获准开工 |
| 不翻门 | 门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用 —— 全部**维持**；本片不改变任何门的结论 |
| 证据等级 | `internal-fixture`：本机 2026-10-26，Windows x64，debug 测试构建，`sico_runner` 限额定为合格跑值 |

## 1. 要回答的问题（D1）

STEP-0547 判定的墙是「发射端用 hex 文本左折叠装配」：`selfhost/codegen.sico` 有 512 处 `sico.text.concat` 引用（461 处 `set X = sico.text.concat(X, …)` 自左折叠）。STEP-0516 的三条策略里只有 S3（一次 `concat`，两个大操作数来自 stdin）是线性的，而发射端只能做 S1/S2 形状。

P1 拟把累加器换成「分片 `List[Text]` + 一次 `sico.text.join`」。这是**假设**，必须先用测量证伪/证实，否则又是补丁叠补丁。D1 的问题因此是：

> 用后端**已经 ship** 的 `sico.list.append`（amortized 就地补位）+ `sico.text.join`（先总长、再一次拷贝），1 MiB 输出能否在合格限额内完成，且曲线是否线性？

## 2. 方法

- 在 `runner/sico-runner/tests/m22_scale_probe.rs` 增加 **S4**：`reps` 次 `sico.list.append(chunks, chunk)` 收集 16 B 分片，末尾一次 `sico.text.length(sico.text.join(chunks, ""))`。
- 与既有策略同阶梯（`LADDER` 换算成 4 KiB…1 MiB 输出）、同限额（fuel 800e9 / timeout 120 s / runner 内存 2560 MiB / hostcall 128 << 20）、同计量法（`min_fuel` 二分 = 所需**最小上限**，不是耗量读数；wall 机器相关只作参考）。
- 失败按类型分类：`arena-exhausted`（Wasm `unreachable` trap）≠ `fuel-exhausted`（typed）≠ `memory-limit` ≠ `timeout`。

## 3. 实测（2026-10-26，106 s / 103 s 两轮复现）

| 策略 | 128 KiB | 256 KiB | 512 KiB | 1 MiB | 2 MiB | 4 MiB | growth 256KiB→1MiB | 判读 |
|---|---|---|---|---|---|---|---|---|
| S1 增量 `concat` | 541,471,259 / 264 ms | **arena-exhausted** | — | — | — | — | 不可测 | 撞 arena |
| S2 `push_chunk` | 274,125,457 / 95 ms | 1,085,117,073 / 413 ms | **arena-exhausted** | — | — | — | 不可测 | 撞 arena |
| **S4 `list.append`+`join`** | 5,314,763 / 29 ms | **10,623,362** / 28 ms | 21,240,386 | **42,474,275** / 38 ms | 未测 | 未测 | **1.21×–1.31×** | **feasible** |
| S3 一次 `concat`（理论下限） | 3,015,169 | 6,029,825 | 12,059,137 | 24,117,785 / 59 ms | 48,235,033 | 96,469,529 | 1.55× | feasible |

（cell 内为 `min_fuel / wall_ms`；失败点只记失败类型。证据：`target/tmp/step0548/assembly-measurement.txt`，与 `target/tmp/step0516/scale-measurement.txt` 同源。）

## 4. 判读

1. **D1 = 绿。** 1 MiB 输出在 **42,474,275 fuel（限额的 0.0053%）、38 ms** 内完成，两个阶梯点 growth 1.21× / 1.31×，远低于 D6 的 6× 阈值。
2. **成本是严格线性的**：S4 每个输出字节 40.5 fuel（10.6 M / 256 KiB 与 42.5 M / 1 MiB 同值），256 KiB→1 MiB×4 只涨 4.0×。
3. **相对 S3 只贵 1.76×**（42.5 M vs 24.1 M）：分片装配的额外开销接近理论下限，不存在隐藏的二次项。
4. **相对现有形状便宜两个数量级**：同样 256 KiB，S2 要 1,085,117,073，S4 只要 10,623,362（**102×**）；而 S1 在 128 KiB 就要 541 M 并在 256 KiB 撞 arena。
5. 因此 P1 不需要新语言特性、不需要 RFC、不需要上调任何限额：`sico.list.append`（`crates/sico-codegen-wasm/src/stdlib.rs` 的 `emit_list_append`，就地补位 + 几何扩容）与 `emit_join`（总长扫描 + 一次拷贝）都已在后端里，`selfhost/codegen.sico` 目前只有 3 处使用 `list.append`、0 处使用 `text.join`。

## 5. 边界（本片**没有**证明的事）

- **只关了 A 墙（输出字节自拼装），没关 B 墙（每指令 IR 重扫）。** 140 个 `FuelExhausted` 站点的 artifact 只有 1–5 KB，其成本来自 `cgs_value_slot` → `cgj_array_item` 对 IR 的逐指令 O(index) 重扫（STEP-0544/0545/0546），与本片测的装配无关。
- gate 3 要求「guest 产出 ~1 MiB Component」，它**同时**需要 A 墙与 B 墙的解法：A 墙现在有measured 解法，B 墙仍只到 n≈256 条指令（STEP-0546）。
- 所以本片**不翻任何门**，也不改变 gate 2/3 的结论；它只解除 P1 的前提风险。

## 6. 常驻钉（4 条，已进测试）

1. S4 必须到达 1 MiB 且 `verdict == ok`；
2. S4 的 1 MiB `min_fuel` 必须 < 限额 1%（实测 0.0053%）；
3. S4 的 256 KiB→1 MiB growth 必须读作 `feasible`（≤6×，实测 1.21–1.31×）；
4. 同样 256 KiB，S4 必须比 S2 便宜 ≥10×（实测 102×）。

## 7. 验证

```
$env:RUSTUP_TOOLCHAIN='1.98.0-x86_64-pc-windows-gnu'
cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test m22_scale_probe -- --nocapture
```

- 首轮（只加 S4、未加钉）：`1 passed`，105.52 s；次轮（加钉）：`1 passed`，102.95 s，exit 0。
- 与 STEP-0516 的既有钉并存：S1 仍在 256 KiB 撞 arena、S3 仍到 1 MiB、内存地板仍等于声明线性内存。

## 8. 诚实残差

- 固定 16 B 分片、无分隔符 join；其它分片尺寸/分隔符未测（不认为它们会改变量级，但**没测就不断言**）。
- `min_fuel` 是「所需最小上限」的上界，不是耗量读数；Runtime ABI 仍无 fuel 读数出口。
- wall time 机器相关（两轮 S4 1 MiB：34 ms / 38 ms），判读只依赖 fuel 与 growth。
- debug 构建；release 只会更快，未被测试。
- 本片未测 >1 MiB 的 S4 输出（gate 3 的 ~2 MiB hex 文本按 40.5 fuel/B 外推约 81 M fuel = 限额的 0.01%，**外推不是测量**）。

## 9. 下一步（P1）

1. 把 `selfhost/codegen.sico` 的六个累加器（`cbsh_out` 37、`cccsh_out` 32、`clgh_out` 25、`cgpr_text` 25、`ccmh_out` 23、`cfh_out` 19 处自左折叠）改为「分片 `List[Text]` + 边界处一次 `text.join("")`」。
2. 每改一段必须保持既有 36 个 `ir-equal+component-equal` 站点逐字节不变（否则回退）。
3. 之后进 P2：把夹具 `MATCH_CLOSURE_FUNCTION_LIMIT` / `MATCH_CLOSURE_BYTE_LIMIT` 放开到 128 fn / 1 MiB 并一次性重跑 396 站点，得到新直方图，用 D2 判断残差是不是结构性的。
