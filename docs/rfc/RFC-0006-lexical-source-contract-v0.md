# RFC-0006: Lexical and source contract v0

> - status: accepted
> - date: 2026-07-15
> - authors: autonomous-agent
> - target language/platform version: M1 draft
> - supersedes: -
> - superseded-by: -

## Summary

Sico M1 源文件采用无 BOM 的严格 UTF-8，位置事实源是 0-based UTF-8 byte 半开区间；标识符采用 Unicode 17.0 `XID_Start`/`XID_Continue` 并要求 NFC，换行接受 LF/CRLF，trivia 仅含水平空白、换行和 `//` 行注释；本 RFC 同时固定 M1 token 边界、无损覆盖不变量与保守输入限额，但不定义 parser 或静态语义。

## Problem

RFC-0005 已选择 B labeled-block，却刻意没有回答文件编码、中文标识符、Unicode 规范化、换行、注释、literal、span 和资源上限。若 lexer 先实现，这些选择会隐藏在 Rust 分支、正则或依赖版本里，造成同形名称、跨平台跨度漂移、formatter 丢 trivia 和不受限输入。

本 RFC 的 corpus 基线是 208 个 `.sico` 文件：全部可严格解码为无 BOM UTF-8，实测 3979 个 LF、0 个 CRLF、0 个 bare CR；54 个 B canonical case 使用 `//`、ASCII 标点、十进制整数和简单双引号字符串，但没有 Unicode identifier。缺少样本不等于禁止 Unicode，因此本 RFC 为 Unicode、新换行形态和失败输入增加独立 contract case。

## Goals and non-goals

目标：固定 source bytes 到 token/span 的确定映射；允许中文等国际化名称；拒绝静默规范化和不可见控制；保证 lossless tree 可重建有效输入；为 hostile input 提供硬上限；让 STEP-0016 可以实现而无需再做语言选择。

非目标：不定义名称相等、作用域、关键字在 grammar 中的位置、数值类型/溢出、字符串运行时表示、缩进语义、formatter 布局、E1xxx 正式 code、parser recovery、类型检查或 Component ABI。

## Source model and spans

### Encoding and file identity

- `.sico` 文件和 stdin 必须是严格 UTF-8；UTF-8 BOM `EF BB BF` 不允许，其他编码不做猜测或回退；
- 解码失败时仍保留原始 bytes 以报告权威 byte range，但不运行 lexer/parser；每个最大非法 UTF-8 子序列在派生 line/column 时作为一个 U+FFFD 计数；
- U+0000、除 TAB/LF/CR 外的 C0 control、U+007F 和 Unicode 17.0 `Default_Ignorable_Code_Point` 在源码任何位置均拒绝；CR 只允许作为 CRLF 的首字节；
- source identity 由编译会话分配的 opaque `SourceId` 表示。显示路径不是 identity；路径规范化与工作区相对显示遵守 RFC-0001，不进入语言语义。

### Byte ranges and human coordinates

- `TextSize` 是可容纳 M1 最大文件的 `u32` UTF-8 byte offset；`TextRange` 是同一 source 内的半开 `[start, end)`；`Span` 是 `SourceId + TextRange`；
- token 和语法节点边界必须位于 UTF-8 scalar boundary；EOF span 是 `[len, len)`；普通 range 不跨 source；
- byte range 是事实源。line/column 只由统一 line index 派生，均为 1-based Unicode scalar value 坐标，与 RFC-0001 一致；
- 文件总有第 1 行。LF 是一个换行；CRLF 是一个换行事件但保留两个原始 bytes。换行后 byte offset 映射到下一行 column 1；诊断不得把主要边界放在 CRLF 两字节之间；
- line index 保存每行起始 byte offset。任何 crate 不得自行扫描行列。

对 `// 😀\n中`：byte 0 是 1:1，byte 3 是 1:4，LF 起点 byte 7 是 1:5，byte 8 是 2:1，EOF byte 11 是 2:2。机器协议仍以这些 byte offset 为权威。

## Identifiers and Unicode

### Accepted rule

标识符遵守 Unicode Standard Annex #31 的 Unicode 17.0 XID 属性：

```text
IDENTIFIER := (XID_Start | "_") XID_Continue*
```

附加约束：

- 完整 spelling 必须已经是 NFC；编译器不得静默改写后再绑定；
- bare `_` 是独立 `UNDERSCORE` token，不是可声明名称；`_name` 是 identifier；
- 上述 source-level control/default-ignorable 禁令优先于 XID；
- 关键字只按下节列出的 exact lowercase ASCII spelling 识别；比较区分大小写，不做 locale case folding；
- identifier token 保留原始 UTF-8 bytes。NFC 检查成功不授权 formatter 改写名称；
- 中文 `计算`、`café`、`_结果2` 合法；以数字或 emoji 开头、`cafe` + U+0301 的非 NFC spelling、包含 U+200B 的名称非法。

Unicode 表由精确锁定的 `unicode-ident 1.0.24`（Unicode 17.0）和 `unicode-normalization 0.1.25` 实现。升级任一版本必须重跑全部 contract/corpus，并审查新增/删除 XID code point；若会改变既有源码的 tokenization，需新 RFC 或语言版本，而不是普通依赖更新。

### Security boundary

本 RFC 接受国际化标识符和 NFC 拒绝策略；它不声称消除 homoglyph。基于 UAX #39 skeleton 的跨 script/confusable warning、项目级 script policy 和名称安全 lint 保持 `proposed`，由有 symbol table 的 M2 步骤决定。全角冒号 `：` 等结构符号当前是非法字符，并应给出指向 ASCII `:` 的短提示；全角字母若满足 XID/NFC，只是 identifier，不会被当作关键字。

## Newlines, trivia, and comments

- `NEWLINE` trivia 的原始 spelling 只能是 LF (`0A`) 或 CRLF (`0D 0A`)；bare CR 非法；
- `WHITESPACE` trivia 只包含一个或多个 ASCII space U+0020 或 TAB U+0009；NBSP、全角空格和其他 Unicode whitespace 非法；
- `LINE_COMMENT` 从 `//` 开始，到 LF/CRLF 前结束；newline 不属于 comment token。comment 内容可含任何未被 source-level 禁止的 Unicode scalar；
- trivia 必须与普通 token 一样保留原始 bytes 和 range。对有效且未超限的 source，按 range 顺序拼接全部非 EOF token 必须逐字节恢复输入；
- 缩进在 M1 grammar 中没有语义。formatter 最终输出两个 ASCII spaces 和 LF，但 lexer 不把 TAB/CRLF 改写后交给 lossless tree；
- block comment、嵌套 comment、doc comment 的独立 token 身份和 shebang 均为 `proposed/not accepted`。`/*` 当前由非法 `/` 开始；`///` 当前只是普通行注释。

## Token contract

lexer 使用 maximal munch，并产生以下 accepted token family：

| Family | Accepted spelling |
|---|---|
| trivia | `WHITESPACE`、`NEWLINE`、`LINE_COMMENT` |
| name | `IDENTIFIER`、`UNDERSCORE`、下列 hard keyword |
| literal | `INTEGER`、`STRING`、`TRUE`、`FALSE` |
| delimiter | `(` `)` `[` `]` |
| punctuation | `:` `,` `.` `@` |
| operator | `+` `-` `=` `==` `<=` |
| recovery | `ERROR`（有效 UTF-8 中不能组成 accepted token 的最小 scalar/run） |
| synthetic | `EOF`，长度为零且不参与 lossless byte 拼接 |

hard keyword exact set：

```text
async await borrow call capabilities capability case effects else end enum
error export from function group if ignore interface invariant let match move
newtype none ok record resource return returns self spawn task try using
```

`true`/`false` 是 literal token；bare `_` 是 `UNDERSCORE`。`Int`、`Result`、`Component`、用户类型和普通小写名称均是 identifier，不由 lexer赋予语义。

`INTEGER` 是一个或多个 ASCII digit `[0-9]+`；符号始终是独立 `+`/`-` token。前导零的含义、范围与类型由后续 RFC/语义阶段决定，lexer 不拒绝。decimal、exponent、digit separator、base prefix 和浮点特殊值均为 `proposed/not accepted`，以免在 RFC-0003 仍 proposed 时抢先决定数值表层。

`STRING` 由 ASCII `"` 包围，不跨 newline。普通内容可以是除 `"`、`\` 和 source-level forbidden scalar 外的 Unicode scalar；只接受 `\"`、`\\`、`\n`、`\r`、`\t`、`\0` 六种 escape。token 保留原始 spelling；escape 的值解释不属于 lexer。raw/multiline/byte string、Unicode escape 和 interpolation 均为 `proposed/not accepted`。

双字符 token `//`、`==`、`<=` 优先于其前缀。独立 `/`、`<`、`>`、`;`、`{`、`}`、反引号、全角标点和任何未列字符产生 lexical error；parser 不得把它们猜作相近 ASCII token。

## Input limits

下列为 accepted M1 compiler hard limit，不是应用 Runtime 配额：

| Limit | Inclusive maximum | Boundary behavior |
|---|---:|---|
| source bytes | 16,777,216 (16 MiB) | 解码/tokenization 前拒绝 |
| bytes per physical line, including newline | 1,048,576 (1 MiB) | source diagnostic 后停止 lexing |
| bytes per identifier | 1,024 | 单个 lexical diagnostic，保留 token bytes |
| bytes per any other token/trivia | 1,048,576 (1 MiB) | 单个 lexical diagnostic，禁止无界缓冲 |
| emitted tokens including trivia, excluding EOF | 1,000,000 | 达到上限后的下一 token 处停止 |
| emitted lexical diagnostics | 100 | 达到上限后停止并标记 truncated |

恰好等于上限允许，超过才拒绝。实现必须在分配与递归前检查，使用 checked arithmetic，不因诊断而复制整文件或超长 token。CLI 可以设置更低的 `max-errors`，不能把上述硬上限调高。STEP-0021 用 fuzz/performance 数据复审；提高限额是兼容变更，降低会拒绝既有输入，必须新 ADR/RFC 和迁移说明。

## Positive and negative cases

机器可读 case 位于 [`tests/lexical/contract-v0.json`](../../tests/lexical/contract-v0.json)。STEP-0015 校验它的 ID、分类、Unicode/version、span 点与 limit 值；STEP-0016 必须把同一文件接入真实 source/lexer 测试。

代表性 positive：无 BOM ASCII、中文/NFC identifier、`_结果2`、LF、CRLF、TAB/中文行注释、整数、字符串 escape、全 token family、emoji line/column 映射、恰好等于各限额。

代表性 negative：非法 UTF-8、BOM、非 NFC identifier、default-ignorable、emoji 开头、bare CR、NBSP、block comment、unterminated/bad-escape string、全角冒号、NUL、每个限额 +1。

在 STEP-0016 分配稳定 code 前，case 中的短消息是 provisional oracle，例如 `source is not valid UTF-8`、`identifier must be NFC`、`use ASCII ':'`、`source exceeds 16 MiB`；不得现在占用 E1xxx 并宣称已有 span/recovery 证据。

## AST and IR

lossless syntax tree 必须包含每个 token/trivia 的 kind、原始 text 和 byte range；error token 也保留。semantic AST 不保留纯 trivia，但每个可归因节点引用统一 `Span`。NFC 失败、source/limit 失败或任何 lexical `ERROR` 都禁止形成可进入 M2/IR 的成功 AST。

Sico IR 不记录 UTF-8 spelling、换行或 comment；identifier 的符号身份由 M2 定义。本 RFC 不允许 lexer 通过 normalization 合并两个名称。

## Component/WIT mapping

source/token/span 是 compiler 输入层，不改变 Component/WIT ABI。未来导出的 Sico 名称如何映射到 WIT identifier、非 ASCII 名称是否需要转义以及 debug/source map 如何携带 span 均保持未决定；lexer 实现不得建立隐式 ABI 映射。

## Workspace engineering decision

STEP-0015 建立七 crate 前端边界，要求 stable Rust（manifest 的最低版本为 1.97）、edition 2024，并锁定全部 registry 依赖。本步骤验证环境是 Rust 1.97.0；`rust-toolchain.toml` 跟随 stable channel，避免把语言版本兼容误当成单一 patch 工具链安装：

- `sico-source` 使用 `text-size 1.1.1`；统一 range API 在 STEP-0016 实现；
- `sico-syntax` 选择 `rowan 0.16.1` 的 immutable green tree。相比自建 tree，它已有成熟 lossless/incremental 基础且不规定 Sico grammar；自建 tree 维护与 fuzz 面更大，拒绝；
- `sico-parser` 选择手写 recursive-descent + event stream（STEP-0017）。相比 parser generator，它更直接控制 B 的具名关闭与 recovery anchor；若 54/12 gate 无法满足则重开选择，不用 grammar hack 放宽；
- `sico-cli` 选择 `clap 4.6.1`、关闭不需要的默认 feature 并精确启用 help/usage/error context/suggestions。相比手写解析，它减少边界错误；相比更轻 parser 增加少量依赖，但 CLI 不是热路径。STEP-0020 前不创建 binary，不提前固定命令/退出码；
- `sico-lexer` 锁定上述 Unicode 两个 crate；所有 registry dependency 使用 exact requirement，`Cargo.lock` 入库，更新必须显式审查。

workspace crate 为空实现边界不是 compiler 功能。STEP-0015 只要求 build/Clippy/test/metadata 通过。

## AI evaluation

该 contract 预期减少模型生成的隐藏字符、换行漂移和错误 escape，但没有真实模型数据。本步骤只验证 deterministic cases。未来 AI corpus 应加入：ASCII 与中文命名、NFC 修复、全角标点修复、CRLF/LF、invalid escape；没有授权运行前不产生成功率或 token 结论。

## Compatibility

M1 尚未发布稳定语言。RFC accepted 后，改变编码、NFC/XID、hard keyword、accepted punctuation、comment/string/number spelling、span 坐标或降低限额都需要新 RFC 与 contract case 迁移；增加 Unicode 版本也可能改变 tokenization，按 identifier 章节审查。

LF/CRLF 都能输入，formatter 未来只生成 LF。该 canonicalization 不改变 lossless parse 的原始输入，也不授权无请求改写文件。

## Security and privacy

严格 UTF-8、禁止 default-ignorable/control、NFC 拒绝、精确 byte range 和 hard limit 降低隐藏 token、跨度欺骗与内存拒绝服务风险，但不是完整 Unicode 安全证明。confusable/script lint 仍 proposed；路径与源码片段遵守 RFC-0001 去敏；诊断不得把非法 bytes 当终端控制序列原样输出。

## Alternatives

### ASCII-only identifiers

实现最小且避免一部分 homoglyph，但排除中文等本地领域名称，迫使 AI/用户转写；拒绝。Unicode XID + 明确 normalization/安全边界更符合国际化目标。

### Unicode XID with silent NFC normalization

输入宽容，但源码 byte spelling 与绑定 identity 分离，两个视觉近似输入会被静默合并；拒绝。要求 NFC 并给出局部修复。

### NFKC identifiers

能折叠更多兼容字符，但会改写语义可见 spelling，并不能独自解决跨 script confusable；本轮拒绝，保留 UAX #39 lint 复审。

### Accept BOM, all Unicode whitespace, or bare CR

提高历史编辑器兼容性，却扩大隐藏字符和跨平台坐标形态；当前 corpus 无需求，拒绝。CRLF 已覆盖 Windows 正常文件。

### Block comments and broad literal grammar now

没有 B positive/negative corpus，也涉及 nesting、文档归属、decimal/float 语义；保持 proposed，禁止 lexer 猜测。

### Custom tree, generated parser, or hand-written CLI

自建 tree 与手写 CLI 扩大通用工程面；生成 parser 限制局部 recovery 控制。本轮分别选择 rowan、手写 event parser、clap；以 M1 gate 和依赖审计为撤销条件。

## Validation and acceptance criteria

RFC 在 M1 v0 范围 accepted，当且仅当：

- contract JSON 同时覆盖 UTF-8、identifier/Unicode、newline、trivia/comment、token/literal、span 和 limit 的 positive/negative；
- 208 个现有 `.sico` 仍是无 BOM 严格 UTF-8 且不含 bare CR；54 个 B case 不使用未接受 lexical form；
- workspace 七 crate、精确依赖和锁文件可在 Rust 1.97.0 build、format、Clippy、test；
- validator 检查 RFC/contract/workspace/version/limit 一致性；M0 validators 不回归；
- 文档明确列出 proposed 项，不把空 crate、case validator 或静态 corpus scan 冒充 lexer/parser 行为。

STEP-0016 的真实实现必须让全部 contract case 成为 executable source/lexer tests；若实现无法保持 byte coverage、span 或有界资源，不能通过删除 case 来结束步骤，必须重开本 RFC。

## Links

- [`STEP-0015`](../steps/STEP-0015-compiler-workspace-lexical-source.md)
- [`M1 compiler frontend plan`](../plans/M1-compiler-frontend.md)
- [`RFC-0001 diagnostics`](./RFC-0001-diagnostics-protocol-v0.md)
- [`RFC-0005 B syntax`](./RFC-0005-labeled-block-syntax-baseline.md)
- [`contract-v0.json`](../../tests/lexical/contract-v0.json)
- [Unicode Standard Annex #31: Unicode Identifiers and Syntax](https://www.unicode.org/reports/tr31/)
- [Unicode Standard Annex #15: Unicode Normalization Forms](https://www.unicode.org/reports/tr15/)
- [Unicode Technical Standard #39: Unicode Security Mechanisms](https://www.unicode.org/reports/tr39/)
- [Unicode 17.0.0](https://www.unicode.org/versions/Unicode17.0.0/)
- [`unicode-ident 1.0.24`](https://crates.io/crates/unicode-ident/1.0.24)
- [`unicode-normalization 0.1.25`](https://crates.io/crates/unicode-normalization/0.1.25)
- [`rowan 0.16.1`](https://crates.io/crates/rowan/0.16.1)
- [`clap 4.6.1`](https://crates.io/crates/clap/4.6.1)
