# Sico 啄木鸟视觉方案

这套概念把 Sico 的产品能力翻译成一只“精准诊断、快速修复”的啄木鸟：

- 短而明确的鸟喙同时构成代码尖括号，表达代码工具属性；
- 青蓝色胸翼形成连续的 `S` 曲线，继承原桌面图标的识别资产；
- 金色火花代表被定位的问题、诊断结果和成功修复；
- 深海军蓝维持开发者工具所需的专业感，圆润几何形让角色保持亲和。

## 色板

| 角色 | 色值 |
| --- | --- |
| Midnight Navy | `#07162F` |
| Electric Cyan | `#21DCE5` |
| Bright Teal | `#16BFCB` |
| Diagnostic Gold | `#FFB547` |
| Warm White | `#FFFFFF` |

## 文件

总览：

- `sico-woodpecker-icon-system.png`：六款图标总览；
- `sico-woodpecker-mascot-sheet.png`：六个吉祥物姿势和五种表情总览。

图标：

- `app-icon.png`：主应用图标；
- `avatar.png`：圆形社区头像；
- `diagnostics.png`：诊断 / bug 定位；
- `code-repair.png`：代码修复；
- `runtime-package.png`：包、组件与运行时；
- `monochrome-mark.png`：小尺寸单色标记。

吉祥物：

- `mascot-hero.png`：标准站姿；
- `mascot-bug-hunt.png`：定位问题；
- `mascot-code-check.png`：检查代码；
- `mascot-success.png`：任务完成；
- `mascot-thinking.png`：分析与思考；
- `mascot-avatar.png`：半身头像；
- `mascot-expressions.png`：表情条。

## 建议用法

- 主应用图标优先使用 `app-icon.png`，16–24 px 场景改用 `monochrome-mark.png`；
- 文档的错误诊断、自动修复和运行时模块分别使用对应的功能图标；
- `mascot-hero.png` 用于首页、发布公告和会议物料；其余动作只在对应功能语境中使用；
- 当前文件是品牌概念阶段的栅格稿。进入正式发布前，建议以这些几何轮廓重绘 SVG，统一曲线、网格、留白和小尺寸像素对齐。

## 生成方式与提示词摘要

使用 Codex 内置 imagegen 生成，原 `assets/desktop/sico-desktop-host.ico` 作为色彩和材质参考。

图标提示词：为 Sico 设计六款一致、可矢量化的啄木鸟图标，分别覆盖主应用、头像、诊断、代码修复、运行时包和单色小尺寸标记；保留深海军蓝、青蓝 S 曲线和金色诊断火花；鸟喙形成代码尖括号；使用暖白背景的 3×2 网格；避免文字、写实羽毛、攻击性和幼稚剪贴画风格。

吉祥物提示词：把图标角色扩展为完整啄木鸟吉祥物，保持相同短尖括号鸟喙、青蓝 S 形胸翼、双层冠羽和金色诊断火花；输出标准站姿、定位问题、检查代码、成功、思考、头像及五种表情；采用专业扁平矢量角色设计；避免服装、人手、复杂背景、猛禽感和过度 Q 版比例。
