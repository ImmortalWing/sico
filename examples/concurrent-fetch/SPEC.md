# Concurrent fetch specification

> 状态：代表性程序草案
> 目的：验证结构化并发、顺序、取消、超时和错误聚合

## 功能

并发获取一组 URL，满足：

- 最大并发数由调用者指定；
- 每个请求有独立超时；
- 输出顺序与输入 URL 顺序一致；
- 单个请求失败不取消其他请求；
- 调用者取消时取消所有未完成请求；
- 每个结果记录成功、HTTP 状态、超时、网络错误或取消；
- 响应 body 有大小上限。

## 结果模型

```text
FetchResult =
  Success(url, status, bytes)
  HttpFailure(url, status)
  Timeout(url)
  NetworkFailure(url)
  Cancelled(url)
```

函数返回 `List<FetchResult>`，长度和顺序与输入相同。

## 测试重点

- 空输入；
- 一个成功请求；
- 并发上限为 2；
- 响应完成顺序与输入顺序不同；
- 单个超时；
- 单个网络失败；
- HTTP 500；
- body 超限；
- 整体取消；
- 并发上限为 0；
- 重复 URL 仍产生两个独立结果。

测试使用可控 `FakeHttpClient` 和虚拟时钟，不依赖真实网络与墙上时间。

## AI 语义概要目标

```text
module: concurrent-fetch
strategy: collect all results, preserve input order
concurrency: bounded
timeout: per request
cancellation: parent cancels children
effects: network.request, time.timeout
```

## 暴露的开放问题

- task、future、stream 和 async 函数的统一模型；
- 结构化并发的语法；
- 并发上限由库还是语言调度器实现；
- 任务取消是错误、独立状态还是效果；
- timeout 使用真实时钟还是注入时钟能力；
- collect-all 与 fail-fast 如何显式区分；
- 并发结果如何在保持输入顺序的同时流式输出；
- child task 是否允许脱离父作用域；
- body 大小限制和资源关闭；
- 测试替身如何实现同一 WIT 接口；
- WASI 0.3 future/stream 的映射；
- Runtime 如何防止任务泄漏。
