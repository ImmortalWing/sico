# M22 燃料墙敏感度测量工件（STEP-0600）

本目录固化 STEP-0600 的测量读数。测量**只做观测**：未改任何产品限额或默认值（`RunnerLimits` 默认值与 `runner/sico-runner/src/lib.rs` 的限额段均未动），fuel 上限经**每次运行的限额参数**传入。

## 文件与读法

| 文件 | 内容 |
|---|---|
| `fuel-sensitivity-groups.csv` | 参与测量的 62 个 guest 函数组（`source:group` 逗号列表） |
| `fuel-sensitivity-sites-per-group.csv` | 每组站点数（`group=count`），合计 **331** 站点 |
| `fuel-sensitivity-1p6t.log` | **2×（fuel=1.6e12）全量扫描**逐行结果（62 组，EXIT=0） |
| `fuel-sensitivity-3p2t-subset.log` | **4×（fuel=3.2e12）子集**逐行结果（8 个最小未翻转闭包，EXIT=0，耗时 1113 s） |
| `fuel-sensitivity-w4-refusals.log` | W4 parity 剩余 3 个 `scope-nesting-limit` 用例 × 3 个 fuel 档（EXIT=0，2.45 s） |

逐行字段：`SENS_FUEL <group> fuel=<上限> outcome=<component|fuel-exhausted> component_bytes=<字节> component_equal=<yes|-> wall_ms=<毫秒> source_bytes=<字节>`；
W4 日志字段：`SENS_CORPUS <文件> fuel=<上限> outcome=<refused:错误码|...> wall_ms=<毫秒> source_bytes=<字节>`。

## 口径

- **被测路径**：真实 ledger 路径（`compile_guest` → guest 自举编译器 → 产出 Component），与 Rust oracle 逐字节比较；`component_equal=yes` 即该站点**转绿**。
- **转绿定义**：在给定 fuel 上限下**成功产出**与 Rust oracle **逐字节相等**的 Component（`component_bytes>0` 且 `component_equal=yes`）。
- **覆盖范围**：62 个函数组 / 331 个站点（`fuel-sensitivity-groups.csv` 与 `-sites-per-group.csv` 给出全集与每组站点数）。
- **M（实测所需 fuel）**：runner 公共 API **不暴露 fuel 读数**（STEP-0572 §5 记录的缺口），故 M 由 `wall_ms × ~1.9e10 fuel/s` **折算**，为量级估计（翻转组：M ≈ wall×rate；耗尽组：M > 该上限）。
- **成本**：~50–75 s/组（1.6T）、~150–190 s/组（3.2T）；速率 ~1.7–2.1e10 fuel/s，与闭包规模无关。

## 关键结论（详见 STEP-0600）

- **2×（1.6T）全量**：62 组中仅 **3 组翻转**（源 17–21 KB 小闭包），59 组耗尽；站点级 **5/331 = 1.51% 转绿**、**326/331 = 98.49% 仍耗尽**；**0 组件失配 / 0 trap / 0 refusal**。
- **4×（3.2T）子集**：8 组中 4 组翻转（M ≈ 1.6–2.8T）、4 组仍耗尽（M > 3.2T）。**未做 4× 全量**（外推 ~3.1 h），该结论仅在该子集范围内成立。
- **W4 剩余 3 项不是燃料墙**：在最低档 800e9 即毫秒级被 `E-SH-IR-FUNCTION-COUNT` / `E-SH-IR-EXPRESSION` 拒绝，**提高 fuel 不改变结果**（订正此前登记的「scope-nesting-limit 燃料墙」口径）。

## 复跑

```
cargo test --offline --manifest-path runner/sico-runner/Cargo.toml --test m22_fuel_sensitivity -- --test-threads=1 --ignored
```

（测量为长耗时测试，默认 `#[ignore]`；单机独占运行约 70 分钟（1.6T 全量）。）
