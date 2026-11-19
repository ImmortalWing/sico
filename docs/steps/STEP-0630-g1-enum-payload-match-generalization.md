# STEP-0630 — G1：match payload 绑定泛化（IR + lowering + codegen 三层）

状态：implementation complete locally（2026-11-19；G1 三个 slice 全部落地；
G2/G3 未开工）；未 commit/push；不翻任何 gate。
计划：[`docs/plans/language-v1-batch4-option-variant-match.md`](../plans/language-v1-batch4-option-variant-match.md)；
合同：[`RFC-0056`](../rfc/RFC-0056-language-v1-batch4-enum-payload-match-option.md) D1/D2/D3。

## 1. 内容

| 层 | 改动 |
|---|---|
| IR（slice 1） | `EnumVariant{name, payload}` + `Module.enums` 声明表（serde default 空表省略 ⇒ 冻结序列化字节不变）；verifier `Project` 泛化：Named 基类型命中 enums 表时按 `"<case>"`（单元数）/`"<case>.<index>"`（多元数）+ payload 类型相等校验，索引解析防 `select.0x` 绕过；record 路径（不在表内 Named）保持 name-unchecked（回归测试钉死） |
| lowering（slice 2） | **实测修正：哨兵共 4 处而非计划登记的 1 处**——`lower_match`、语句块 builder、RFC-0049 let-bound match 三条路由各有一份 ok/error-only 扫描 + spill 预检启发式。新增共享件 `resolve_payload_binding` / `scan_arm_payload_binding` / `arm_has_payload_binding` 全部泛化；`Definitions.variant_payloads` 元数据表（`from_declarations` enum walk 填充，与 `variants` 注册同作用域）；Result 路径发射逐字节保持原状（回归锚 `result_payload_binding_lowering_is_unchanged_by_the_generalization`）；enum 臂绑定臂级隔离（withdraw），legacy Result 路径行为不动 |
| codegen（slice 3） | `ScriptAbi.user_enums`（镜像 `register_user_records` 的 `register_user_enums`；**boundary `flat_ir_types` 不查此表** ⇒ 函数签名/包边界维持拒 user enum 的 v0 非目标）；`lower_local_types` enum local = tag + 最宽 case payload；Variant 值取**全并集宽度**（对齐 Result full-width 先例，inactive 槽构造时零填充）⇒ payload-less 值与 enum local 槽数一致；Variant/ReadLocal 布局注册 D3 裸 case 名 + 逐槽 field（现有键是全限定名 `Notice.Message`，Project 字段是裸 `Message`） |

## 2. 实测记录（全部本机 `target/debug/sico.exe` / runner）

- `cargo test -p sico-ir` 23/24（唯一红 = 先存 IND-003）；`cargo test -p sico-codegen-wasm` 全绿；两者 clippy `-D warnings` 净、fmt 净。
- e2e：新固件对 `tests/end-to-end/enum-payload-match.sico(+test.json)`（单绑定 set 臂 / 多绑定 Pair(a,b) / let-bound enum match 三形状）`sico test` **PASS**（`x|7:9|hello`）。
- 运行期：p8（Message 臂 stdout=`x` exit 0）、p9（Stop 臂 exit 1）、p10（多槽投影 7:9）、p11（let-bound `hello`）全部实测正确——STEP-0629 §4 登记的「运行期判别字正确性」开放项对该形状关闭。
- 探针冻结表更新（11/11 `FRONTIER_UNCHANGED`）：p3/p4 身份从 `lower:match-payload-binding` 移至 `codegen:non-scalar-parameter`（这两个 fixture 声明 enum 形参/返回，v0 边界拒为设计非目标；**臂级哨兵已消失**，瞬态形状 p8/p10/p11 转 accepted 并入库为回归锚）。

## 3. 实测修正（已回写 RFC-0056 / 计划）

1. **哨兵 4 处**（见上）——计划 §2/§4 的「一处哨兵」口径已更正。
2. **源侧 enum 名必须 snake_case**：`ui-event.click(...)` 在表达式/模式位置被词法为减号（E2031）；D5 改为 `enum ui_event`，经 `boundary_kebab` 映射 WIT 名。
3. **未限定 case 名与嵌套绑定在语义层先被 E3001 拦**（叉积完备性）——管线级拒绝身份如实登记（G1 不裁决嵌套语义，v0 非目标）。

## 4. 诚实收窄

- G2（Option/some/none）与 G3-R1（ui_event 宿主面约定）未开工；IR 层的 `Type::Option` + verifier 已预置（`Operation::Variant` 校验早接受 some/none）。
- p1/p2（enum 作函数返回/形参）两 profile 维持 `unsupported non-scalar parameter/result`——v0 非目标，非缺口。
- component-v0 profile 未 widened（本片只动 script-v0）。
- M22 guest 前端对新形状的拒绝面未复测（typed refusal 为可接受中间态，D6）。
- 全量 workspace 回归未跑（G3 收口片按判据 (5) 执行）。

## 5. 先存红（非本片引入，登记不修）

- `sico-ir` `all_thirty_three_invalid_cases_stop_before_ir_support_dispatch`（IND-003 parser recovery）与 `sico-cli` `check_matches_all_58_semantic_oracles` 红于同一根因：IND-003（`zero-indent-recovery-declaration.sico` 的 E1001 恢复路径）——已用 stash 证实改动前即红。
