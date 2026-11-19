# RFC-0055 Amendment A1: picker 返回根相对路径 + `outside-grant`（ACCEPTED）

> - status: **accepted by owner directive 2026-11-06「都走推荐项推进」**（M24 计划 §9.10 决策 D5）
> - port: **ported to `dev` from `codex/m24-vision-ui-v2` `6274ae6` under owner porting
>   authorization (2026-11-18)**；记录为 **accepted-contract 但明确尚未实现**（codex 上仅
>   合同、无实现；独立后续 STEP，不随基础 picker 端口落地）。
> - date: 2026-11-06
> - amends: [`RFC-0055`](./RFC-0055-native-ui-file-open-save-picker-v0.md) §3（返回值形态）、§4（权限）
> - 依赖：`sico:script/fs-read@` / `fs-write@`（`wit/script-profile-v0/world.wit`）、RFC-0052 A1
> - 不移动任何 §3 gate：本修订只冻结**合同**，实现与证据由后续 STEP 落地

## 1. 为什么需要本修订（实测缺口）

RFC-0055 §3 冻结"对话框返回路径、但不做 I/O"。但实测的文件能力路径解析
（`runner/sico-runner/src/lib.rs`）是：

- `scope_path`：**拒绝绝对路径**（Windows 盘符/`:`/反斜杠分量一律
  `path escapes the granted scope`），路径必须是**相对分量的正斜杠路径**；
- `resolve_read`：再做一次**规范化后的根内包含**检查（符号链接逃逸被拒）；
- `resolve_write`：写侧还要求**父目录已存在**于写根内。

而原生对话框（`IFileOpenDialog` / `IFileSaveDialog`）返回的是**绝对 Windows 路径**
（如 `C:\Users\...\a.png`）。因此：

> **RFC-0055 接受的"用户选择路径"与现有文件能力接受的"根相对路径"之间没有桥，
> M24 §9.4 的试点应用无法读取用户所选文件。**

Sico 语言面也无路径原语（无 basename / relative_to / join），应用侧无法自行换算。

## 2. A2 — 返回值形态（修订 §3）

- `open` / `save` 返回的路径改为**相对已授予根的路径**，分量以 `/` 分隔
  （与 `scope_path` 接受的形态一致）。
- **对话框的初始目录**设为本次运行的**读根 / 写根**（第一个已授予根）；对话框
  仍允许用户导航到根外，但根外选择的结果按下条处理。
- 选中项**不在任何已授予根内** → 返回**新增 typed case** `outside-grant`
  （`picker-error` 追加式扩展，不改动既有 case 的顺序与语义）。
- 取消仍然是**空 ok 字符串**（STEP-0557 既有编码），与本修订无冲突。

## 3. A3 — 权限（不变的部分必须写清）

- 对话框**仍然只做路径选择**，不做任何 I/O；**不授予**任何读写权限。
- 读写授权**仍然只来自** `fs-read` / `fs-write` 的既有根授予；返回相对路径
  **不放宽**任何权限——文件能力仍会独立做根内包含检查。
- 本修订**不新增**捕获 / 输入 / 剪贴板 / 凭据 / 进程权限（M16 不变）。

## 4. 落地面

- 宿主侧（runner picker seam + `crates/sico-ui-d2d` 的 `picker.rs`）：把原生绝对
  路径换算为**相对第一个匹配已授予根**的 `/` 分隔路径；不匹配任何根 → 映射为
  `outside-grant`。
- guest 侧：错误 case 集合追加 `outside-grant`（case 名为文本，沿用
  `picker-error` 的既有编码路径）；`cancelled` 语义不变。

## 5. 出口测试

1. 授予根内选择 → 返回 `/` 分隔的根相对路径，可直接被 `sico.fs.read` 接受。
2. 授予根外选择 → `outside-grant`（**不返回**越界路径）。
3. 取消 → 空 ok 字符串（既有语义不变）。
4. 未授权 picker → `authority-denied`（既有语义不变）。
5. 符号链接逃逸仍被文件能力拒绝（既有 `resolve_read` 行为不变）。

## 6. 不变量

- 本修订只改**路径形态**与**一个新增 typed case**，不新增权限、不新增原语。
- 本修订**不移动** M24 §3 的任何 gate。
