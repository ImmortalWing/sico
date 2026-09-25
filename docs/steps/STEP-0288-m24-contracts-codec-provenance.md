# STEP-0288: M24 合同起草——codec RFC-0050 + provenance RFC-0051（卡 24-A 合同面）

> - status: complete / two RFCs registered as **draft**; no implementation, no gate moved
> - phase: M24 prerequisite contracts (execution card 24-A contract surface)
> - date: 2026-09-25
> - evidence class: contract drafting over the STEP-0287 inventory and accepted contracts (RFC-0041, M7 discipline)

## 1. Scope and decision

续卡 **24-A**：起草 STEP-0287 盘点标出的四份前置合同中可起草的三份里
最具体的两份（第三份 = UI source-binding RFC + renderer ADR，面最大，
另行起草；加速 ADR 的证据本身被外部输入卡死，不属可起草范围）：

1. **[`RFC-0050`](../rfc/RFC-0050-image-codec-jpeg-png-v0.md)（draft）—
   image-codec@1 包**：`decode-jpeg/decode-png/encode-png/encode-jpeg`
   四个纯函数，`Image` record 复用 RFC-0041 的 BGRA8/ packed rows/
   top-left/stride=w·4 契约（与 image-vision@1 同一数据面）。接受
   profile 与编码参数**按名冻结**：JPEG 解码 = baseline SOF0/Huffman/
   8-bit/YCbCr(444,422,420)+灰度（progressive/算术/12-bit/CMYK/
   EXIF 方向拒绝或忽略）；PNG 解码 = 8-bit 四种 color type（Adam7/
   palette/16-bit/tRNS 拒绝）；encode-png = RGBA-8 + Paeth 滤波 +
   zlib 9 + 仅 IHDR/IDAT/IEND；encode-jpeg = SOF0 quality 90 4:4:4。
   固定输入输出**字节级稳定**；限额从 header 先检（limit+1 typed）；
   §8.5 拒绝面收进封闭 `CodecError`。消费走 M7，零 compiler/Runtime
   变更。
2. **[`RFC-0051`](../rfc/RFC-0051-model-provenance-manifest-v0.md)
   （draft）— M17 gate 4 provenance manifest**：封闭 manifest schema
   （name/kind/media/sha256/size/provenance/budgets/interface），SHA-256
   复用 M7 单一信任路径（安装期+加载期双验）；预算字段 typed 强制；
   fixture 模型五项证据集（digest 载入、坏 digest/截断/超限拒绝、
   预算拒绝、中途取消 typed、恶意资产拒绝）。gate 4 单独裁决，不推
   M17 总 GO；零 live-model 主张。

## 2. Grounding facts

- BGRA8 像素契约直读自 `package_builder.rs:1940-2143`（RFC-0041 注记：
  packed rows、top-left origin、stride = w·4、luma 公式）——codec 的
  `Image` 与 vision 包零转换衔接。
- SHA-256 hex 纪律直读自 `crates/sico-ecosystem/src/registry.rs`
  （`sha256_hex`、policy/event/policy 链字段）——manifest 不引入第二条
  信任路径。
- M24 计划 §8.4/§8.5/§8.2 的既有义务逐条映射进两份 RFC 的合同与
  证据集；无新增语法、无权限放宽。

## 3. Discipline

- 两份均 **draft**：无 owner 接受不写实现；实现 STEP 另行分配编号，
  并须把 §8.5 拒绝语料逐行落齐（RFC-0050）与五项证据集跑通真实
  runner（RFC-0051）。
- 本 STEP 不计 R3（合同起草）；M22/M23/M25/M26 不受影响。

## 4. Executable evidence

- 无 Cargo 工作区/runner 变更；`git diff --check` 与
  `validate-step-0124.ps1 -SelfTest` 通过；rfc/README 登记两条、
  下一可用编号推进至 RFC-0052。

## 5. Gate accounting

无 gate 变化。M24 = planned（盘点 + 四份合同中两份已起草、一份待起草
[UI 对]、一份外部卡死）；M17 = NO-GO 不变。owner 待决保持三项：W1
裁决方式、RFC-0048/0049 接受；新增可决项：RFC-0050/0051 接受。
