# STEP-0166: M17 re-audit — image-vision@1 package, tetris recognition chain

> - status: complete — **audit verdict upgrade: M17 gate 1 GO；整体仍 NO-GO（gate 2/4 保持诚实登记）**
> - phase: M17 (M17 plan §3.2/§6; RFC-0043 implementation per STEP-0160's completion path)
> - completed: 2026-09-13
> - owners: autonomous-agent
> - artifacts: `image_vision_package()` in [`crates/sico-codegen-wasm/src/package_builder.rs`](../../crates/sico-codegen-wasm/src/package_builder.rs); content corpus `runner/sico-runner/tests/vision_package.rs` (5/5); consumer [`pilots/tetris-vision/`](../../pilots/tetris-vision/) + chain e2e `crates/sico-cli/tests/tetris_vision_chain.rs` (2/2)

## 1. What was done

**RFC-0043 v0 首批确定性 CV 包**（`sico:user/image-vision@1.0.0`，签名
`.sapp` 由 `generate_tetris_vision_package` 产出，与 csv/table-stats 同
一生产路径）——三个纯 wasm 发射的确定操作：

- `to-grey8`: BT.601 luma（固定点 `(299r+587g+114b)>>10`，exact-integer
  截断，i32 全程无溢出——1000 < 1024×255/255 保证）
- `threshold`: 按阈值（参数 2）二值化，逐像素 0/255
- `occupancy` / `occupancy-mask`: tetris 列检测原语——每列任一
  luma≥128 则点亮；mask 变体输出 ASCII `'0'/'1'`（消费者可 utf8 解码；
  v0 无 byte-at 内建的设计决定，记录于 RFC-0043 修订）

**Tetris 识别链 e2e**（M18 pilot 4 的消费者半环）：`board-reader.sico`
经 lock + 签名包 + user WIT 消费 `occupancy_mask`，对三个确定性
BGRA8 板面夹具输出 byte-exact 掩码（`01001010` 等），空捕获 typed
fail-closed。零 compiler/Runtime 补丁。

## 2. Defects found and fixed while landing

1. **lift 方向返回区 ABI**：包核心函数对聚合结果返回"被调方分配的返回
   区指针"，区内是**扁平结果元组** `(list_ptr, list_len)`——无判别式
   （12 字节判别式形式属于 lowering 方向）。首个内容契约测试暴露。
2. **帧平衡**：wasm-encoder 不自动追加函数级 `end`——vision 发射器显式
   收尾（现有 csv/table-stats 发射器本就手动 `End`，new emitter 漏了）。
3. **occupancy 死循环**：暗像素经 `br` 跳过了循环头的自增——重构为
   if/else 结构（暗路径空 arm，buffer 预置 '0'），自增在共享尾不再可跳。
4. **降低参数与输出重叠**：包的 bump 分配器从 heap_base=0 起步，而
   wasmtime 把 lowering 参数放进低端 guest 内存——输出写覆盖了输入
   （tetris 语料呈现为多出的列亮）。修复：heap_base 下限 64 KiB 预留
   （对全部包核心生效，一并消除 csv/table-stats 的潜在同类问题）。
5. **threshold 参数重载设计缺陷**：最初的"height 槽当阈值"使循环上界
   变成阈值——重构为 threshold(pixels, threshold, pixel_count) 契约
   （参数 2 = 阈值，参数 3 = 像素数 = 循环上界），RFC-0043 文本同步。

## 3. Gate updates (M17 plan §6)

| # | Gate | 之前 | 现在 |
|---|---|---|---|
| 1 | Deterministic baseline completes fixed corpus | NO-GO | **GO（首批）**：image-vision@1 三个操作 5/5 内容契约（字节级）+ tetris 链 2/2 e2e；RFC-0043 其余 roster（template match/grid/contour）仍待后续包 |
| 3 | Consumers don't modify compiler/Runtime | GO（保持） | tetris 消费者零补丁 |
| 5 | Resource corpora fail closed | GO（资源半区，保持+扩展） | 空捕获 fail-closed；截断输入 clamp 语义记录在案 |

整体 M17 维持 **NO-GO**：gate 2（加速 provider）与 gate 4（model RFC）
仍开放——诚实不变。

## 4. Declared limits

- 包 v0 值域限制：列索引/掩码以字节表达（板宽 ≤ 255 列；更宽的板面走
  RFC-0043 后续版本）。
- `occupancy-mask` 的阈值固定 128（mask 契约内定；需要自定义阈值的消费
  者用 `threshold` + `occupancy` 组合，或等 RFC 修订）。
- `to-grey8` 的截断语义 = exact-integer floor（RFC-0043 记录），非
  "approximate"。
