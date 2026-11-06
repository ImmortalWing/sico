# STEP-0572 · M22 W2 · op 级燃料剖析阻塞解除（编译矩阵实测 + 测试侧采样钩）

> **片型** = 阻塞解除 + 测量片（诊断）。不推进任何出口门、不上调限额、不改 runner 公共 API、M22 整体仍 **NO-GO**。
> **唯一代码产物** = 新集成测试 `runner/sico-runner/tests/m22_fuel_profile.rs`（runner 侧新文件唯一写者）。
> **权威报告** = [`docs/reports/m22-op-level-profile-2026-11-06.md`](../reports/m22-op-level-profile-2026-11-06.md)。
> **禁改清单（本片遵守）**：`selfhost/parser.sico`、`selfhost/codegen.sico`、`runner/sico-runner/tests/selfhost_compiler.rs`、`crates/*` 生产代码、reviewer 文档（STATUS/handoff/plans/ROADMAP/rfc/adr/**steps README**）。本片**未**编辑 `docs/steps/README.md`（本批该权指派给 STEP-0573）。

## 0. 背景与前序矛盾

两个前序结论冲突：
- `cargo check --locked --offline --manifest-path runner/sico-runner/Cargo.toml --tests` → exit 101，`cc-rs: failed to find tool "gcc.exe"`。
- `cargo test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler --no-run` → 却链接成功（rust-lld）。

STEP-0562 据此判定「runner 无法重编译 ⇒ wasmtime epoch/opcode 采样无法构建」，op 级燃料源未定位（M22 第一硬阻塞）。本片实测整张编译矩阵裁定该矛盾。

## 1. 编译矩阵裁定（命令 + 退出码 + 耗时，全部实跑）

| 格 | 命令 | exit | 耗时 | 性质 |
|---|---|---:|---:|---|
| (a) | `cargo check --tests` | 101 | ~2 s | gcc/cc-rs（check profile） |
| (b) | `cargo test --no-run --test selfhost_compiler` | 101 | 6 s | **非 gcc**：编译过，rust-lld 写 exe `Permission denied`（并发代理持锁） |
| (c) | `cargo build --tests` | **0** | 427 s | 全测试目标编过 |
| (d) | `cargo test --no-run`（全工作区） | **0** | 69 s | 全编过 |
| 附加 | `cargo clippy --test m22_fuel_profile -- -D warnings` | 101 | — | gcc/cc-rs（clippy profile） |

**卡点（精确）**：依赖 crate **`wasmtime-internal-fiber v47.0.2`** 的 `build.rs` 用 `cc-rs` 编 `src/windows.c`，需 `gcc.exe`（本机无）。是否触发取决于 build-dir 哈希是否已有缓存 `out/ea708c7824d36062-windows.o`：
- build/test profile → `...-4e89d76ba2d977c7/` **有缓存 `.o`**（2026-11-04）⇒ 不调 cc-rs ⇒ 链接成功；
- check profile → `...-669e568f003f8b7c/`、clippy profile → `...-b3d5…` **无缓存 `.o`** ⇒ 重编 ⇒ gcc 缺失 ⇒ 101。

**裁定**：**runner 可编译、可测试**；gcc 只挡 check/clippy 两个 profile，而二者**不是**新增/运行测试侧钩所必需。STEP-0562「runner 无法重编译」对 build/test **过强**，收窄为「check/clippy 被 gcc 挡」。**最小解除输入**：装 mingw-w64 `gcc.exe`，或对测试侧钩改用 `cargo build/test`（已足够）。

## 2. 测试侧采样钩：可编译性 = 可，已落地

- `cargo test --no-run --test m22_fuel_profile` → **exit 0** ⇒ **可编译**，任务第 2 项判定为「可行」。
- **方法**：`fuel_bracket`（几何阶梯 + 末倍频程细化）+ **emit-only 注入**（canonical IR 直喂冻结 `codegen`，跳过 guest 前端 ⇒ 只付发射 fuel）+ **函数级归因**（受控合成闭包，每次只变一个发射器 workload）。即「per-function closure replacement」在**输入侧**的落地：换闭包体构成，发射器保持冻结。
- **为何仍非 op 级**：runner 公共 API（`PreparedProgram::run`）**无 fuel 消耗读数、无 epoch/opcode 回调**（加钩需改 runner 生产代码，本片禁改）⇒ 只能二分「发射整组件所需 fuel 上限」。

## 3. 实测结果（关键输出）

### 3.1 真实 `FuelExhausted` 站点（emit-only，实测下界）

```
site=parser.sico:fixed_literal_const_segment ir_bytes=92057 component_bytes=0
  fuel_bracket=exhausted (lower bound only) search_ms=2281     # 有界 ceiling 6.4e9
```

经依赖闭包抽取后 emit-only **92,057 B** IR（STEP-0562 同站点 92,055 B，差 2 B 来自 `run` 包装），连 6.4e9 都 exhaust、`component_bytes=0` ⇒ 从「ceiling 截断」改为**实测下界（≥6.4e9）**；`SICO_M22_PROBE_FUEL` 可抬到 800e9 跑全括号。

### 3.2 发射器热点 cost-center 排名（emit-only，ceiling 25.6e9）

| 排名 | family | n=8 | n=16 | n=32 | n=64 |
|---:|---|---|---|---|---|
| 1 | **match_statements** | exhausted（21 KB IR） | exhausted（40 KB） | exhausted（78 KB） | exhausted（154 KB） |
| 2 | **text_concat** | bracket **14.4e9**（6.6 KB） | exhausted | exhausted | exhausted |
| 3 | **functions** | bracket 1.8e9（4.8 KB） | bracket 4.4e9（per-unit 斜率 1.22） | bracket 14.4e9（斜率 1.64） | exhausted（23 KB） |
| — | forwarded_let_control | 450e6 | 450e6 | 450e6 | 450e6（**平坦阴性对照**） |

**排名**：`match_statements` > `text_concat` > `functions` ≫ 平坦对照。`match_statements` 连 n=8/21 KB IR 都 >25.6e9 ⇒ match 降低路径单位 IR 最贵；`text_concat`（hex 组装）次之；`functions` per-unit 超线性（1.22→1.64）⇒ 每模块/每函数扫描项。平坦对照证明增长族测的是发射器真实功，非二分搜索伪影。

### 3.3 附带发现

- Rust oracle 有 per-function 上限：`match_statements` n=128 被拒 `VerifyError{kind: Limit}` ⇒ 本片记为 `oracle_refused`（不 panic），n 收到 64。
- 源级 `read_local` 链被前端 SSA 前向传播折叠（IR 恒 ~2 KB、fuel 恒 450e6）⇒ 非有效探针；值槽成本的合法仪器是 `selfhost_compiler.rs` 的直构 IR `x_family_ir`（本片禁改该文件）。

## 4. 先红后绿

- **红**：`fixed_literal_const_segment` emit-only 在有界阶梯全 exhaust、`component_bytes=0` ⇒ 发射器 fuel 墙在该真实站点为真（非边际站点）；`match_statements`/`text_concat` 在 modest ceiling 即 exhaust ⇒ 超线性为真。
- **绿**：`the_fuel_bracket_brackets_a_known_threshold` 钉死二分本身（3 阈值全括号 + 超 ceiling 无括号）；排名测试 3/3 passed（exit 0）。

## 5. 诚实收窄与未跑项

- fuel 括号给 `(low, high]` 或「仅下界」，**不定位到 wasm-op**；**op 级源仍未定位**（需 runner 生产 API 暴露 fuel 读数/epoch·opcode 钩）。
- `match_statements`/`text_concat` 大 n 只有下界；`functions` 斜率来自合成臂，迁到真实站点是外推。
- 证据等级仍 `internal-fixture`；单机单快照，非运行时/性能声明。
- **`cargo clippy` 未过本文件**（clippy profile gcc-blocked）；本文件经 `cargo build/test` + `cargo fmt --check` 验证。
- 整轮电池 / 独立 CI **未跑**（测量片，仅新增默认隔离的测试文件）。

## 6. 门状态（不变）与复跑

门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用 —— **全部维持**；M22 整体 **NO-GO**，本片零门翻转。

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test m22_fuel_profile --no-run
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test m22_fuel_profile -- --test-threads=1 --nocapture --include-ignored
& $cargo fmt --manifest-path .\runner\sico-runner\Cargo.toml -- --check   # 本文件无 diff
git diff --check
```

最近一次：构建 exit 0；全量测试 `test result: ok. 3 passed; 0 failed`（70.6 s）；fmt 对本文件无 diff；`git diff --check` 干净。