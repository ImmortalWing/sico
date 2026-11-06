# STEP-0562 · M22 W2 · fuel 墙根因仪器化剖析（测量片）

> **本片性质** = **测量片**：`crates/*` 零改动、`selfhost/parser.sico` 与 `selfhost/codegen.sico` **零改动（只读 guest 源码）**、`runner/sico-runner/tests/selfhost_compiler.rs` **零改动**（未被指定写）。
> **唯一代码产物** = 新工具 `tools/profile-m22-fuel-wall.ps1`（版本化剖析器，编排预构建二进制里**已存在**的 `#[ignore]` fuel 探针）。
> ⇒ **不推进任何出口门、不上调 fuel/内存限额、不加尺寸门、不改 runner 公共 API、M22 整体仍 NO-GO。**
> **对象** = 账本 `selfhost/match-component-coverage-v1.json` 里 **335** 个 `component-other:FuelExhausted` 站点（**全在 `parser.sico`**）把 fuel 烧在哪。
> **权威报告** = [`docs/reports/m22-fuel-wall-profile-2026-11-06.md`](../reports/m22-fuel-wall-profile-2026-11-06.md)。

## 0. 入口门槛

| 门槛 | 状态 | 证据 |
|---|---|---|
| 有可执行的耗尽现象 | ✅ | 账本 396 站点中 **335** verdict = `*FuelExhausted`，**100% 属 `parser.sico`**（`codegen.sico` 0 个） |
| 测量口径不改公共面 | ✅ | `FuelExhausted` 只判 `store.get_fuel()==0`、**无剩余量读数** ⇒ 只能**二分 fuel 上限**反推；工具的 `SICO_M22_PROBE_FUEL` 仅是**本次测量上限**，**不改**冻结 800e9 与任何 runner 限额 |
| 合成形状先跑通 oracle | ✅ | 合成 scaling 臂全部是被 oracle 接受的形状（`read_local` 链 / 多小函数模块），测的是**发射**不是拒绝 |
| 有阳性对照 | ✅ | `EMIT_SPLIT`/`EMIT_PADDED` 是 C-vs-D 判别对照；耗尽/trap 分开记，只有**拒绝对照**算硬失败 |
| `crates/`/`selfhost/` 零改动 | ✅ | 见 §5 `git status`：仅新增 `tools/profile-m22-fuel-wall.ps1` + 2 份文档 |
| 测量不进固件、不拖慢 CI | ✅ | 全部走 `#[ignore]` 探针，`--exact --ignored` 显式调用，不进默认 `cargo test` |

## 1. 环境阻塞（决定方法，如实登记）

| 项 | 实测 |
|---|---|
| 工具链 | `RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu`；**`gcc`/`cc` 不在 PATH** |
| **重编译被阻塞** | `cargo check --locked --offline --manifest-path runner/sico-runner/Cargo.toml --tests` → **exit 101**，`error occurred in cc-rs: failed to find tool "gcc.exe": program not found` |
| PowerShell / Python | PowerShell `5.1.26100.9818` 可用；**Python 不可用**（`python`/`python3`/`py` 均 not found）⇒ 工具用 PowerShell 写 |
| 预构建二进制 | `runner/sico-runner/target/debug/deps/selfhost_compiler-4fa1b02d73be6796.exe`（2026-11-06，对应干净 HEAD `a939862`）——**直接运行，无需重编译** |

**阻塞点（确切）**：runner 是 `windows-gnu`，依赖树里 `cc-rs` 需要 `gcc.exe` 链接；本机未装 ⇒ **任何 runner 侧 Rust 改动编译不出**。

**被此阻塞挡掉的方法**：任务建议的 **(b) wasmtime epoch/opcode 采样剖析**需要加测试侧钩子 ⇒ **无法构建**。⇒ 本片退到方法 **(a)+(c)**：用预构建二进制**已有的** env-selectable fuel 探针做**层级归因 + 真实站点 top-N 排名**（更便宜、可复现、版本化）。

**最小环境输入（解除阻塞）**：装 `gcc.exe`（mingw-w64 `x86_64-w64-mingw32-gcc` 或 MSYS2 `pacman -S mingw-w64-x86_64-gcc`）入 PATH，使 `cargo check --manifest-path runner/sico-runner/Cargo.toml --tests` 通过 ⇒ 之后即可建 epoch 采样做 op 级剖析（§4 残余缺口 1）。

## 2. 方法

工具 `tools/profile-m22-fuel-wall.ps1` 三条探针（全部只读 guest 源码）：

| 探针 | 隔离 | 选站点 |
|---|---|---|
| `..._real_match_group_is_measured`（**emit-only 注入**） | oracle 预建 canonical IR 直喂发射器，**跳过前端解析** ⇒ 只测**发射** fuel | `SICO_M22_PROBE_GROUPS=path:function,...` |
| `..._through_the_compiler_is_measured`（**ledger 路径**） | 完整 guest 编译器（`--emit-component-hex`），付**解析+造 IR+发射** | 同上 |
| `the_emitter_fuel_scaling_in_read_local_count_is_measured`（**合成 scaling**） | 固定形状族（`read_local` 计数 N / 函数数 F / `EMIT_ONLY`/`EMIT_SPLIT`/`EMIT_PADDED`） | 固定臂 |

三类结果分开记（沿用 STEP-0544）：`measured` / `exhausted`（**被测对象**）/ `refused`/`trap`；只有拒绝对照算硬失败。

## 3. 实测证据（命令 + 退出码 + 关键输出）

### 3.1 真实站点 top-N 排名（emit-only，IR 体量驱动）

```
powershell -NoProfile -File tools/profile-m22-fuel-wall.ps1 -Probe sites -Ceiling 100000000
# exit 0，13.3 s → target/tmp/step0562/fuel-wall-sites.csv（17 行 = 表头 + 16 族）
# 另在 Ceiling=6.4e9 复跑（exit 0，74 s）确认更高上限下仍全耗尽
```

| 排名 | guest 函数（`parser.sico`） | 站点数 | emit-only IR 字节 | @1e8 | @6.4e9 | ledger @800e9 |
|---:|---|---:|---:|---|---|---|
| 1 | `general_while_function_ir` | — | **4,573,438** | exhausted | exhausted | exhausted |
| 2 | `fixed_guard_chain_ir` | 7 | 3,378,177 | exhausted | exhausted | exhausted |
| 3 | `control_while_function_ir` | 23 | 3,288,574 | exhausted | exhausted | exhausted |
| 4 | `straight_cell_function_ir` | 18 | 3,209,321 | exhausted | exhausted | exhausted |
| 5 | `control_if_function_ir` | 12 | 3,157,992 | exhausted | exhausted | exhausted |
| 6 | `gw_condition_packed` | 11 | 3,144,004 | exhausted | exhausted | exhausted |
| 7 | `gw_slice_subject_packed` | 8 | 2,927,809 | exhausted | exhausted | exhausted |
| 8 | `gw_compare_packed` | 13 | 2,858,155 | exhausted | exhausted | exhausted |
| 9 | `gw_slice_sub_length_subject_packed` | 8 | 2,767,263 | exhausted | exhausted | exhausted |
| 10 | `gw_rhs_multi_pack` | 7 | 2,628,480 | exhausted | exhausted | exhausted |
| 11 | `while_call_rhs_packed` | 13 | 2,628,480 | exhausted | exhausted | exhausted |
| 12 | `gw_text_concat_packed` | 12 | 2,628,480 | exhausted | exhausted | exhausted |
| 13 | `checked_arithmetic_set_match_ir` | 13 | 579,065 | exhausted | exhausted | exhausted |
| 14 | `straight_text_concat_packed` | 16 | 507,543 | exhausted | exhausted | exhausted |
| 15 | `gw_ok_wrap_rhs_packed` | 12 | 480,008 | exhausted | exhausted | exhausted |
| 16 | `fixed_literal_const_segment` | — | **92,055** | exhausted | exhausted | exhausted |

**判读**：**16/16 族在 emit-only 层连 1e8 都耗尽**，`component_bytes=0` 全体成立 ⇒ **没有一个能出组件**，这批站点**不是边际站点**而是在发射层就差 **3 个数量级以上**。**IR 体量跨 2 个数量级（92 KB→4.6 MB，49.7×）** ⇒ 按 fuel~IR² 折算 fuel 需求差 **~2470×** ⇒ **IR 体量就是 fuel 墙的驱动量，top-N 据此成立**。

### 3.2 合成 scaling：fuel ~ IR²，墙在发射器（上限 25.6e9 复现）

```
SICO_M22_PROBE_FUEL=25600000000 selfhost_compiler-4fa1b02d...exe \
  the_emitter_fuel_scaling_in_read_local_count_is_measured --exact --ignored --nocapture
# SYNTH_EXIT=0 → target/tmp/step0562/synthetic-scaling.log
```

**① 每函数内平方（`read_local` N，含解析）**：64/128/256 → `2e8…4e8`（平）；512 → `4e8…8e8`（2×）；1024 → `8e8…1.6e9`（2×）；**2048 → `3.2e9…6.4e9`（4×）**；**4096 → `6.4e9…1.28e10`（4×）** ⇒ N≥2048 每翻倍烧 4× = **每函数 O(N²)**，膝点 ≥2048，**真实函数（10–300 指令）在膝点之下** ⇒ 非主导。

**② 模块级平方（函数数 F，含解析）**：16 → `1.6e9…3.2e9`；32 → `6.4e9…1.28e10`（**4×**）；64/128/256 → **exhausted**（>25.6e9）⇒ 模块级平方。

**③ emit-only（免解析，决定性）**

| F | IR 字节 | EMIT_ONLY fuel | 含解析 fuel | 判读 |
|---:|---:|---|---|---|
| 16 | 7,405 | 3.2e9…**6.4e9** | 1.6e9…3.2e9 | emit-only **贵 2×** |
| 32 | 12,509 | 1.28e10…**2.56e10** | 6.4e9…1.28e10 | emit-only **贵 2×** |
| 64 | 22,717 | **exhausted** | exhausted | 两者都耗尽 |

⇒ **免解析仍平方、每档贵 ~2×** ⇒ **平方项在发射器层，非解析器**（第二个上限复现 STEP-0555 §10.4c）。

**④ fuel/IR² 归一（`EMIT_SPLIT`/`EMIT_PADDED`/`EMIT_ONLY` 印证 total-IR²）**

| 臂 | F | IR 字节 | fuel(high) | fuel/IR²（G/kb²） |
|---|---:|---:|---|---:|
| EMIT_SPLIT | 4 | 3,619 | 1.6e9 | 122.2 |
| EMIT_SPLIT | 16 | 7,433 | 6.4e9 | 115.8 |
| EMIT_PADDED | 8 | 4,846 | 3.2e9 | 136.3 |
| EMIT_PADDED | 32 | 12,489 | 2.56e10 | 164.2 |
| EMIT_ONLY | 16 | 7,405 | 6.4e9 | 116.8 |
| EMIT_ONLY | 32 | 12,509 | 2.56e10 | 163.6 |

⇒ 两种控制变量下 **fuel/IR² 近似恒定（115–164，同量级）** ⇒ **fuel ~ total-IR²**（C 主导，D 有小贡献）。**真实站点 IR 92 KB–4.6 MB ⇒ 折算 fuel 需求 ~1e12–3.5e12，超 800e9 预算 1.5–4×** ⇒ **定量解释 335 站点全耗尽**。

## 4. 结论 · 诚实收窄 · 未跑项

### 4.1 本片钉死的

1. **墙在发射器层（非解析器）**——合成 emit-only 免解析仍平方且贵 ~2×（§3.2③）。
2. **发射器 fuel ~ total-IR²**——三臂 fuel/IR² 近似恒定（§3.2④）。
3. **真实 335 耗尽站点 = IR 92 KB–4.6 MB 闭包在发射层耗尽**（§3.1，16/16 @1e8/@6.4e9/@800e9 全 exhausted、component_bytes=0）。
4. **真实站点 top-N 排名产出**（§3.1，按 IR 体量）。

### 4.2 诚实收窄

- **平方项的具名 wasm-op 源未定位**：缺 gcc ⇒ runner 无法重编译 ⇒ **epoch/opcode 采样无法构建**（§1）。静态可读路径已被 STEP-0555 §10.6/§10.7 与 STEP-0556 §10 三刀证伪（字节等价但 fuel 不变）⇒ 剩余候选是发射器里**某个随 total-IR 增长的隐藏平方**（`cgj_member`/`cgj_array_item` 按字段名/按字节重扫、或 core-helper 体内部未测 O(n²)），**本片无法在 op 级裁决**。**这是下一刀前置。**
- **真实站点无 measured 括号**：任何可达上限全 exhausted ⇒ 只给**下界**；fuel~IR² 斜率来自**合成**臂，**系数形状相关**，迁移到真实站点是**外推**。
- **`g_per_kb2` 在真实站点列失真为 0**（fuel_lb 被上限截断、IR 达 MB 级 ⇒ 精度下溢）⇒ 真实站点 fuel/IR² **不能用该列读**，只能用 IR 体量排名；该列只在合成 measured 点有意义。
- 证据等级仍 `internal-fixture` ⇒ 非 `clean-room-consumer`/`external-pilot`/`production`。

### 4.3 未跑项（如实登记）

| 项 | 状态 | 阻塞 |
|---|---|---|
| wasmtime epoch/opcode 采样剖析 | **未跑** | 本机缺 `gcc.exe` ⇒ runner 无法重编译（exit 101，§1） |
| 真实站点 parse-vs-emit 分账（`-Probe parsevs`） | **未跑** | emit-only 与 ledger 路径在同一上限下都 exhausted ⇒ 括号不可分，预期无区分度；分账结论取自合成 EMIT_ONLY vs EMITTER_SCALING（§3.2③） |
| 整轮电池 / 独立 CI | **未跑** | 本片零代码改动（仅新增工具 + 文档），不触发 runner 指纹变化；且重编译被 gcc 阻塞 |

### 4.4 门状态（不变）

本片**实测账本 verdict 分布**（`selfhost/match-component-coverage-v1.json`，396 站点）：**335** `ir-equal+component-other:FuelExhausted` / **32** `ir-equal+component-equal` / **24** `guest-refused:Trap(8 MiB)` / **5** `ir-equal+component-refused-guest`。
> **漂移登记（如实，不翻门）**：任务简报给的是 **331 exhausted / 36 component-equal**；本片在当前树（HEAD `a939862`）实测为 **335 / 32**（4 站点从 equal 移到 exhausted）。本片是测量片，**不重推导账本、不翻任何门**，仅登记实测读数供 reviewer 核对。

门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用 —— **全部维持**。**M22 整体仍 NO-GO，本片零门翻转。**

## 5. 改动面

```
$ git status --short   # 本片新增（另有其他蜂群代理的在途文件，非本片）
?? tools/profile-m22-fuel-wall.ps1
?? docs/reports/m22-fuel-wall-profile-2026-11-06.md
?? docs/steps/STEP-0562-m22-w2-fuel-wall-profile.md
```

`selfhost/parser.sico`、`selfhost/codegen.sico`、`runner/sico-runner/tests/selfhost_compiler.rs`、`crates/*`、reviewer 文档（STATUS/handoff/plans/steps README/ROADMAP/rfc/adr）**均未改动**。`git diff --check` 干净（无空白错误）。

## 6. 复跑（从干净命令）

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
powershell -NoProfile -File tools/profile-m22-fuel-wall.ps1 -Probe sites -Ceiling 100000000      # 真实站点 top-N，~15 s
powershell -NoProfile -File tools/profile-m22-fuel-wall.ps1 -Probe synthetic -Ceiling 25600000000 # 合成 scaling，~5 min
```
