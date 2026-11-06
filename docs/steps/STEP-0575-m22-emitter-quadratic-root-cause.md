# STEP-0575 · M22 W2 · 发射器平方项根因：`match_statements` 族 O(n²) 定位 + 修复设计

> **片型** = **设计片 + 判别实验**（**不落地**）。`selfhost/codegen.sico`、`selfhost/parser.sico` **零改动（只读）**；`runner/sico-runner/tests/selfhost_compiler.rs`、`selfhost/match-component-coverage-v1.json` **零改动**（非本片指定写）。
> **唯一代码产物** = `runner/sico-runner/tests/m22_fuel_profile.rs` 新增一个 `#[ignore]` 判别臂（本片是该文件唯一写者）。
> **权威报告** = [`docs/reports/m22-emitter-quadratic-design-2026-11-06.md`](../reports/m22-emitter-quadratic-design-2026-11-06.md)。
> **禁改清单（本片遵守）**：`selfhost/parser.sico`、`selfhost/codegen.sico`、`runner/sico-runner/tests/selfhost_compiler.rs`、`selfhost/match-component-coverage-v1.json`、`crates/*` 生产代码、reviewer 文档（STATUS/handoff/plans/ROADMAP/rfc/adr/steps README）。本片**未**编辑 `docs/steps/README.md`（登记留评审者）。

## 0. 一句话结论

**判别实验证明 `match_statements` 的 fuel 是 match 数的 perfect-square（n=2→4 恰好 4.0×，observed/预测 = 1.000），且平方项随 match 数而非指令数**（1 条 match + N−1 条 `set` 的对照臂在更大 body 下亚平方）⇒ 平方项在 **match 降低路径本身**；静态读码钉死两个具名源（**源 A** = `cgs_match_hex` 每 match 调一次 `cgs_dispatcher_slot` 全 body 重扫；**源 B** = match 族左折叠 concat 累加），给出候选 patch 落点 + 预期曲线，**但按 §3.3 量级估算两源绝对贡献都 <0.1% ⇒ 与「墙 = 常数主导」一致，故主刀落地前必须先跑 ledger match-密度分账（§4.4）证伪/证实，否则预测回绿 ~0**。**未落地、未改门、M22 仍 NO-GO。**

## 1. 任务与前序

- 任务：精读 `cgs_match_hex`/`cgs_match_condition_hex`/`cgs_match_pattern_hex`/`cgs_match_payload_ok` + 公共发射循环，找出随 IR 字节二次增长的具体循环/字符串操作；用 STEP-0572 的钩做判别实验；产出修复设计（不新增 packer、不上调限额、字节不变、过自举形状前沿）；给出伪代码落点 + 预期曲线，**不落地**。
- 前序：STEP-0562/0572 定位墙在发射器层、`fuel ~ IR²`、`match_statements` 最热（n=8/21 KB 即耗尽）；STEP-0555/0556 已证伪六类假设（数组游标化 / cell_slot / 文本累加 / 解析器 / join 内部 / append 内部 / bytes.length），§10 line 124 称「只剩 `cgj_member` 一个候选」。本片把该「唯一候选」**收窄并部分推翻**：`cgj_member` 扫描限单对象、不随 match 数超线性；真正的随 match 数放大项是 **`cgs_match_hex` 每 match 重算 `cgs_dispatcher_slot`**。

## 2. 静态读码（只读，零改动）

| 嫌疑 | 位置 | 读码判定 |
|---|---|---|
| `cgs_match_hex` 每 match 调 `cgs_dispatcher_slot` | `codegen.sico:6500` | **源 A（主）**：`cgs_dispatcher_slot:2209` 用 index 形 `cgj_array_item` 走**全 run body** ⇒ N match × O(N) body = **O(N²)** |
| `cgs_match_hex`/`condition`/`pattern` 左折叠 `concat` 累加 | `:6525-6532` / `:6471` / `:6405,6436` | **源 B（次）**：输出 O(N) hex ⇒ 累加 O(N²) 字符拷贝 |
| arm/values/payload 循环用 index 形 `cgj_array_item` | `:6509` / `:6458-6459` / `:6349` | B3 已证伪（游标化 fuel 净负）；payload 极短 ⇒ 小项 |
| `cgj_member` 字段查找重扫 | 族内多处 | **排除**：扫描限**单对象**跨度、非全 body，不随 match 数超线性（部分推翻 STEP-0556 §10「唯一候选」） |
| `cgs_value_slot`/`cgs_value_ty` | `:2118`/`:2133` | B2 已 O(1) stride-2 索引，无残差 |
| `cgs_function_instr_hex` 主循环 | `:4064` | **已线性**（`cgj_array_first/next` 游标 + 一次 `join`，P1 落地）⇒ 平方只能来自 match 族自身 |

## 3. 判别实验（实测，命令 + 退出码 + 关键输出）

### 3.1 新增探针（`m22_fuel_profile.rs`，`#[ignore]`）

`the_match_quadratic_term_is_discriminated_from_body_scanning`，沿用 STEP-0572 的 emit-only 基建，两臂各只变一个因素：

- **`match_flat`**：单函数 N 条 match（2 case）——隔离 match 降低路径。
- **`match_one`**：**1 条** match + N−1 条 `set acc = acc`——body 指令数 ≥ `match_flat` 但 match 数恒 1——隔离通用逐指令体扫描。

**首版失败（如实登记）**：对照臂首版用 `set s{i} = acc`（未声明名）⇒ oracle 4/4 拒 `Unsupported { feature: "set without prior let" }`（exit 0，oracle_refused）。改为 `set acc = acc` 后 4 档全接受。

### 3.2 实测（`--include-ignored`，exit 0，35.19s）

| family | n | ir_bytes | fuel_mid | observed/预测平方 |
|---|---:|---:|---:|---:|
| `match_flat` | 2 | 6,995 | 5.6e9 | — |
| `match_flat` | 4 | 11,635 | **2.24e10** | **1.000**（5.6e9×4，perfect square） |
| `match_flat` | 8 | 21,060 | exhausted | >25.6e9 |
| `match_flat` | 16 | 40,052 | exhausted | >25.6e9 |
| `match_one` | 2 | 4,933 | 2.8e9 | — |
| `match_one` | 4 | 5,429 | 3.6e9 | 3.111（**低于** 1.12e10） |
| `match_one` | 8 | 6,418 | 5.6e9 | 2.571（低于 1.44e10） |
| `match_one` | 16 | 8,394 | 1.44e10 | 1.556（低于 2.24e10） |

**判读**：`match_flat` n=2→4 **恰好 4.0×** = O(n²)；`match_one` 在 n=16（body 8,394 B）与 `match_flat` n=4（11,635 B）同量级但 match 数=1 ⇒ fuel 仅 1.44e10 且**始终低于**自身平方外推 ⇒ **平方项随 match 数、不随指令数 ⇒ 在 match 降低路径本身**。

### 3.3 诚实收窄（决定设计，必读）

在 `match_flat` n=4（fuel 2.24e10）处估算两源绝对贡献：源 A（4 次 × 8 指令 × 数十 op）≈ **1e2–1e3 op**；源 B（数百 hex 字符 O(N²) 拷贝）≈ **数千 hostcall** —— 相对 2.24e10 **都 <0.1%**。⇒ 2.24e10 主要是**每 match 固定常数 × N**，与 STEP-0556 §11「墙 = 常数主导，不是算法主导」一致。**判别实验钉死的是「平方项存在且随 match 数」，不是「平方项是 fuel 墙主导项」**——后者需 fuel 读数/epoch 钩（STEP-0572 §5，本机不可得）。

### 3.4 先红后绿

- **红**：`match_flat` n=8/16 在 25.6e9 ceiling 耗尽 + n=2→4 perfect-square ⇒ match 降低路径真有平方墙（非边际）。
- **绿**：对照臂 `match_one` 亚平方 ⇒ 平方项是 match 专属、非通用 body 扫描（否则两臂都超线性）；默认 2 测仍 `2 passed`。

### 3.5 未跑项

- 源 A vs 源 B 的**源级分离未做**（合法源每 match 恰 2 case，无法只增单 match 累加长度；合成 IR 直构 `values` 长度可变的路径 STEP-0555 §10.4 已记录被发射 gate 拒，本片未重试）。
- 修复**未落地、未测 ledger**（设计片约束）。
- `cargo clippy` 未过本文件（clippy profile gcc-blocked）；经 `cargo build/test` + `cargo fmt --check` 验证。

## 4. 修复设计（不落地，详见报告 §4）

- **主刀（源 A）**：`cgs_dispatcher_slot` 每 match 重算 → **每函数一次**，经 `cgs_function_instr_hex → cgs_block_terminator_hex → cgs_match_hex` 透传（+1 个 U64 形参，删 `:6500-6503` 自算）。纯函数缓存 ⇒ **字节不变**、无嵌套 call、无新 packer。**预期曲线**：若源 A 主导，`match_flat` 从 4.0×/倍降为 ~2.0×/倍；**但据 §3.3 更可能逐档不变**（沿用 B3/B4-D/P1/C 五刀模式）。
- **次刀（源 B）**：match 族三处累加器改 `list.append` + 一次 `join`（STEP-0549/0556 idiom）。**诚实预测 ~0**：STEP-0556 §10 实测 `join` 线性、`append` 摊还 O(1) ⇒ 只挪拷贝位置、总 O(N²) 字节不变。低优先。
- **已否决**：游标化（B3 净负）、value_slot 索引化（B2 已做）、`cgj_member` 索引化（单对象 + B4-D 同构已证伪）。
- **落地前必跑判据（核心交付）**：先对 `match-component-coverage-v1.json` 396 站点按 **match 密度**分桶，比较「耗尽 335」vs「全等 32」——若耗尽站点 match 密度**不超配**，则主刀**预测回绿 ~0、不应开工**；若超配才落地，exit 单列 195 新开测回绿数 + 32 全等站点 `component_sha256` 逐字节不变 + 词表 `source_sha256` 刷新 + ledger 联动重测（>8 min）。

## 5. 门状态（不变）

门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用——**全部维持**；M22 整体 **NO-GO**，本片**零门翻转、零生产代码改动、零限额变更、零落地**。证据等级 `internal-fixture`。

## 6. 改动文件

| 文件 | 改动 |
|---|---|
| `runner/sico-runner/tests/m22_fuel_profile.rs` | **新增** `#[ignore]` 判别测试 `the_match_quadratic_term_is_discriminated_from_body_scanning`（2 臂 `match_flat`/`match_one`），+约 120 行；本片唯一写者 |
| `docs/reports/m22-emitter-quadratic-design-2026-11-06.md` | **新建**（权威报告） |
| `docs/steps/STEP-0575-m22-emitter-quadratic-root-cause.md` | **新建**（本文件） |
| `selfhost/codegen.sico`、`selfhost/parser.sico` | **零改动**（只读，`sha256(codegen)` = `013f2d7e…` 未变） |
| `runner/sico-runner/tests/selfhost_compiler.rs`、`selfhost/match-component-coverage-v1.json` | **零改动** |

## 7. 复跑

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test m22_fuel_profile --no-run
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test m22_fuel_profile
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test m22_fuel_profile -- --test-threads=1 --nocapture --include-ignored the_match_quadratic_term_is_discriminated_from_body_scanning
& $cargo fmt --manifest-path .\runner\sico-runner\Cargo.toml -- --check
git diff --check
```

最近一次：`--no-run` exit 0（13.72s，无 warning）；默认 `2 passed; 0 failed; 2 ignored`；判别臂 `1 passed`（35.19s）；fmt 无 diff；`git diff --check` 干净。
