# M22 line_tokens 攻坚 — 交接手稿

> 状态：**未完成，停在 split_lines else 悬空语法错**。本文档交接给下一个 AI 继续。

## 目标
让 `line_tokens`（formatter 第 16 个函数）与 Rust oracle **byte-exact**，沿 M22 计划 §3.2 队列推进。这是高杠杆点——修通后 `source_has_lex_error`→`main` 一串函数大概率解锁。

## 当前 git 状态
- 提交 `8979063`（STEP-0256，已推送双远端）之前是干净的基线。
- **当前工作树有未提交改动**（selfhost/parser.sico 大改 + selfhost_compiler.rs 加了 dump_lt_region_ir 测试），**停在编译错误**：
  `sico: selfhost/parser.sico: unsupported else outside if at bytes 237334..237339`（约 line 5600）。

## 环境 / 快速迭代环
```bash
# 工具链（本机缓存是 msvc，gnu 缺 gcc/dlltool——见 STEP-0254 注记）
export RUSTUP_TOOLCHAIN='1.98.0-x86_64-pc-windows-msvc'
export PATH="$USERPROFILE/.cargo/bin:$PATH"
CARGO="$USERPROFILE/.cargo/bin/cargo.exe"

# 快速环（不重编 Rust 测试）：
./target/debug/sico.exe build --profile script-v0 --output /tmp/m22iter/compiler.component.wasm selfhost/compiler.sico
cat /tmp/m22iter/lt_prefix.sico | ./runner/sico-runner/target/debug/sico-runner.exe --fuel 5000000000 /tmp/m22iter/compiler.component.wasm > out.txt 2>err.txt
# lt_prefix.sico = formatter.sico 截到 "function source_has_lex_error" 之前
# oracle 目标：/tmp/m22iter/lt_expected.json（已 dump 好）
```
- oracle 蓝图已提取：line_tokens 52 块、locals `src..next` + `#match10/#matchword_bytes/#match12/#matchraw_bytes`，slice match 主体在 block 11/42。
- **迭代前必查回归**：`scan_space` 最小用例 `/tmp/m22iter/ss.sico` 必须 exit=0。

## 已完成（本轮改动，未提交）
1. **多 match 门放宽**（scalar_ir）：`match_count>1 && while==0 → CONTROL`；while 路径去掉 `match_count>0 → STATEMENT` 门。
2. **declared_return_kind** 识别 `List` → `"list"`（**回归风险**：append_pair 等 list-return 函数字节可能变，须全套回归确认）。
3. **gw NOIF 放宽**：while 函数无 if 不再 SKIP（NOIF 改为直通）。
4. **gw_rhs_packed split_lines 分支**（新增）：识别 `sico.text.split_lines("<str>")`，发 `const_string`+`intrinsic(list)` 两条指令。
5. **while_call_rhs_packed 嵌套 intrinsic 实参**：`inner(sico.bytes.utf8_decode(x))` 支持（含 `scl_width_override`）。
6. **nested_intrinsic_return_ir** 增加单层 `sico.bytes.utf8_decode` 白名单。
7. **gw match 区域**（大改）：`gw_match_open_pack` 助手 + `gmt_entries` 打包表 + 装配 match 终止符 + `#match` locals 特判。
8. **256 local 压缩**：gw 曾 252 let 超限（STEP-0249 约束），已把 gmt 表 11→1 个 Text 表 + pending 打包。

## 当前 blocker（else 悬空）
`gw_rhs_packed` 的 multi-RHS 分支结构（约 line 5535-5657）：
```
if gws_multi:                         # 5535
  if is_intrinsic_path(bytes,at):     # 5536 ... end match 5550
  else:                               # 5551
    if is_intrinsic_path(text,split_lines):   # 5552 —— SL 分支
      ... end match (SL value)
    # ← 这里缺 SL if 的 end if，或 else 配对错
    else:                             # 5600 —— **报错点：else outside if**
      let gws_second = ...            # dot/call 链
      ...
    end if / end if / end if          # 5653-5656
  end if                              # gws_multi else
else:                                 # 5657 gws_multi 的 else（单字路径）
```
Sico 对 `else / if / ... / else / ... / end if`（elif 风格多分支链）的 else 配对似乎有限制。SL if 的 end if 位置不对导致 5600 的 else 找不到匹配的 if。

**修复思路**（任选）：
- A. 把 SL 分支改成**自成一体的 if/else**（`if split_lines { ... } else { dot/call链 }`，独立的 end if），套在 bytes.at 的 else 里——避免三层 elif 链。
- B. 研究仓库里既有的多分支 intrinsic 识别（如 5536 bytes.at / lgm 的 list_get）怎么写 else 链，照搬其缩进/结构。
- C. 用 `git diff selfhost/parser.sico` 对照 5536 的 bytes.at 分支（能编译的参照），逐行对齐 SL 分支的 if/else/end if 布局。

## 验证 oracle 关键事实（已提取，不必重 dump）
- line_tokens block 11 主体：`read_local src/cursor/size → call sub(function 2) → intrinsic sico.bytes.slice args[0,2,sub_result]`，subject range = match 行 range。
- `#match10` local ty = `{"kind":"result","data":{"ok":{"kind":"bytes"},"error":{"kind":"named","data":"NumericError"}}}`，range = match 行 range。
- ok 臂：project + 溢出绑定 `#matchword_bytes`（bytes，case 行 range）+ set 体。
- error 臂：`const_string`（或参数回退）→ 跳 join。
- join 块 range = match 行 range；arms range = case 行 range。
- 第二处 match（block 42）同构，binding=`raw_bytes`，local=`#match12`。

## 下一步
1. 先修 else 悬空（上面 A/B/C），`scan_space` 回归 exit=0 后再继续。
2. line_tokens byte-exact 对照 `/tmp/m22iter/lt_expected.json`。
3. 全套回归：`$CARGO test --locked --offline --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler -- --test-threads=1` + 其它 selfhost 套件。
4. 记 STEP-0257 文档 + validate-step-0257.ps1 + STATUS/ROADMAP 同步，提交推送双远端（github 网络偶断，重试即可）。

## 纪律提醒
- 每步保持 refusal-first：不支持的形状要 typed 拒绝，绝不部分降级。
- parser.sico 接近 256 local 上限（STEP-0249），新增 let 优先复用/打包。
- 改 declared_return_kind / parameters_ir 等共享函数后**必须**跑全套回归（append_pair/item 等 list-return 函数字节可能变）。
