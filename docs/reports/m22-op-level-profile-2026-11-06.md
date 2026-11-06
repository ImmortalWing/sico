# M22 op 级燃料剖析阻塞解除：编译矩阵实测 + 测试侧采样钩（STEP-0572）

> **片型** = 阻塞解除 + 测量片。`crates/*` 零改动、`selfhost/parser.sico` 与 `selfhost/codegen.sico` 零改动（只读）、`runner/sico-runner/tests/selfhost_compiler.rs` 零改动（未被指定写）、reviewer 文档（STATUS/handoff/plans/ROADMAP/rfc/adr/steps README）零改动。
> **唯一代码产物** = 新集成测试 `runner/sico-runner/tests/m22_fuel_profile.rs`（runner 侧新文件，本片为唯一写者）。
> **不推进任何出口门、不上调 fuel/内存限额、不改 runner 公共 API、M22 整体仍 NO-GO。**
> 环境：Windows + Git Bash，`RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu`，cargo=`~/.cargo/bin/cargo.exe`，manifest=`runner/sico-runner/Cargo.toml`，全部 `--locked --offline`。

## 1. 结论先行（一段话）

前序矛盾已裁定：**runner 在本机是可编译、可测试的**；`gcc`/cc-rs 只挡 `cargo check` 与 `cargo clippy` 两个 profile，而这两个 profile **不是**新增/运行测试侧钩所必需的。因此测试侧采样手段**可编译、已落地、已跑通**（3/3 passed）。但**真正的 op 级（opcode/epoch）采样仍未打通**，原因与 gcc 无关，而是 runner 公共 API（`PreparedProgram::run`）**不暴露 fuel 消耗读数、也不暴露 wasmtime Store/epoch/opcode 回调** ⇒ 本片退到 **fuel 二分 + 函数级归因**，产出一个真实 `FuelExhausted` 站点的实测下界 + 发射器热点 cost-center 排名。STEP-0562「runner 无法重编译 ⇒ epoch 采样无法构建」的表述对 build/test profile **过强**，本片予以收窄。

## 2. 编译矩阵实测（每格：命令 + 退出码 + 耗时）

| 格 | 命令 | 退出码 | 耗时 | 卡点 |
|---|---|---:|---:|---|
| (a) | `cargo check --tests` | **101** | ~2 s | `wasmtime-internal-fiber` build.rs → cc-rs → `gcc.exe` 缺失 |
| (b) | `cargo test --no-run --test selfhost_compiler` | **101** | 6 s | **非 gcc**：编译全程通过，仅 rust-lld 写 exe 时 `Permission denied`（并发代理持有该 exe） |
| (c) | `cargo build --tests` | **0** | 427 s (7m06s) | 无（全部测试目标编过） |
| (d) | `cargo test --no-run`（runner 全工作区） | **0** | 69 s | 无 |
| 附加 | `cargo clippy --test m22_fuel_profile -- -D warnings` | **101** | — | 同 (a)：clippy profile 的 fiber build dir 无缓存 `.o` → cc-rs → gcc |

### 2.1 cc-rs 到底卡哪个 crate / 哪条路径（精确）

- **卡点 crate** = 依赖 **`wasmtime-internal-fiber v47.0.2`**（非任何 `crates/*`，非 runner 自身代码）。
- **卡点路径** = 该 crate 的 `build.rs` 用 `cc-rs` 编译 **`src/windows.c`**（fiber 栈切换的 Windows 实现），`cc-rs` 需要 `gcc.exe`；本机 `which gcc/cc/clang` 全部 not found，`/e/msys64` 下也无 `gcc.exe`。
- **为什么 (a) 失败但 (c)/(d) 成功 —— build-dir 哈希 + 缓存 `.o`**：
  - build/test profile → build dir `target/debug/build/wasmtime-internal-fiber-4e89d76ba2d977c7/`，**含缓存 `out/ea708c7824d36062-windows.o`**（2026-11-04 编译）⇒ 不再调 cc-rs，直接 rust-lld 链接。（(b) 失败的链接命令行里可见 `-L ...wasmtime-internal-fiber-4e89d76ba2d977c7\out`，证明用的是缓存 `.o`。）
  - check profile → build dir `...-669e568f003f8b7c/`，**只有 build-script 二进制、无 `out/*.o`** ⇒ 必须重编 `windows.c` ⇒ cc-rs ⇒ gcc 缺失 ⇒ exit 101。
  - clippy profile → build dir `...-b3d5…` 同理无缓存 `.o` ⇒ exit 101。
- **最小解除输入**：(i) 安装 mingw-w64 `gcc.exe` 入 PATH（`pacman -S mingw-w64-x86_64-gcc` 或等价），使 check/clippy profile 也能过；或 (ii) 不用 check/clippy，改用 `cargo build/test`（本机已可用）。对新增测试侧钩而言 **(ii) 已足够**。

### 2.2 (b) 的 101 是并发写冲突，不是编译失败

(b) 6 s 失败，错误是 `rust-lld: error: failed to write output '...selfhost_compiler-4fa1b02d73be6796.exe': Permission denied` —— 另一蜂群代理的测试进程正持有同名 exe。按并发纪律未杀对方进程、未重试同一命令；(c)/(d) 在对方释放后 exit 0，反证编译本身无问题。**新增文件的 exe 名（`m22_fuel_profile-*.exe`）不与既有目标冲突**，故本片测试构建无此冲突。

## 3. 测试侧采样钩：可编译性裁定 + 落地

- **可编译**：`cargo test --no-run --test m22_fuel_profile` → **exit 0**（17 s 首编；改动后复跑同样 exit 0）。⇒ **任务第 2 项的可编译性判定为「可」**，测试侧钩可行。
- **方法**：fuel 二分（`fuel_bracket`，几何阶梯 + 末倍频程细化）+ **emit-only 注入**（canonical IR 直喂 guest `codegen`，跳过 guest 前端 ⇒ 只付发射 fuel）+ **函数级归因**（受控合成闭包，每次只让一个发射器 workload 变化）。这就是任务所称「per-function closure replacement 法」在**输入侧**的落地：替换闭包体构成，发射器保持冻结。
- **为何不是 op 级**：runner 公共 API 无 fuel 读数/无 epoch·opcode 回调（`crates/*` 与 runner src 本片禁改）⇒ 无法读「每 opcode 烧多少」，只能二分「发射整组件所需 fuel 上限」。

### 3.1 复跑命令

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
# 构建（证明测试侧钩可编译）
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test m22_fuel_profile --no-run
# 全部 3 个测试（含 #[ignore] 排名，~70 s）
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test m22_fuel_profile -- --test-threads=1 --nocapture --include-ignored
```

最近一次全量：`FINAL_EXIT=0`，`test result: ok. 3 passed; 0 failed`，70.6 s。`cargo fmt -- --check` 对本文件**无 diff**。

## 4. 实测结果

### 4.1 真实 `FuelExhausted` 站点（emit-only，实测下界）

```
M22_FUEL_PROFILE site=parser.sico:fixed_literal_const_segment ir_bytes=92057
  component_bytes=0 fuel_bracket=exhausted (lower bound only) search_ms=2281
```

- 站点 = `parser.sico:fixed_literal_const_segment`（STEP-0562 top-N 里**最小**的耗尽站点），经依赖闭包（`sico_closure`，≤256 fn / 1 MiB）抽取后 emit-only 降为 **92,057 B** canonical IR（与 STEP-0562 的 92,055 B 一致，差 2 B 来自 `run` 包装）。
- 在有界阶梯（seed 1e8 → ceiling 6.4e9）**全 exhausted**，`component_bytes=0` ⇒ **连 6.4e9 都发不出组件**，与 STEP-0562「16/16 @1e8/@6.4e9 全 exhausted」一致。本片把它从「ceiling 截断」改成**实测下界**（≥6.4e9），并保留 `SICO_M22_PROBE_FUEL` 抬到 800e9 跑全括号的入口。

### 4.2 发射器热点 cost-center 排名（emit-only，ceiling 25.6e9，实测）

| 排名 | family（受控合成闭包） | n=8 | n=16 | n=32 | n=64 | 判读 |
|---:|---|---|---|---|---|---|
| 1 | **match_statements**（单函数 N 个 `match U64.checked_add` 语句） | **exhausted**（21 KB IR） | exhausted（40 KB） | exhausted（78 KB） | exhausted（154 KB） | **最热**：连 n=8/21 KB IR 都 >25.6e9 ⇒ match 降低路径单位 IR 最贵 |
| 2 | **text_concat**（单函数 N 次 `sico.text.concat`） | bracket **14.4e9**（6.6 KB IR） | exhausted（11 KB） | exhausted（20 KB） | exhausted（37 KB） | **很热**：hex 组装路径（`cgj_text_concat`/逐 op concat） |
| 3 | **functions**（F 个微型函数） | bracket 1.8e9（4.8 KB） | bracket 4.4e9（7.4 KB）per-unit 斜率 **1.22** | bracket 14.4e9（12.5 KB）斜率 **1.64** | exhausted（23 KB） | **热且超线性**：per-unit 递增 ⇒ 每模块/每函数扫描项 |
| — | forwarded_let_control（`let a{i}=a{i-1}` 链） | 450e6 | 450e6 | 450e6 | 450e6 | **平坦（阴性对照）**：前端 SSA 前向传播把 `let` 链折叠，IR 恒 ~2 KB ⇒ 证明增长族测的是发射器真实功，非二分搜索伪影 |

**排名产出**（fuel 上界递减 = 越热）：`match_statements` > `text_concat` > `functions` ≫ `forwarded_let_control`（平坦）。前两者在大 n 只有下界（全 exhausted），`functions` 有 3 个 bracket 点可读出 per-unit 超线性（1.22→1.64）。

### 4.3 附带发现（如实登记，不改门）

- **oracle 自身有 per-function 上限**：`match_statements` n=128 被 Rust oracle 拒绝 `InvalidIr([VerifyError { path: "function[0]", kind: Limit }])`（单函数指令数超限）。本片把它作为**记录型测量结果**（`oracle_refused=…`）而非 panic；n 上限因此收到 64。
- **源级 `read_local` 族不是有效探针**：前端 SSA 前向传播把 `let a1=a0` 链折叠，IR 恒 ~2 KB、fuel 恒 450e6 ⇒ 无法把 read_local 计数当受控变量。值槽成本的合法仪器是 `selfhost_compiler.rs` 里的**直构 IR** `x_family_ir`（本片禁改该文件，故未复制），已在此如实标注。

## 5. 诚实收窄

- **fuel 括号 ≠ op 级读数**：给的是 `(low, high]` 或「仅下界」，**不定位到具体 wasm-op**。op 级源仍未定位（需 runner 公共 API 暴露 fuel 读数或 epoch/opcode 钩，属生产代码改动，本片禁改）。
- **大 n 只有下界**：`match_statements`/`text_concat` 在 n≥16 即 exhausted ⇒ 斜率只能从 `functions` 的 3 个 bracket 点与各族的 exhaust 顺序读出，属**排名证据**，非精确指数。
- **系数形状相关**：`functions` 的 per-unit 超线性斜率来自合成臂，迁移到真实站点是外推（同 STEP-0562 §4.2 的收窄）。
- **证据等级仍 `internal-fixture`**，非 clean-room-consumer / external-pilot / production。
- **clippy/check gcc-blocked** ⇒ 本文件只经 `cargo build/test` + `cargo fmt --check` 验证，**未经 clippy**（如实登记）。
- 单机单快照观测，非运行时/性能声明。

## 6. 改动面

```
$ git status --short   # 本片唯一新增
?? runner/sico-runner/tests/m22_fuel_profile.rs
```

`crates/*`、`selfhost/parser.sico`、`selfhost/codegen.sico`、`runner/sico-runner/tests/selfhost_compiler.rs`、reviewer 文档（STATUS/handoff/plans/ROADMAP/rfc/adr/**steps README**）**均未改动**。本片**未**编辑 `docs/steps/README.md`（该维护权在本批指派给 STEP-0573）。`git diff --check` 干净（仅他人 CRLF 提示，非本片文件）。

## 7. 门状态（不变）

门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用 —— **全部维持**。**M22 整体仍 NO-GO，本片零门翻转、零限额上调。**

## 8. 未跑项

| 项 | 状态 | 阻塞 |
|---|---|---|
| true op 级（opcode/epoch）采样 | **未跑** | runner 公共 API 无 fuel 读数/epoch·opcode 回调；加钩需改 runner 生产代码（本片禁改） |
| 真实站点全括号（抬到 800e9） | **未跑（默认）** | 有界默认 6.4e9 已 exhaust ⇒ 只有下界；800e9 全括号留作 `SICO_M22_PROBE_FUEL` 显式入口 |
| `cargo clippy` 对本文件 | **未过** | clippy profile gcc-blocked（§2 附加行） |
| 整轮电池 / 独立 CI | **未跑** | 测量片，仅新增一个 `#[ignore]`-默认隔离的测试文件；不触发 runner 生产指纹 |