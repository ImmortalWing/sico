# Language v1 batch 4 — enum payload match + Option（计划）

状态：draft（2026-11-19 登记）；**阶段 1 完成：RFC-0056 草案已入库**
（`docs/rfc/RFC-0056-language-v1-batch4-enum-payload-match-option.md`，
**accepted**，owner 指令 2026-11-19「RFC-0056 接受」；README 已登记，下一可用号 RFC-0057）；
依赖：STEP-0628（ui@1 源级绑定的 poll_event §1 residual）；
不阻塞 STEP-0628 收口——STEP-0628 以 Outcome B（codegen 侧 Text 解码）先行落地，
本批落地后按 D7 回收 Text 面。归属：RFC-0056（`docs/rfc/README.md` 下一可用号），
实现 STEP 号自 0629 起按共享序列顺取（最新占用 0628）。

## 1. 背景与动机

`wit/script-ui-v1/ui.wit:83` 的 `poll-event: func() -> option<ui-event>` 在源语言不可表达：
Sico match 的 case 名只有 `else`/`ok`/`error`/`Enum.variant`/`true`/`false`
（`crates/sico-semantics/src/lib.rs:2865-2916`），且无 `Option` 构造表达式
（STEP-0494 fact-fix：`some(...)` 报 `E2031`）。STEP-0628 以 codegen 侧 Text 解码
（`UI_EVENT_TEXTS`，`crates/sico-codegen-wasm/src/canonical.rs:140`）绕过，
代价是 guest 失去静态类型且当前实现丢失事件 payload（handle/index 未读，
`crates/sico-codegen-wasm/src/lib.rs:4254-4348`）。本批闭合该语言缺口。

## 2. 现状实测基线（STEP-0629 实测钉死，2026-11-19；`docs/steps/STEP-0629-language-v1-batch4-frontier-probe.md`）

| 环节 | 状态 | 证据 |
|---|---|---|
| enum payload 声明解析进语义模型 | ✅ 已存在 | `build_enum_variants`，`crates/sico-semantics/src/lib.rs:789-828`（`VariantDefinition{name, payload: Vec<Type>}` :137-140） |
| 表达式位置带 payload 的 variant 构造 lowering | ✅ 已存在 | `crates/sico-ir/src/lower.rs:3850-3865`（`definitions.variants` → `Operation::Variant{name, payload}`）；STEP-0629 p1 实测 lower 通过、**发射器在函数返回边界拒** `unsupported non-scalar result` |
| match 模式文法解析 payload 臂（含嵌套） | ✅ 已存在 | `parse_pattern`，`crates/sico-ir/src/lower.rs:4053-4075`；**嵌套臂语义层按叉积展开**（STEP-0629 p4b：`E3001 missing case: Wrap.Box\|Notice.Message, …`） |
| match 臂 payload **绑定** lowering | ❌ 哨兵拒绝 | `crates/sico-ir/src/lower.rs:2955-2994`：仅 `ok`/`error` + `Type::Result` subject 放行（:2960、:2991），其余 `unsupported("match payload binding")`；STEP-0629 p3/p4 实测逐字 `unsupported match payload binding at bytes …` |
| `Option[T]` 类型 | ⚠️ IR 有、源面无构造 | `Type::Option`，`crates/sico-ir/src/lib.rs:78`；`some`/`none` 不存在——STEP-0629 p5 实测 `E2031 unresolved call target some`（与 STEP-0494 fact-fix 一致） |
| IR enum 声明表 | ❌ 不存在 | `Module` 有 `records`（RFC-0047 D4）无 enums；verifier project 校验硬编码 `"ok"`，`crates/sico-ir/src/lib.rs:865-869` |
| lowerer payload 元数据 | ❌ 不存在 | `Definitions.variants` 只存 `"Enum.case" → Named`，`crates/sico-ir/src/lower.rs:41-56`、:505-513 |
| 发射器对 enum 类型**函数形参/返回** | ❌ 两 profile 同拒 | STEP-0629 p1/p2 实测 `unsupported non-scalar parameter/result` |
| 发射器对**瞬态** enum 值（不跨函数边界） | ✅ 已可用 | STEP-0629 p7 实测内联构造 + 通配臂完整构建成功（1294 B） |
| guest 前端（selfhost）对上述形状 | ❌ typed refusal | 可接受中间态（D6），本计划零 selfhost 热文件改动 |

## 3. 设计决策（RFC-0056 的冻结内容）

- **D1 match payload 绑定泛化**：subject ∈ {`Result`（现状保持）、`Named` enum、`Option`}；
  臂按声明 payload 元数/类型逐槽校验；支持多绑定（`case select(h, i)`）。
  **嵌套模式的语义须先裁决**（STEP-0629 p4b 实测语义层按叉积展开 + E3001 完备性）：
  叉积展开 vs 分层解构二选一，写入 RFC 后 lowering 才放行嵌套绑定臂。
- **D2 IR schema 增 enum 声明表**：镜像 RFC-0047 D4 的 `records` 先例
  （`enums: BTreeMap<String, Vec<EnumVariant{name, payload: Vec<Type>}>>`，
  `serde(default)` 空表 ⇒ 冻结序列化字节不变）。
- **D3 project 字段约定**：单元数 payload = `"<case>"`（对齐 `"ok"` 现状）；
  多元数 = `"<case>.<index>"`；verifier 对照 D2 表校验类型。
- **D4 `Option[T]`**：Generic 语法（`List[T]` 先例）；`some(expr)`/`none` 构造走
  expected-type 约束（对齐 `unconstrained ok` 拒绝模式）；
  match 臂 `some(x)`/`none`。**完备性跟 Result match 先例**（语义层不强制
  sealed-exhaustiveness，v0）；`some`/`none` 为关键字 token（`some` 新增
  `SomeKeyword`；`none` 复用既有的 `TokenKind::None`——它与 effects/capabilities
  空列表语法 `effects: none` 共存，管线两侧各自识别，实测无冲突）。
  **v0 生产者边界**：用户函数签名不跨 `Option`（与 user enum 同规则，
  `non-scalar parameter/result` 拒）——guest 侧生产者 = host intrinsic
  （`sico.ui.poll_event`，G3-R1）；`some`/`none` 在无名期望位（如 let-bound
  match subject）保持 `unconstrained some/none` typed 拒。
- **D5 宿主面 enum 约定（R1）**：程序内按钉死形状声明
  `enum ui_event { case click(U64), case select(U64, U64), case closed }`；
  语义层对照形状表逐字校验（名/序/payload 类型），不符 typed 拒绝；
  调用 `poll_event` 而未声明 ⇒ typed 拒绝（unresolved type）。
  **命名修正（STEP-0630 实测）**：源侧名必须 snake_case——连字符
  （`ui-event`）在表达式/模式位置被词法为减号，实测构造调用报
  `E2031`；`ui_event` 经 `boundary_kebab` 映射到 WIT 的 `ui-event`。
  `sico.ui.poll_event` 源签名改 `Option[ui_event]`
  （`crates/sico-ir/src/lib.rs:1661` 的 intrinsic 表）。
- **D6 自举涟漪登记**：新形状在 guest 前端支持前 = typed refusal；
  corpus/账本/emitter-vocabulary 不同步声明。**本计划不碰
  `selfhost/parser.sico` 与 `selfhost/codegen.sico`** ⇒ 零热文件改动、
  不触发 ledger/vocabulary 重冻结。
- **D7 STEP-0628 Text 面回收**：`emit_ui_poll_event`
  （`crates/sico-codegen-wasm/src/lib.rs:4254-4348`）改 typed 发射——
  复用 `UI_EVENT_RESULT_SIZE`（`canonical.rs:118-123`）的 canonical ABI 读取，
  把 `some/none` 判别与 payload 绑进 guest 局部；`UI_EVENT_TEXTS` 表退役；
  `runner/sico-runner/tests/ui_source.rs` transcript 改带 handle/index 的
  typed 断言；STEP-0628 文档补「已回收」注记。`ui_instance()` 与
  `wit/script-ui-v1/ui.wit` 字节不动。

## 4. 执行切片（STEP 号顺取 0629+）

| 片 | 内容 | 出口证据 |
|---|---|---|
| STEP-0629 | 前沿探针（纯测量，零生产代码）：P1 构造 / P2 无绑定臂 / P3 带绑定臂 / P4 多 payload+嵌套；探针脚本版本化进 `tools/` | **measurement complete（2026-11-19）**：8/8 `FRONTIER_UNCHANGED`；新增 p6 对照锚 + p7 瞬态形状；三点实证修正见 §2/§6 与 [`STEP-0629`](../steps/STEP-0629-language-v1-batch4-frontier-probe.md) |
| STEP-0630 | G1：Definitions payload 元数据表（挂点 `lower.rs:505-513`）；哨兵泛化（`lower.rs:2955-2994`）；`Module.enums` + verifier 泛化（`lib.rs:865-869`）；单测 + e2e `.sico` fixtures + 拒绝边界 | **implementation complete locally（2026-11-19，三 slice 全落地）**：IR 层（`Module.enums`+verifier 泛化，serde default 冻结序列化不变）；lowering 层（**实测哨兵 4 处全泛化**，共享 `scan_arm_payload_binding`/`resolve_payload_binding`/`arm_has_payload_binding`，Result 字节级回归锚）；codegen 层（`user_enums` 内部并集布局、Variant 全宽值、D3 裸 case field、`apply_enum_fields`）。证据：`cargo test -p sico-ir` 23/24（唯一红=先存 IND-003）、`cargo test -p sico-codegen-wasm` 全绿、clippy/fmt 净、e2e `enum-payload-match` PASS、运行期 p8/p9/p10/p11 四形状实测、探针 11/11（p3/p4 身份移至 `non-scalar-parameter` 并登记）。详见 [`STEP-0630`](../steps/STEP-0630-g1-enum-payload-match-generalization.md) |
| STEP-0631 | G2：`some`/`none` 构造（模板 `lower.rs:3778-3821`）；`parse_match_pattern` 加 case 名；exhaustiveness 完备规则；codegen Option 布局（判别字+payload 槽） | **implementation complete locally（2026-11-19）**：lexer `SomeKeyword`（`none` 复用既有 `None`）；语义推断+case 名+Result 先例完备性；lower 三路由 `PayloadBinding::Some`；verifier Project 补 Option 臂（e2e 抓出）；codegen Option local + `apply_option_fields`。单测 3 新、探针 11/11、clippy/fmt 净。详见 [`STEP-0631`](../steps/STEP-0631-g2-option-some-none.md) |
| STEP-0632 | G3-R1：D5 形状表 + 校验；intrinsic 表 poll_event 签名；`emit_ui_poll_event` 按 D7 重写；ui_source.rs 转 typed | **implementation complete locally（2026-11-19）**：E2028 语义形状校验（catalog 登记）；发射重写 + `UI_EVENT_TEXTS` 全退役（源码零命中）；嵌套判别字布局修正（e2e 抓出）；runner `ui_source` **6/6 绿**（click(5)/select(6,2)/closed/none 携带 handle/index）；探针 13/13。详见 [`STEP-0632`](../steps/STEP-0632-g3-r1-ui-event-host-enum-convention.md) |
| STEP-0633 | 收口：全量验证 + 文档同步 | **closure complete locally（2026-11-19）**：fmt/clippy 绿；根工作区 467/6（6=先存 IND-003 族）；runner 工作区 467/6（同六件，EDR 关闭 + 磁盘清理 + 运行器二进制/SICO_RUNNER 三层解除后实测）；matrix/语料/STATUS/ROADMAP/steps README 同步；selfhost 零改动证实。详见 [`STEP-0633`](../steps/STEP-0633-language-v1-batch4-closure.md) |

## 5. 验证

```powershell
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
& $cargo fmt --all -- --check
& $cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
& $cargo test --locked --offline --workspace
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml
```

外加：machine support matrix 与 backend-refusal corpus 更新（AGENTS §5）、
semantic-cases 增补、新 typed refusal 身份登记。

## 6. 风险与未证实点（各片首任务核掉，不追认）

1. ~~named-enum payload 值的 codegen 运行时布局与 match 判别字读取~~
   **已由 STEP-0629 实测部分回答**：发射器拒绝 enum 类型的函数形参/返回
   （两 profile 同形），但**瞬态 enum 值完整走通**（p7 构建成功）⇒ G1/G3 的
   发射器工作收窄为「match 判别 + payload 投影局部布局」；残余未证实点 =
   **运行期判别字取值的正确性**（`emit_match_condition`，
   `crates/sico-codegen-wasm/src/lib.rs:2530` 附近）——G1 实现片的第一道测试任务。
2. 构造侧语义对 payload 元数/类型的校验存在性（只追到 lowering）。
3. ~~RFC-0052 在 dev 仍为 draft~~（**2026-11-19 更新：RFC-0056 已获 owner 接受**，
   D5 宿主面 enum 约定随接受冻结为合同；RFC-0052 本身的 A3 裁决仍开放，若 owner
   日后裁 A3（宿主侧事件），D5 形状表按 RFC-0056 的修订流程重裁）。

## 7. 边界（全程有效）

不 commit/push；不翻任何 gate；不改冻结工件（语料/账本/帧语料）；
`wit/script-ui-v1/ui.wit` 与 runner `ui.rs` 字节不动；不碰 `selfhost/` 两个热文件；
RFC-0056 的 owner 接受裁决不在本计划范围（草案入库即可）。
