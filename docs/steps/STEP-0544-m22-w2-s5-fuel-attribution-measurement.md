# STEP-0544 · M22 W2 S5 · fuel 归因测量片（140 个 `FuelExhausted` 站点的成本归因）

> **本片性质** = **测量片**：`crates/` **零改动**、`selfhost/` **零改动**；只动 `runner/sico-runner/tests/selfhost_compiler.rs`（**+436 行，纯新增**）与文档。
> ⇒ **不推进任何出口门**、**不上调 fuel 限额**、**不加尺寸门**、**不改 runner 公共 API**、**M22 整体仍 NO-GO**。
> **对象** = 账本里 **140** 个 `component-other:FuelExhausted` 站点（**800e9 主 fuel** 预算下耗尽）——**它们到底由哪条路径烧掉 fuel**。
> **落点 = 隔离 worktree** `E:\github\sico-m22-step0544`（分支 `codex/m22-w2-step0544` @ `50c6229`）：主 worktree 当时有**并发写入方**在改 `selfhost/codegen.sico` 与账本 ⇒ 本片必须让**证据树 = 提交树**，故另开隔离树（主树的改动**未纳入**本片）。

## 0. 入口门槛

| 门槛 | 状态 | 证据 |
|---|---|---|
| 有**可执行的**耗尽现象 | ✅ | 账本 `selfhost/match-component-coverage-v1.json`：**140 / 396** 站点 verdict = `component-other:FuelExhausted`（**800e9 主 fuel**） |
| 计量口径**不改公共面** | ✅ | `RunOutcome::FuelExhausted` 只判 `store.get_fuel() == 0`（`runner/sico-runner/src/lib.rs:3552`），**不带剩余量读数** ⇒ 只能**二分 fuel 限额**反推；而限额只是 `RunnerLimits.fuel` 一个数（`lib.rs:2072` / `:2282` 的 `store.set_fuel`）⇒ **零 `crates/` 改动、零 runner 公共 API 改动** |
| 合成形状**先跑通 oracle** | ✅ | X 族由 `match_shape_module()` 派生、Y 族由 `rust_module(match_fixture(...))` 派生 ⇒ 都是**已被 oracle 生产输出接受**的形状 ⇒ 测的是「发射」不是「拒绝」 |
| 有**阳性对照** | ✅ | 每族第一点**必须**测到阈值（`FuelProbe::Measured`），否则断言失败 ⇒ 探针测的是拒绝路径时**当场红** |
| `crates/` / `selfhost/` 零改动 | ✅ | `git diff --stat` = `runner/sico-runner/tests/selfhost_compiler.rs | 436 +++++`（**代码侧唯一文件**；另有 7 份**文档**同步） |
| 测量**不进固件、也不拖慢 CI** | ✅ | 测量测试 `#[ignore]`；常驻的只有一条**搜索算法钉**（`the_fuel_bracket_brackets_the_threshold_where_the_guest_flips`） |

## 1. 方法

### 1.1 计量口径（二分 fuel 限额）

`guest_emission` 的副本 `guest_emission_with_fuel(ir, fuel)` 只把 `fuel` 参数化，其余（组件缓存、池、limits 的其余三项）**逐字照抄** ⇒ 被测行为与固件**同口径**。

`fuel_bracket` 先按**几何阶梯**（×2）从 seed 往上找第一个「能出组件」的预算，再在最后一个八度内**二分**，直到区间宽度 ≤ 上端的 1/4 ⇒ 真实消耗落在 `(low, high]` 内，区间宽度 ≤ 25%。

三类结果**分开记**：

| 结果 | 含义 | 是否硬失败 |
|---|---|---|
| `Measured { low, high }` | 出组件，阈值在 `(low, high]` | 否（这是正常结果） |
| `Exhausted` | 800e9 上限仍不够 | **否**——这正是账本里 140 个站点的现象，是**被测对象** |
| `Trapped { fuel, reason }` | guest **trap**（`unreachable`）而不是回答 | 否——**第三类结果**，记为形状上限 |
| `Refusal` | guest typed 拒绝 | **是**——探针测到拒绝路径 ⇒ 立即红 |

> **纪律**：只有**拒绝对照**才是硬失败；耗尽与 trap 都是**测量结果**。

### 1.2 两族合成 IR

| 族 | 变化量 | 保持不变 | 隔离的是 |
|---|---|---|---|
| **X 族** | module 的**指令条数 n** 递增（8 → 128，每条 = 一条 `ConstU64`，**指令数精确**） | 其余全部 | **IR 遍历路径**（指令数依赖）vs **hex 拼装路径**（输出字节数依赖） |
| **Y 族** | **字面量长度 L** 递增（1 KiB → 32 KiB，在 canonical IR 层打补丁，避开源码行长限制） | **指令条数恒定** | **hex 拼装 / 字面量编码路径**（字节数依赖） |

X 族的关键性质：**组件字节数只随 n 线性增长**（1407 → 4596 B），**IR 字节数也随 n 线性增长**（7871 → 58855 B）⇒ 若 fuel 随 n 超线性增长，**就不能**用「输出更大」解释 ⇒ 归因必然落在**按指令数计**的路径上。

### 1.3 判读口径：区间可满足性（不是最小二乘）

测量给的是**区间** `(low, high]` 而不是点值。候选模型 `f(n)` 与测量**相容**，当且仅当存在**同一组**参数使 `low_n < f(n) ≤ high_n` 对**每个**测点成立。

因为每条约束对线性参数都是线性的，可行集 = 「固定形状参数，扫描另一参数，取 `a` 的区间交集」⇒ **交集为空 = 模型被测量排除**。这比「最小二乘拟合差」是**更强的陈述**。

## 2. 实测结果

### 2.1 X 族：指令数递增（8 点，全部测到阈值）

| n | IR 字节 | 组件字节 | fuel 区间 `(low, high]` | 搜索墙钟 | **单次工作墙钟** |
|---|---|---|---|---|---|
| 8 | 7 871 | 1 407 | (5.12e9, 6.144e9] | 1 948 ms | **265 ms** |
| 16 | 11 255 | 1 599 | (1.2288e10, 1.536e10] | 3 554 ms | **838 ms** |
| 32 | 18 039 | 2 005 | (3.072e10, 3.84e10] | 10 878 ms | **2 136 ms** |
| 48 | 24 823 | 2 391 | (5.76e10, 7.68e10] | 15 069 ms | **4 399 ms** |
| 64 | 31 607 | 2 777 | (1.152e11, 1.536e11] | 27 589 ms | **7 579 ms** |
| 80 | 38 391 | 3 225 | (1.536e11, 1.92e11] | 58 478 ms | **12 471 ms** |
| 96 | 45 175 | 3 673 | (2.4e11, 2.88e11] | 81 804 ms | **18 023 ms** |
| 128 | 58 855 | 4 596 | (4.32e11, 5.76e11] | 114 109 ms | **33 969 ms** |

（「单次工作墙钟」= 在**测到的阈值预算**上跑**一次**的墙钟 ⇒ 跨点可比；「搜索墙钟」还含阶梯与二分的全部探针，**不可**当成本列读。上表是**最终那一轮**（= 将提交的字节，harness sha256 `1db659c6…`）的实测值。**fuel 两列是确定性的**：**四轮独立运行逐项一致**；**墙钟两列随机器负载浮动**——最后一轮是在整轮电池**同时**跑的时候测的 ⇒ 绝对值是**上界**，只用于**比值**判读。）

### 2.2 Y 族：字面量长度递增（7 点，5 测到阈值 + 2 trap）

| L | IR 字节 | 组件字节 | fuel 区间 `(low, high]` | 搜索墙钟 | 单次工作墙钟 |
|---|---|---|---|---|---|
| 1 024 | 3 204 | 2 070 | (6.4e8, 7.68e8] | 570 ms | **72 ms** |
| 2 048 | 4 228 | 3 094 | (7.68e8, 9.6e8] | 430 ms | **92 ms** |
| 4 096 | 6 276 | 5 142 | (1.2e9, 1.44e9] | 535 ms | **105 ms** |
| 8 192 | 10 372 | 9 241 | (2.16e9, 2.88e9] | 887 ms | **257 ms** |
| 16 384 | 18 564 | 17 436 | (4.32e9, 5.76e9] | 2 092 ms | **586 ms** |
| 24 576 | 26 756 | — | **trap**（`unreachable`，预算 1.152e10） | 652 ms | — |
| 32 768 | 34 948 | — | **trap**（`unreachable`，预算 1.152e10） | 633 ms | — |

（同 §2.1：两列**墙钟**取自**最终那一轮**（2026-10-26 00:04:20 那次运行，guest sha256 `da0fec5f…` = 将提交的字节）；**fuel 两列确定性**、四轮逐项一致。两个 trap 点的墙钟是**探针撞到 trap 的那一档**的耗时，不是成功出组件的耗时。）

### 2.3 模型相容性裁决

| 模型 | X 族（指令数 n） | Y 族（字面量 L） |
|---|---|---|
| `a + b·n`（线性） | ❌ **排除** | ✅ **相容**（b ≈ 2.40e5，a ∈ (3.95e8, 4.59e8]） |
| `a + b·n²`（二参数二次） | ❌ **排除** | ❌ **排除** |
| `a + b·n³`（三次） | ❌ **排除** | ❌ **排除** |
| `a · n^p`（纯幂，p ∈ [0.5, 4.0] 连续扫描） | ❌ **排除**（**没有任何 p 相容**） | — |
| `a + b·n + c·n²`（三参数） | ✅ **相容**（**302 个可行 (b, c) 格**；b ∈ [9.26e7, 4.73e8]，c ∈ [2.28e7, 2.82e7]） | — |

- X 族的**有效纯幂指数**（对 `fuel_high` 的 log2 拟合）= **1.615**，最大残差 **0.361 个八度** ⇒ 曲线**不是**纯幂律，而是**带常数项的二次主导**。
- X 族**工作墙钟**的有效纯幂指数 = **1.720**（与 fuel 同阶 ⇒ 墙钟由同一路径决定）。
- Y 族的**有效纯幂指数**（fuel）= **0.740**、工作墙钟 = **0.753**；**上段实测比值**：每翻倍字面量 ⇒ fuel **×2.0 / ×2.0**，工作墙钟 **×2.45 / ×2.28** ⇒ **时间比 fuel 增长更快**，但**未达到** O(L²)。

### 2.4 外推（按 X 族实测曲线）

- **800e9 主 fuel 预算在 n ≈ 167–178 条指令处触顶**（三参数可行域上的区间）。
- 账本里那些耗尽站点的闭包是 **193–282 KB** ⇒ 按 X 族实测的每指令 IR 体量（≈ 423 B）折算，**n 在数百量级** ⇒ **远超** 167–178 ⇒ 与「这些站点在 800e9 下耗尽」的观察**一致**。
- **若修复去掉二次项**（即把「每个被发射值都重扫一遍指令数组」改成 O(1)），剩下的线性部分在 **n = 700** 处为 **6.8e10 – 3.3e11 fuel**（**2.4×–12×** 低于上限），在 **n = 1000** 处为 **9.6e10 – 4.7e11 fuel**（**1.7×–8×** 低于上限）⇒ **140 个耗尽站点有理由预期回绿**。

> ⚠️ 上面这一条是**预测**，不是结果 ⇒ **必须由修复片实测**（见 §7）。

## 3. 归因结论

### 3.1 140 个站点的 fuel 墙 = **IR 遍历路径**（B 路），但它是**二次**不是三次

1. **A 路（hex 拼装 / 输出字节数）被排除为 fuel 墙**：Y 族的 fuel **与字面量长度线性相容**、二次与三次被排除 ⇒ 随字节数增长的那部分代价**记在时间上，不记在 fuel 上**；X 族里「输出更大」也解释不了超线性增长（组件字节只线性增长）⇒ **A 路不是这 140 个站点的 fuel 墙**。
2. **B 路（IR 遍历 / 按指令数计）被测量确认**：X 族的 fuel 随**指令数**超线性增长，三参数模型 `a + b·n + c·n²` 相容且**二次项主导**（n = 128 时二次项 ≈ **3.7e11–4.6e11**，占实测 5.76e11 的 **86%–97%**（可行域枚举，302 格））。
3. **设计稿的 O(n³) 读法被本片排除**：三次模型与全部 8 个区间**无公共解**（区间可满足性扫描 ⇒ 交集为空）。单对点手算复核（n = 32 / 64）：三次系数被逼进 `b ∈ (3.35e5, 5.36e5]` ⇒ 那在 n = 128 处要求 `(7.30e11, 1.14e12]` fuel，而实测是 `(4.32e11, 5.76e11]` ⇒ **两区间不相交**（最近处差 **≈ 1.27×**；换 n = 8 / 16 一对复核则 ≈ 6.2×）⇒ 排除**成立，但不宽裕**。
4. **代码级对应**：`selfhost/codegen.sico:3793` 对**每条指令**调一次 `cgs_value_slot`；`cgs_value_slot`（`:1948-2005`）内部经 `cgj_array_item`（`:846-864`）**从数组起点逐元素重扫** ⇒ **单次调用 O(index)、总量 O(n²)** ⇒ **与实测的二次项一致**。设计稿把它读成 O(n³)，是**多算了一个幂次**（`cgj_value_end` 扫的是**单个元素的字节**，不是整条数组）。
5. ⇒ **修复方向不变、量级要改**：把「逐值重扫」改成 **O(1)（游标推进 / 前缀和 / 随发射走计数器）** ⇒ 预期把二次项消掉 ⇒ 140 个站点有理由回绿（预测，见 §2.4）。

### 3.2 A 路是**时间墙**，且在 24 KiB 字面量处**guest 直接死掉**

- Y 族上段：每翻倍字面量，**fuel ×2.0** 而**工作墙钟 ×2.45 / ×2.28** ⇒ **时间比 fuel 增长更快** ⇒ A 路的代价主要落在**时间**上（对 1 MiB 级整文件自举才是致命的；对 140 个站点不是）。
- **新披露的硬上限**：字面量 **≥ 24 576 B** 时 guest **trap**（wasm `unreachable`），**16 384 B 仍正常出组件** ⇒ guest 在这个规模上**不是变慢，是死掉**。**原因本片未定位**（候选：guest 侧 arena / 字面量编码的中间缓冲；**均未验证**）⇒ **登记为 A 路的新缺口**，由后续片定位。

### 3.3 本片不改变任何出口门

| 门 | 本片裁决 | 理由 |
|---|---|---|
| 1 / 5 / 6 | **不变（GO）** | 本片零代码改动（只加测量测试） |
| 2 | **仍 PARTIAL** | 组件级全等仍是 **32 / 396**；140 个站点**仍未闭合**（只多了归因，不是闭合） |
| 3 | **仍 NO-GO** | guest 仍未产出组件 B |
| 4 | **不得沿用 GO** | `B(S)` 仍未实测 |
| 7 | **不得给完成信用** | 语料覆盖仍 0 条、整文件自举未关闭 |

**M22 整体仍为 NO-GO。**

## 4. 交付物

1. **新建** `docs/steps/STEP-0544-m22-w2-s5-fuel-attribution-measurement.md`（本文件）。
2. **测量块**：`runner/sico-runner/tests/selfhost_compiler.rs` **+436 行**（纯新增，文件末尾一个区块）=
   - `STEP0544_FUEL_CEILING`（**800e9**，与固件同值）、`GuestFuelEmission`（含 `Trap`）、`guest_emission_with_fuel`、
   - `fuel_bracket`（阶梯 + 二分）、`the_fuel_bracket_brackets_the_threshold_where_the_guest_flips`（**常驻钉**）、
   - `x_family_ir` / `y_family_ir`（两族合成 IR）、`FuelProbe` / `FuelMeasurement` / `measure_fuel` / `step0544_sizes` / `step0544_verdict` / `step0544_flush`、
   - `the_fuel_attribution_is_measured_before_any_repair`（**`#[ignore]`** 的测量测试；档位可用 `SICO_STEP0544_X_SIZES` / `SICO_STEP0544_Y_SIZES` 覆盖）。
3. **支持面登记表** `docs/reports/m22-support-surface-match.md`：把「**归因待实测**」改写为**实测结论**（并新增 A 路的 trap 上限缺口）。
4. **同步** `docs/STATUS.md` / `docs/handoff-m22.md` / `docs/plans/M22-compiler-self-host.md` / `docs/ROADMAP.md` / `docs/steps/README.md`。
5. **测量工件**（`target/tmp/`，**不入库**）：`fuel-attribution.txt`（逐点表格）、`fit-analysis-v2.py`（区间可满足性分析）、`measure-run.log`、`chain-run.log`。

> **测量工件不入库的理由**：它由**探针**在**特定树**上产出，数值随树变化 ⇒ 归入 `target/tmp/` 与既有测量片一致（STEP-0516 / STEP-0531 同款做法）；**文档里已逐点登记原始数字**，审计不依赖被删掉的文件。

## 5. 独立 CI

- **STEP-0542（提交 `ce271b2`）的 run #53 = completed / success ✅**（run id `40197442833`，job id `121181236315`，`windows-gnu`，13:24:13Z → 14:30:56Z = **1h06m43s**）⇒ **STEP-0542 的独立 CI 裁决已回填**。
- **STEP-0543（提交 `50c6229`）的 run #54 = completed / success ✅**（run id `40197541303`，job id `121181529570`，`windows-gnu`，13:25:30Z → 14:40:06Z = **1h14m36s**）⇒ **STEP-0543 的独立 CI 裁决已回填**（门 6 出口审计片因此**有独立 CI 绿**）。
- 本片推送后触发下一轮独立 CI，**裁决由下一片回填**。

> **口径纪律**：文档里的 CI 结论**只来自 GitHub API 的实测值**，不是抄写。

## 6. 验证

### 6.1 第 1 轮电池红（本片自己的钉）—— 诚实登记

- 第 1 轮整轮电池在**第 5 拍 `ci-one-key` 红**（**2632s**，exit 1）；失败子步骤 = `test (runner suite)`，
  其余子步骤（fmt / clippy / build / test (runner core) / test (workspace) / module boundaries /
  comment discipline / planning contract / 应用矩阵 / 跨主机矩阵 / whitespace）**全绿**。
- 根因 = **本片自己的常驻钉** `the_fuel_bracket_brackets_the_threshold_where_the_guest_flips`：
  它断言「**成本高于上限**时二分不出区间」，但阈值被写成 `9_000_000_000`，而**上限是 `800_000_000_000`**
  ⇒ 阈值**低于**上限 ⇒ 区间**找得到** ⇒ **断言语义写反**（写的是「阈值高于上限」，实际是「阈值低于上限」）。
- **为什么此前没被抓到** = 测量测试是 `#[ignore]` 的（**不进固件电池**），而 `cargo clippy` **不执行测试**
  ⇒ 这条钉从写完到第 1 轮电池之间**从未被执行过** ⇒ **电池第一次执行它就红了**
  ⇒ 这正是「改了 `runner/` 必须整轮重跑电池」这条纪律的**实际价值**：钉绿 ≠ 钉被跑过。
- 修法 = 阈值改成 `fuel > STEP0544_FUEL_CEILING`（**自解释**：阈值必须**高于**上限，而不是「一个大数」）。
- 修完后**测量重跑**（第 4 轮；**fuel 区间与前三轮逐项一致**）⇒ **被测字节 = 将提交的字节**
  （harness sha256 `1db659c6…`；guest sha256 `da0fec5f…`）。
- **电池状态重置后整轮重跑**：`runner/` 变了 ⇒ 源码指纹变了 ⇒ 旧绿灯**不可复用**（指纹守卫的作用）
  ⇒ 第 1 轮那 4 拍绿灯**没有**被继承，第 2 轮从第 1 拍重跑。

### 6.2 电池表

| # | 步骤 | 耗时 | exit | 命令 |
|---|---|---|---|---|
| 1 | `fmt (workspace)` | 3s | **0** | `cargo fmt --all -- --check` |
| 2 | `fmt (runner)` | 2s | **0** | `cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check` |
| 3 | `clippy (workspace)` | 2s | **0** | `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` |
| 4 | `clippy (runner)` | 5s | **0** | `cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings` |
| 5 | `ci-one-key` | 2039s | **0** | `powershell -File tools/run-ci.ps1` |
| 6 | `validate-0245` | 1944s | **0** | `powershell -File tools/validate-step-0245.ps1` |
| 7 | `validate-0262` | 1718s | **0** | `powershell -File tools/validate-step-0262.ps1` |
| 8 | `probe-frontend-frontier` | 506s | **0** | `powershell -File tools/probe-m22-frontend-frontier.ps1` |
| 9 | `probe-ir-plan-frontier` | 28s | **0** | `powershell -File tools/probe-m22-ir-plan-frontier.ps1` |
| 10 | `comment-discipline` | 0s | **0** | `python tools/validate-selfhost-comment-discipline.py` |
| 11 | `match-site-census` | 0s | **0** | `python tools/census-m22-match-sites.py --check` |
| 12 | `match-site-coverage` | 1729s | **0** | `powershell -File tools/probe-m22-match-coverage.ps1` |
| 13 | `emitter-vocabulary-census` | 51s | **0** | `python tools/census-m22-emitter-vocabulary.py --check` |
| 14 | `firmware-selfhost` | 1600s | **0** | `cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler` |
| 15 | `git-diff-check` | 0s | **0** | `git diff --check` |

**BATTERY_GREEN 15/15**（源码指纹 `93f47afb9a306224`）

> 整轮电池 = `tools/validate-m22-s5.ps1` 的 **15 拍**，由 `target/tmp/step0544/battery/resumable-battery.ps1`（**未入库的包装器**）逐拍执行 —— **步骤与命令逐字等同**，只多了「每拍落盘 + 指纹守卫 + 可续跑」。指纹 = 受保护目录（`selfhost/` / `crates/` / `runner/` / `tools/` / `wit/` / `semantic-cases/` + workspace manifests）的内容摘要 ⇒ **文档改动不影响续跑**，而**代码改动会使旧绿灯失效**。

### 6.3 第 2 轮的两次宿主中断（诚实登记：墙钟被切开，指纹未变）

第 2 轮电池**不是一次跑完**的，途中被宿主打断过两次，根因都已定位：

1. **宿主正常关机**（2026-10-26 01:07，事件 1074 由 `StartMenuExperienceHost.exe` 发起，事件 13 记录关机）⇒ 电池实例随之消失；11:22 宿主重启（事件 12）⇒ 看门狗按**指纹**续跑（当时 11 拍已绿）⇒ **已绿的拍不被重跑**。
2. **看门狗计划任务把电池实例一起杀掉** —— 续跑实例在 11:50 / 12:21 / 12:52 三次无预告消失。根因 = 看门狗任务的设置含 `StopOnIdleEnd=true` + `StopIfGoingOnBatteries=true` + `DisallowStartIfOnBatteries=true` ⇒ 系统一旦判定「不再空闲」或「改用电池供电」，Task Scheduler 就**停掉该任务**，而任务启动的电池与 `cargo` 子进程**在同一进程树里** ⇒ 一并被杀。修法 = 重写任务 XML（`StopOnIdleEnd=false` / `StopIfGoingOnBatteries=false` / `DisallowStartIfOnBatteries=false` / `RestartOnIdle=true`），并在最后两拍期间由会话直接接管实例 ⇒ **15/15 绿**。
3. **另一次双实例重叠**（发生在第 2 轮开头）：一次性任务 `sico-m22-step0544-battery` 在 23:59 自己起了一个实例，**绕过了看门狗的进程扫描守卫**（守卫只看 `resumable-battery.ps1` 的命令行，不看是谁起的）⇒ 两个实例同时跑同一拍。处置 = 杀掉重复实例 + **删除该一次性任务**（保留带守卫的看门狗）⇒ 恢复单实例。

> **这对证据意味着什么**：表里每一行的耗时是**该拍自身**的耗时；第 2 轮的**墙钟**被中断切成多段（23:17 → 01:07 → 11:22 → 12:55），但**源码指纹全程是 `93f47afb…`（同一快照）** ⇒ 15 拍都跑在**同一份被测字节**上 ⇒ **绿灯可采信**。反之，任何**改过受保护目录**的续跑都会使指纹变化 ⇒ 旧绿灯立即作废（这正是第 1 轮红之后必须从第 1 拍重跑的原因）。

## 7. 诚实收窄

- 本片是**测量片**：它把 140 个站点的 fuel 墙**归因**到 IR 遍历路径，**没有**修复任何站点 ⇒ **组件轴仍是 32 / 396**。
- **二次项是二次，不是三次**：设计稿（`target/tmp/step0544/design.md`，**不入库**）的 O(n³) 读法**被本片排除**；该设计稿的落点（B1 游标推进 / B2 前缀和）**方向仍然成立**，但**收益要按二次项重估**。
- **系数是形状相关的**：X 族只有**一种**指令（`ConstU64`），每条 IR ≈ 423 B ⇒ 「n ≈ 167–178 触顶」这个数字**只对本形状族成立**；可迁移的是**指数**与**二次项主导**这个结构，**不是**系数本身。
- **外推是外推**：X 族的 IR 体量是 **7.9–58.9 KB**，而账本站点是 **193–282 KB** ⇒ 「n 在数百量级」是**按每指令字节数折算**的估计，不是逐站点实测。
- **修复效果是预测**：§2.4 的余量数字来自可行域包络，**必须由修复片实测**；本片**不得**据此宣称「140 个站点会回绿」。
- **A 路的时间墙未被完整刻画**：实测区间（≤ 16 KiB）里时间只呈**轻度超线性**（上段指数 ≈ 1.2–1.3）⇒ **O(L²) 未被证实**；1 MiB 级整文件自举的时间代价**仍需另一档尺度的测量**。
- **trap 的原因未定位**：字面量 ≥ 24 576 B 时 guest `unreachable`，**候选原因均未验证** ⇒ 只登记为**新缺口**。
- 证据等级仍是 `internal-fixture` ⇒ **不是** `clean-room-consumer` / `external-pilot` / `production`。
- **M22 整体仍 NO-GO**；门 3 是主欠。
