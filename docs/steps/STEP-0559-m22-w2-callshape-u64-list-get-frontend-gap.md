# STEP-0559 m22-w2 **CALL-SHAPE 根因收窄：`sico.list.get[U64]` 结果在 guest 前端不可 match（负结果/能力缺口登记）**

| 项 | 值 |
|---|---|
| 片型 | **诊断片（负结果）**：把 `selfhost_compiler` 固件里 4 个 CALL-SHAPE 既有红中的 2 个（`step0519`/`step0520`，`cgs_value_ty`/`cgs_value_slot`）的根因从「嵌套调用作实参」**收窄**到「guest 前端无法 materialize + match `sico.list.get[U64]` 的 `Result[U64,_]`」。三种语法重塑逐层探测，**非收敛**（每剥一层露出下一个错误码）⇒ 判定为**前端能力缺口**，非 codegen.sico 语法重塑可修 |
| exit | **达成（诊断目标）**：确立 3 层错误递进（CALL-SHAPE → E2001 类型错 → GWPACK-OTHER）+ 与通过者 `cg_list_at`（Text）的对照；确认 `ERR:E-SH-IR-CALL-SHAPE` 不是嵌套实参引起（let 提升实参后仍 CALL-SHAPE）。**不修 codegen.sico**（实验已回退，`git checkout`，工作树 codegen.sico 无 diff） |
| 不翻门 | 门 1/5/6 GO、门 2 PARTIAL、门 3 NO-GO、门 4/7 不得沿用 —— 全部**维持**；本片为诊断，不改任何契约/语料/门 |
| 证据等级 | `internal-fixture`：本机 Windows x64，debug 测试构建，`--test-threads=1` 直跑 guest 前端 |

## 1. 起点（STEP-0558 §4.2 的 4 个 CALL-SHAPE 既有红）

STEP-0558 把固件失败判为「4 个 CALL-SHAPE = 既有红 + MS-225 ledger 行移位（后续项）」。本片聚焦其中 `step0519`/`step0520` 两个，其失败形如：

> `codegen function cgs_value_ty must compile through the guest frontend: Domain { code: "invalid-input", message: "ERR:E-SH-IR-CALL-SHAPE" }`

**开工假设（被本片否证）**：`cgs_value_ty`/`cgs_value_slot` 是 codegen.sico 里唯一在 `match` 头里用 `sico.list.get[U64](…)` 且**实参含嵌套调用**的函数；假设「嵌套调用作 match 头调用的实参」是 CALL-SHAPE 真因，let 提升实参即可修。

## 2. 探测（3 层错误递进，`selfhost/codegen.sico`）

对照**通过者** `cg_list_at`（`codegen.sico:532` 声明 / `:533` match 头，返回 `Text`）：

```sico
function cg_list_at(items: List[Text], index: U64) returns Text:
  match sico.list.get(items, index):   // 无类型实参，Text 重载，match 头直接调用 —— 通过
    case ok(cgl_item): return cgl_item
    case error(_): return ""
  end match
end function
```

对**失败者** `cgs_value_slot`/`cgs_value_ty`（返回 `U64`，列表 `List[U64]`）依次试三种形式，逐层探测：

| 形式 | `match` 头写法 | 结果 | 错误码 |
|---|---|---|---|
| **A（原状/committed）** | `match sico.list.get[U64](vindex, cgs_mul(cgs_decimal_u64(id), U64.literal(2))):` | FAIL | `ERR:E-SH-IR-CALL-SHAPE` |
| **B（let 提升实参 + 去类型实参）** | `match sico.list.get(vindex, cvs_at):` | FAIL | `E2001 …:2112:25 expected List[Text], found List[U64]` |
| **C（let 提升实参 + let 绑定调用结果）** | `let cvs_got = sico.list.get[U64](vindex, cvs_at)` 然后 `match cvs_got:` | FAIL | `ERR:E-SH-IR-GWPACK-OTHER` |

**逐层判读**：

- **A→B（去 `[U64]` 类型实参）**：错误从 `CALL-SHAPE` 变 `E2001 expected List[Text], found List[U64]` ⇒ guest 前端的 `sico.list.get` **非泛型形式被定型为 `List[Text]` 单态重载**（即 `cg_list_at` 用的那个），传 `List[U64]` 直接类型错。⇒ **`[U64]` 类型实参是必需的**，去掉不行。
- **B→C（保留 `[U64]`，但把调用从 match 头移到 `let` 初值，match 变量）**：错误从 `CALL-SHAPE` 变 `GWPACK-OTHER` ⇒ **`CALL-SHAPE` 的触发点是「`match` 头里出现带类型实参的 host 调用」本身**，与实参是否嵌套无关（B 已 let 提升实参、A 形式仍 CALL-SHAPE 即为证）；而把该调用挪出 match 头后，`let` 绑定 `Result[U64,_]` 又撞上 `GWPACK-OTHER`（guest 前端无法 pack/unpack 该 Result 局部量以供 match）。
- **非收敛结论**：三种形式分别命中 `CALL-SHAPE` / `E2001` / `GWPACK-OTHER` 三个**不同**错误层，无一层到达「可编译」。对照 `cg_list_at`（Text，无类型实参、match 头直接调用）通过 ⇒ 缺口**特定于 `List[U64]` 元素**：guest 前端**既不允许 match 头里放带类型实参的 `sico.list.get[U64]`（CALL-SHAPE），也不允许把其 `Result[U64,_]` let 绑定后 match（GWPACK-OTHER），且无类型实参则定型成 Text（E2001）**。即：**selfhost codegen 当前没有任何语法可在 `List[U64]` 上读元素并 match 其 Result**。

**与 STEP-0507 形状前沿的关系（一致 + 增量）**：STEP-0507（`handoff-m22.md` 形状前沿表）已确立 `match <host call>` ⇒ `ERR:E-SH-IR-CALL-SHAPE`、host 套 host ⇒ `ERR:E-SH-IR-GWPACK-OTHER`，并给出成例修法「**一律先过 cell**（`let` 取值 → `set` 拼接 → `return` cell）」。本片**一致**：A→C 正是按该成例把 `match <host call>` 改成 `let` 绑定后 match cell。**增量发现**：STEP-0507 的「先过 cell」成例是在**返回普通值**（text/binary）的 host 上验证的；而 `sico.list.get[U64]` 返回 **`Result[U64,_]`**，「先过 cell」在此**失效**——`let` 绑定该 Result 本身撞 `GWPACK-OTHER`（前端无法 pack 该 Result 局部量供 match）。⇒ 缺口不在「match 头形状」这一层（那已被 STEP-0507 记录、且有成例绕法），而在**下一层**：**Result 返回值的 host 调用结果无法在 guest 前端被 cell 化后 match**。这是 STEP-0507 形状前沿表未覆盖的**新前沿点**。

## 3. 判定与处置

- **根因收窄**：`ERR:E-SH-IR-CALL-SHAPE`（对这 2 个函数）= guest 前端不支持 `match <带类型实参的 host 调用>`；**不是**嵌套实参（否证开工假设）。更深层是 `List[U64]` 元素读取 + Result match 的**前端能力缺口**（`GWPACK-OTHER`/`E2001` 双向堵死）。
- **不修**：这是 guest 前端能力缺口，非 codegen.sico 语法重塑可解；继续在此打转会无限非收敛。本片**回退全部 codegen.sico 实验**（`git checkout selfhost/codegen.sico`，工作树无 diff），保持 STEP-0558 交付面干净（= `parser.sico` 修复 + ledger 收口）。
- **登记为能力缺口（不属本片修复范围）**：把「selfhost codegen 无法 match `sico.list.get[U64]` 结果」记入机器支持矩阵/已知缺口，归 M14/M15 前端能力项，**不得**在 M22 自举语境下声称 codegen.sico 这 2 个函数「可通过 guest 前端」。固件 `step0519`/`step0520` 维持**既有红**（能力缺口，非回归、非本系列引入）。
- **方向安全性**：本片为纯诊断，0 处 codegen.sico 落地改动、0 处契约/语料/门变更 ⇒ 对 verdict parity、ledger、七门**中性**。

## 4. 本片结论

- **2 个 CALL-SHAPE（`step0519`/`step0520`）根因已收窄并定性**：`List[U64]` 元素读取 + Result match 的 guest 前端能力缺口（3 层错误递进为证），非嵌套实参、非语法重塑可修。
- **处置**：回退实验、登记缺口、固件这 2 项维持既有红；不翻任何门。
- **剩余 CALL-SHAPE（`step0507`/`step0526`）**：同属 codegen 函数过 guest 前端的同类既有红，根因待后续片按同法逐个收窄（不在本片范围）。