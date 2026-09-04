# 俄罗斯方块消除脚本

## Sico 实现结论（2026-09-04）

已实际尝试用当前仓库的 Sico `0.0.2-dev` 承载本案例。`sico/board_input_probe.sico` 可以编译并运行，证明 Sico 当前能够完成 UTF-8/JSON 输入边界检查；但完整求解器目前不能诚实地迁移到 Sico：

- 棋盘求解需要通用循环/递归、动态集合遍历、位运算或可变搜索状态，当前 Script codegen 只覆盖受限的 Text/Bytes/List/JSON/文件/HTTP intrinsics 和特定控制流；
- 截图、像素/图像处理、窗口查找和鼠标拖动没有 Sico Host capability；
- M5 的 UI 是严格 companion model，尚无 compiler-facing Sico binding；本案例已进入路线图验收链：M14 纯 Sico 离线求解器、M16 capture/input、M17 vision、M18 完整应用试点，当前均未预留实现 STEP。

因此当前可运行实现仍由 Python 承担视觉、求解和控制。Sico 探针可这样验证：

```powershell
$repo = Resolve-Path ..\..
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build --locked --release `
  --manifest-path "$repo\runner\sico-runner\Cargo.toml"
$env:SICO_RUNNER = "$repo\runner\sico-runner\target\release\sico-runner.exe"
Get-Content -Raw .\sico\probe_input.json |
  & "$repo\target\release\sico.exe" run .\sico\board_input_probe.sico
```

预期输出：

```text
board input accepted; solver backend is not available in Sico yet
```

这是一个按职责拆分的微信小程序方块消除原型：

- `gamebot/model.py`：棋盘、图形、放置和消除规则。
- `gamebot/solver.py`：当前托盘穷举、局面评分和可选的随机后续评估。
- `gamebot/vision.py`：截图中的棋盘、目标宝石和底部图形识别。
- `gamebot/control.py`：Windows 微信窗口与 Android ADB 的截图、拖动实现。
- `gamebot/runner.py`：识别 → 规划 → 执行一步 → 校验的安全循环。
- `gamebot/cli.py`：命令行入口。
- `tests/`：不依赖 OpenCV 的后端算法测试。

## 为什么不用神经网络

棋盘和图形的几何结构固定，图像识别优先使用网格采样、颜色分类和轮廓检测。决策采用当前三个图形的完整搜索。下一批未知时，用“未来图形可放置数量”衡量棋盘弹性，也可以根据实际出现频率执行 Monte Carlo rollout。只有更换皮肤后传统识别不稳定，才值得用小型分类模型替换 `vision.py` 中的单格分类器。

## 安装

建议 Python 3.11～3.13：

```powershell
cd E:\github\sico\案例项目\俄罗斯方块消除
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -r requirements.txt
Copy-Item config.example.json config.json
```

## 先测试后端算法

```powershell
python -m unittest discover -s tests -v
python -m gamebot.cli demo
```

## 调试截图识别

项目中的示例配置已经按用户提供的 `1260 × 2720` 截图估算了棋盘和托盘区域：

```powershell
python -m gamebot.cli inspect `
  "C:\Users\64473\AppData\Local\Temp\codex-clipboard-6513b4bf-15c1-40b0-acd1-3955e304a64f.jpg" `
  --config config.json --debug debug.png
```

终端会打印识别出的棋盘和三个图形，`debug.png` 会标出格子、目标类别和方块中心。不同手机比例或游戏皮肤需要调整 `config.json` 中的归一化区域、亮度阈值和轮廓尺寸。

也可以先从正在运行的微信窗口或 Android 设备取一张校准图：

```powershell
python -m gamebot.cli capture --mode windows --config config.json --output capture.png
python -m gamebot.cli inspect capture.png --config config.json --debug debug-capture.png
```

## 运行方式

默认只截图和规划，不操作鼠标：

```powershell
python -m gamebot.cli run --mode windows --config config.json
```

确认识别与落点都正确后才能增加 `--execute`：

```powershell
python -m gamebot.cli run --mode windows --config config.json --execute
```

Android 调试设备可以使用：

```powershell
adb devices
python -m gamebot.cli run --mode adb --config config.json
```

Windows 模式按标题查找微信窗口；ADB 模式对设备执行 `screencap` 和 `input swipe`。一次只放一个图形，等待画面稳定后重新识别，避免消除动画导致连续误操作。

## 需要校准的地方

1. `board_rect`：棋盘在截图中的 `[x, y, width, height]`，范围为 0～1。
2. `tray_rects`：三个候选图形区域，同样使用归一化坐标。
3. `occupied_delta`：格子亮度比空格背景至少高多少才算占用。
4. `piece_cell_ratio`：底部单个小方块边长与屏幕宽度之比。
5. `release_offset`：拖动释放点修正。小程序若在拖动时把图形抬高，需要调整 Y 值。

`solver.future_samples` 默认为 `0`，使用快速的未来兼容性评分。校准完成后可设为 `16`～`64` 开启随机后续批次评估；数值越大决策越稳，但规划更慢。脚本在看到完整三件托盘时会在线更新图形出现频率。

脚本仅应在个人测试、无竞争和服务条款允许的场景使用。
