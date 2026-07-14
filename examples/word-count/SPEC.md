# Streaming word count specification

> 状态：代表性程序草案
> 目的：验证文件能力、流、资源生命周期、Unicode 和资源限额

## 功能

读取一个 UTF-8 文本文件并统计：

- Unicode 标量或用户可见字符数量；
- 单词数量；
- 行数；
- 字节数。

文件必须以流式方式读取，不能假设整个文件能放入内存。

## 正式错误

```text
PermissionDenied(path)
NotFound(path)
InvalidUtf8(path, offset)
FileTooLarge(limit)
ReadFailure(path)
```

## 资源规则

- 只申请指定文件的只读能力；
- reader 在成功、失败和取消时都必须关闭；
- 设置最大读取字节数；
- UTF-8 字符跨 chunk 时不得被拆坏；
- 单词跨 chunk 时仍计为一个单词；
- 不把路径错误信息泄漏到无权限应用。

## 测试重点

- 空文件；
- 单行 ASCII；
- 中文和 emoji；
- 最后一行没有换行符；
- 字符和单词跨 chunk；
- 非法 UTF-8；
- 无权限；
- 文件不存在；
- 超过大小限制；
- 中途读取错误仍关闭资源。

## AI 语义概要目标

```text
module: word-count
purpose: stream a UTF-8 file and count text units
effects: filesystem.read
capabilities: one read-only file
resource scope: reader
limits: max_bytes
```

## 暴露的开放问题

- Text 的 Unicode 单位：字节、标量、字素簇和单词边界；
- 文件、路径和 capability handle 的类型；
- 同步流还是异步流；
- `with`/RAII/析构如何保证关闭资源；
- 流式 UTF-8 decoder 属于标准库还是 Runtime；
- `for`、iterator、stream 的统一模型；
- 显式可变局部变量是否必要；
- 取消如何传播并执行清理；
- 大小和时间限额由程序还是 Runtime 强制；
- 文件错误如何避免泄漏宿主信息；
- 字节计数由 reader 还是文件元数据提供。
