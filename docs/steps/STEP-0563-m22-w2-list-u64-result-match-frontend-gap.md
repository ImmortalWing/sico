# STEP-0563 m22-w2 **关闭 4 个既有红：guest 前端 `List[U64]` Result-match 能力片（诊断 + 根因收窄 + 最小复现夹具）**

| 项 | 值 |
|---|---|
| 片型 | **诊断片（根因收窄 + 能力缺口登记）**：复核 STEP-0559 把 `selfhost_compiler` 固件 4 个既有红（step0507/0519/0520/0526）定性为「guest 前端 `List[U64]` Result-match 三层堵死 / 前端能力缺口，推 M14/M15」的结论。**实测证伪其「类型层堵死」表述**：`List[U64]` Result-match **本身可降低**（`parser.sico:751` `u64_list_at` 全身体即用此形状、149/149 census 通过；本片夹具 H 全身体 `List[U64]` 泛型 match **LOWERED**）。真根因收窄到**单一行**：嵌套路径 `gw_list_get_subject_packed`（`parser.sico:13987`）要求 `get` 后一 token 是 `(`，泛型形式 `sico.list.get[U64](…)` 该 token 是 `[` ⇒ `ERR:E-SH-IR-CALL-SHAPE`。属**前端降低缺口**（Rust oracle 能降低全部 4 个函数 ⇒ 构造合法），**非语言面缺口 ⇒ 不触发 RFC-0033 门，不改语言** |
| exit | **达成（诊断目标）**：4 个红逐函数归因（step0507=`cgs_result_return_hex`、step0519=`cgs_value_ty`、step0520=`cgs_value_slot`、step0526=`cgs_function_instr_hex`，各为该钉列表里第一个拒者）+ 捕获真实失败输出 + 定位到 `parser.sico:13987` 单行根因 + 产出 14 个最小复现夹具（`target/tmp/step0563/*.sico`，其中 `I_if_list_u64_3param.sico` = 3 参数嵌套 `List[U64]` 泛型 match = cgs_value_slot/ty 同形状，`H_body_list_u64_3param.sico` = 同形状全身体 = LOWERED 对照）。**未改 `parser.sico`**（诚实收窄见 §5），`parser.sico`/`codegen.sico`/`selfhost_compiler.rs` 工作树 **0 diff** |
| 不翻门 | 门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用 —— 全部**维持**；M22 overall **NO-GO** 不变。本片纯诊断，不改任何契约/语料/门/语言 |
| 证据等级 | `internal-fixture`：本机 Windows x64，debug 构建，`--test-threads` 默认直跑 guest 前端；CLI 复现夹具经 `target/debug/sico.exe run selfhost/compiler.sico`（默认 fuel 预算） |

## 1. 起点：4 个既有红的真实失败输出（实测）

复跑命令（runner 独立 workspace，`RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-gnu`）：

```
~/.cargo/bin/cargo.exe test --locked --offline \
  --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- step0507   # 及 0519/0520/0526
```

实测输出（首次运行，退出码 1，逐个捕获）：

| 钉 | 第一个拒者（函数） | 真实诊断 |
|---|---|---|
| `step0507` | `cgs_result_return_hex` | `Domain { code: "invalid-input", message: "ERR:E-SH-IR-EXPRESSION" }` |
| `step0519` | `cgs_value_ty` | `Domain { code: "invalid-input", message: "ERR:E-SH-IR-CALL-SHAPE" }` |
| `step0520` | `cgs_value_slot` | （泛型 match 嵌套在 `if` 内，同 `cgs_value_ty` 形状） |
| `step0526` | `cgs_function_instr_hex` | （泛型 match 嵌套在 `while` 内） |

`git stash` 基线（STEP-0560 已实证）+ 本片复跑确认：这 4 个红是 **pre-existing 既有红**（对应函数 STEP-0502/0503/0506-0521 加入、钉 STEP-0507/0519/0520/0526 建立时即红），非本系列回归。

## 2. 复核 STEP-0559 定性：**「List[U64] Result-match 不可 match」被证伪**

STEP-0559 三层探测（A `match sico.list.get[U64](…)` → CALL-SHAPE；B 去类型实参 → E2001 Text 单态；C `let` 绑定 Result 再 match → GWPACK-OTHER）结论为「前端无法 materialize + match `List[U64]` 的 Result ⇒ 能力缺口」。本片用**位置对照**推翻其「类型层堵死」表述：

**决定性对照（`parser.sico` 自身即反例）**：`parser.sico:750-758` 的 `u64_list_at`：

```sico
function u64_list_at(values: List[U64], idx: U64, fallback: U64) returns U64:
  match sico.list.get[U64](values, idx):
    case ok(value):  return value
    case error(_):   return fallback
  end match
end function
```

——**完全相同的 `List[U64]` 泛型 Result-match**，`parser.sico` 经 guest 前端降低 **149/149 census 字节全等**（即此形状可降低）。`cgs_value_slot`（`codegen.sico:2118`）与其**唯一结构差异** = match 被**嵌套在 `if` 块内**。

## 3. 最小复现夹具矩阵（`target/tmp/step0563/*.sico`，CLI guest 前端实测）

判据：`sico run selfhost/compiler.sico` 输出含 `"schema":"sico.ir.v0"` = **LOWERED**；含 `ERR:E-SH-IR-*` = **typed 拒绝**。

| 夹具 | 形状（match 位置 × 元素类型 × 泛型实参） | verdict |
|---|---|---|
| `H_body_list_u64_3param.sico` | **全身体** × `List[U64]` × `[U64]` × 3 参 | **LOWERED** |
| `C_body_list_text.sico` / `G_body_generic_text.sico` | 全身体 × `List[Text]`（带/不带泛型） | **LOWERED** |
| `D_if_list_text.sico` / `F_if_generic_text.sico` | **嵌套 `if`** × `List[Text]`（无泛型实参） | **LOWERED** |
| `I_if_list_u64_3param.sico` | **嵌套 `if`** × `List[U64]` × `[U64]` × 3 参 | `ERR:E-SH-IR-CALL-SHAPE` |
| `J_nested_plainarg.sico` / `L_nested_generic_1param.sico` / `B_if_list_u64.sico` | 嵌套 × `List[U64]` × `[U64]`（实参/参数数变化） | `ERR:E-SH-IR-CALL-SHAPE` |
| `E_body_list_u64.sico` | 全身体 × `List[U64]` × **2 参** | `ERR:E-SH-IR-MATCH-SHAPE`（`list_get_match_ir:18945` 的 1/2/3 参门放行 2 参、但此夹具另踩形状；非本片根因） |
| `K_body_callarg.sico` | 全身体 × `[U64]` × **实参是用户调用** | `ERR:E-SH-IR-CALL-ARGUMENT`（STEP-0507 已记录的「call 实参不得是 call」，与本片根因正交） |
| `shallow_list_get.sico` / `nested_list_get.sico` | 2 参初版夹具 | MATCH-SHAPE / CALL-SHAPE（CLI 默认 fuel 下的噪声，**不作判据**） |

**判读**：`List[U64]` 泛型 Result-match **在全身体位置可降低**（H、且 `u64_list_at` 为活证）；**仅当嵌套进块（`if`/`while`）且带 `[U64]` 泛型实参**时拒 `CALL-SHAPE`。去掉泛型实参（`List[Text]`，D/F）则嵌套也可降低 ⇒ **判别式 = 嵌套 × `[U64]` 泛型实参**，非「List[U64] 类型」本身。

## 4. 根因（单行，代码证据）

嵌套 match 主语由 `gw_match_open_pack`（`parser.sico:15678`）路由。它**已支持** `list.get` 主语：`gmo_is_list_get = is_intrinsic_path(words, gmo_subject, "list", "get")`（`:15702`）→ `gw_list_get_subject_packed`（`:15777`）。`is_intrinsic_path`（`:1121`）只匹配 `sico . list . get` 四 token，**不消费其后的 `[U64]`** ⇒ 泛型形式照样路由进来。

缺陷在 `gw_list_get_subject_packed`（`:13958`）入口：

```sico
let glg_get_seg = next_index(glg_dot2, glg_n)   // "get"
let glg_open    = next_index(glg_get_seg, glg_n) // = "get" 后一 token
...
if is_word(word_at(words, glg_open), "("):       // :13987 —— 泛型形式此处是 "["
  ...
else:
  return "ERR:E-SH-IR-CALL-SHAPE"                // ⇒ 命中此行
```

对照**全身体**路径 `list_get_match_ir`（`:18945`）：它在 `:18984` 起**显式检查 `[`、读泛型元素类型（U64/Text）、再找 `(`** ⇒ 这正是 `u64_list_at` 与夹具 H 能降低的原因。

⇒ **根因 = `gw_list_get_subject_packed` 不消费 `[U64]` 泛型实参 token**。这是**前端降低缺口**（Rust oracle `lower_core` 能降低全部 4 个函数并产出 canonical IR，被 `rust_ir()` 用作期望值 ⇒ 构造合法），**不是语言面缺口**：不需改语言、不触发 RFC-0033 门。STEP-0559 判「非 codegen.sico 语法重塑可修」正确，但归因到「类型层不可 match」过宽——真缺口是**嵌套主语 packer 漏读泛型 token**。

## 5. 诚实收窄：本片为何**不落地** `parser.sico` 修复

修复落点明确且**窄**（`gw_list_get_subject_packed` 在 `:13964` 取 `glg_open` 后，若该 token 是 `[` 则扫到匹配 `]`、再取其后的 `(`；全身体 `list_get_match_ir:18984` 已有同形状可逐字节照搬），但本片**选择登记而非落地**，原因（如实登记，不包装为完成）：

1. **验证基建当前被占用**：`selfhost_compiler` 测试二进制 `selfhost_compiler-4fa1b02d73be6796.exe` 被其他蜂群代理的并发测试进程持锁（本片运行期间实测 PID 15508 / 30652），`rust-lld` 重链接报 `failed to write output … Permission denied` ⇒ 任何 `parser.sico` 改动**无法在本片内经 harness 复跑验证**（先红后绿的第二拍跑不成）。CLI 路径（`sico run compiler.sico`）对**整函数隔离**夹具撞默认 fuel 墙（§3 噪声行），只有**最小夹具**可用，不足以验证「4 个钉转绿 + 149/149 census + 全语料无回归」。
2. **热文件爆炸半径**：`parser.sico` 是全蜂群共享热点、且必须**自举可降低**（任何新增代码自身也要过 guest 前端形状前沿）+ 与 Rust oracle **逐字节等同**（149/149 census、全语料 component parity）。在无法跑 harness 的窗口里改它，一旦引入细微字节差或不可自举形状，会破坏全蜂群的 census 基线。
3. 按 §10.2「先写会失败的测试再写最小实现」+「没跑成的验证如实登记」：**宁可登记「本片测不了/未落地」，不落地一个未经验证的改动**。

⇒ 本片交付 = **精确能力缺口登记 + 最小复现夹具 + 单行根因 + 修复落点**，供下一个拥有干净验证窗口的实现片执行。

## 6. 4 个红的逐函数归因（供实现片逐个收口）

| 钉 | 函数 | 根因分类 | 修复落点 |
|---|---|---|---|
| `step0519` | `cgs_value_ty` | 嵌套 `if` 内 `match sico.list.get[U64]` | `gw_list_get_subject_packed:13987` 泛型 token（**本片根因**） |
| `step0520` | `cgs_value_slot` | 同上 | 同上 |
| `step0507` | `cgs_result_return_hex` | `ERR:E-SH-IR-EXPRESSION`（**不同**错误码，非 list.get 主语；函数体含深层 `if` 嵌套 + `cgs_value_slot`/`cgs_dispatcher_slot` 调用） | 待下一片按同法收窄（本片未逐层定位） |
| `step0526` | `cgs_function_instr_hex` | 嵌套 `while` 内 `match sico.list.get[U64]`（同根因族）+ 可能叠加深层 `set` 嵌套 intrinsic 右值（STEP-0491 记录的形状前沿） | `gw_list_get_subject_packed:13987`（+ 待核深层块 `set`） |

⇒ 收口这 4 个红**不需要语言变更**；`cgs_value_ty`/`cgs_value_slot`（可能含 `cgs_function_instr_hex`）经**一个** `gw_list_get_subject_packed` 泛型 token 修复即可转绿；`cgs_result_return_hex` 需另一次 `EXPRESSION` 层收窄。

## 7. 实测/未跑项（诚实边界）

**实测（本片真实执行，含退出码/关键输出）**：
- `cargo test … --test selfhost_compiler -- step0507/0519/0520/0526`：4 个红真实失败输出已捕获（§1），exit 1。
- `sico run selfhost/compiler.sico` × 14 夹具：判据矩阵 §3（LOWERED / typed 拒绝码逐个读出）。
- `python tools/validate-selfhost-comment-discipline.py`：`SELFHOST_COMMENT_DISCIPLINE_GREEN files=2`，exit 0（`parser.sico`/`codegen.sico` 未改，基线绿）。
- `git status --short` on `selfhost/parser.sico` / `selfhost/codegen.sico` / `runner/sico-runner/tests/selfhost_compiler.rs`：**0 diff**（本片未改这 3 个文件）。

**未跑 / 被阻塞（如实登记）**：
- `tools/probe-m22-frontend-frontier.ps1` 全量：后台跑 900 s **超时无输出**（每夹具起一次 guest ~25-40 s × ~70 夹具 ≫ 10 min）⇒ **未取得全量前沿读数**。本片改用 §3 定向夹具替代，**不等于**全量前沿探针通过。
- `parser.sico` 修复的先红后绿 + 4 钉转绿 + 149/149 census + 全语料 parity 回归：**未跑**（测试二进制被并发进程持锁，重链接 Permission denied；见 §5.1）。
- runner clippy / fmt：**未跑**（同上，重链接被阻塞）。

## 8. 对 RFC/ADR 的含义

- **不触发 RFC-0033（语言门）**：4 个红对应构造 Rust oracle 全部能降低 ⇒ 合法 Sico，缺口在 guest 前端降低器，属 §10.5/§10.6 的**前端能力片**（非语言面）。STEP-0559「归 M14/M15 前端能力项」的**处置方向**成立，但其「类型层三层堵死」**归因过宽**，应按本片 §4 收窄为「嵌套主语 packer 漏读泛型 token」。
- **ADR-0016（selfhost SoA 编译单元）decision 6（typed refusal / 无静默回退）不受影响**：当前 `CALL-SHAPE` 是 typed 拒绝，修复后应产出与 `list_get_match_ir` 逐字节相同的 canonical IR，不得引入静默近似。
- 建议实现片在 `tools/` 版本化一个**定向**前沿探针（嵌套 `List[U64]`/`List[Text]` × 泛型/非泛型 match 主语矩阵 = §3 夹具），把此判别式固化进前沿地图，避免下一个片重新踩 STEP-0559 的过宽归因。
