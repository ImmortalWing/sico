# STEP-0360 — M22: gw 机器两处潜伏 span 缺陷实锤 + fn_end 走线修复尝试（过脆回退，全净）

> - 状态：**reverted（全净）**——两处缺陷实锤、一个修复方案证伪；回退后 census
>   68/126、0 DIFF、histogram 与 STEP-0357 终值逐格一致、无残留
> - 里程碑：M22 compiler self-host（S4 lowering 片，诊断片）
> - 证据级别：internal-fixture / runtime-verified（真实 runner，Rust oracle 字节差分）

## 1. 实锤的两处潜伏缺陷（to_text 门的前置 blocker）

to_text 分支的 3 例 DIFF 经截体逐字节审查，根因是 **gw 机器的两处与
to_text 无关的潜伏 span 缺陷**（此前不可见因相关函数整体拒绝）：

- **缺陷 A（语句关键字跨命中）**：`gcursor = fn_start` 初始化 → let/set
  处理器 `find_text_from(src, "let", gcursor)` 对函数名含 "let" 的
  （straight_let_function_ir / chained_let_function_ir）命中**函数名内
  子串**，首个 write_local span start 偏 −188B（oracle 42178 vs guest
  41990 实锤）；
- **缺陷 B（fn_end 串内命中）**：epilogue `find_text_from(src, "end
  function", fn_start)` 对函数体含 `"end function"` 字符串字面量的
  （list_get_match_ir 等，其体内有 `find_text_from(src, "end function",
  fn_start)` 自身的源代码文本！）命中**字面量内部**，fn_end 偏短
  212B（46700 vs 50670 实锤）。

## 2. 修复尝试与证伪

- 缺陷 A 修复（gcursor 改从声明冒号后起）**单独验证有效**（write_local
  span 归位，diff 点从 355051 移至 355272 = 纯 fn_end 问题）；
- 缺陷 B 修复尝试：token 走线 helper（`gw_fn_end_offset`：从函数名真实
  偏移逐词 find_text_from 链走到 body_end−2 再搜 "end function"）——
  **证伪**：链对单字符/常见词（"i"/"n"/"end"）漂移，多函数模块中
  fn_end 频繁落到下一函数的 "end function"，census 31 例 DIFF；
- 全量回退净验证：68/126、0 DIFF、四格 histogram 与 0357 终值一致、
  构建绿、无残留。

## 3. 下片设计输入（缺陷 A/B 的正解方向）

- 缺陷 A 正解：保留 gcursor=fn_start 语义不变（已证改动面大），改
  let/set 处理器的**关键字定位**——`find_text_from` 后校验命中点是否
  词边界/等于 words[gi] 的 token 位置（或以 words[gi] 的 token 偏移
  直接取代字节搜索——需一词级 token→字节映射助手，类似 word_span_offset
  但从 gcursor 锚到 words[gi] 的单跳）；
- 缺陷 B 正解：在 `end function` 处理器处（gcursor 已越过体内全部字面量）
  惰性求 fn_end 并存入新变量，epilogue JSON 用之；init `gcur_re = gfn_end`
  （11837 行）的依赖另行处理（入口块 flush 的 re 恒等于 fn_end——可把
  入口块 re 也改为惰性/在首个结构处理器处回填）；
- 两片修复落地后，to_text 分支（0358/0359 已备）按同法重落地 +
  立即 census，预计一次解锁 CALL-ARGUMENT 大盘户。

## 4. 验证

- 回退后：census 68/126、0 DIFF；HIST CALL-ARGUMENT 36 / CALL-SHAPE 7 /
  EXPRESSION 14 / STATEMENT 1 与 0357 终值逐格一致；git diff --check 净。
- 本片未增减固件（无落地变更）。
