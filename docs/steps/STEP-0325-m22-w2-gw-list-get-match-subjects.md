# STEP-0325: M22 W2 gw 机器接受 list.get match 主语

> - status: implementation complete locally / independent CI pending（GitHub 本机不可达，快照推送 gitcode `origin`）
> - phase: M22 W2 S3/S4（完成计划 A 队列；in-body match 第四主语族）
> - completed: 2026-09-30
> - evidence: internal-fixture，Windows x64 真实 runner（`1.98.0-x86_64-pc-windows-gnu`）

## Objective / Changes

- 新函数 `gw_list_get_subject_packed`：in-body `match sico.list.get(<列表>,
  <索引>)` 主语（列表限 `List[Text]` 参数——cell 列表按声明边界拒
  CALL-TYPE；索引为 参数/cell/字面量 U64），发射 `sico.list.get` intrinsic
  调用（ty = ok:string/error:NumericError，range=[receiver 起, 闭括号+1)，
  result id = base+前置指令数——与 map.get 打包器的 id 惯例一致）。
- `gw_match_open_pack`：`is_intrinsic_path(…,"list","get")` 检测 + 选包器
  第四分支 + ok_ty="string"。
- result-kind 映射补全：临时 cell kind 新增 `result-i64`/`result-string`
  （此前 `result`=ok:bytes / `result-u64` 二值——顺带修复 0324 引入的
  i64 in-body 主语临时 cell 类型潜在错配，0324 测试未覆盖 i64 in-body 故
  未暴露）。

## Measured results（本机真实 runner）

- L1 探针（if 分支内 list.get match + ok 臂嵌套 if）对 Rust oracle 字节
  全等；typed 拒绝（列表 cell 操作数 → CALL-TYPE、`sico.list.set` →
  CALL-TARGET）+ 恢复全等；套件 **57/57** 绿（共享 gw 路径无回归）。
- 下游推进：`find_body_end`/`count_params`/`count_until_end_function` 由
  CALL-TARGET 前移至 `ERR:E-SH-IR-CONTROL`（主语已过，卡臂内嵌套控制流
  组合——下一切片目标）；`matching_close` 维持全等。

## Honest residuals

- 本步无新增翻绿函数：parser 维持 **12/115**，按计划 §3.2 纪律
  **R3 连续停滞计数 1/4**（canary 零增长且前缀前沿未动）。
- CONTROL 缺口（臂内 match 后随语句/嵌套 if-else-return 组合）与
  EXPRESSION 缺口（text.encode/bytes.at let-RHS）不变；M22 NO-GO。
