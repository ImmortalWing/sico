# HTTP greeting service specification

> 状态：代表性程序草案
> 目的：验证异步、路由、请求验证、序列化和并发请求

## 功能

服务提供两个端点：

```text
GET  /health  → 200 { "status": "ok" }
POST /greet   → 200 { "message": "Hello, <name>!" }
```

`POST /greet` 接受：

```json
{ "name": "Ada" }
```

## 错误映射

| 场景 | 状态码 | 响应错误码 |
|---|---:|---|
| 路径不存在 | 404 | `not_found` |
| 方法不允许 | 405 | `method_not_allowed` |
| JSON 非法 | 400 | `invalid_json` |
| name 缺失或为空 | 422 | `invalid_name` |
| body 超限 | 413 | `body_too_large` |
| 内部编码失败 | 500 | `internal_error` |

错误响应保持统一结构：

```json
{ "error": "invalid_name" }
```

## 约束

- handler 必须可被并发调用；
- handler 不使用全局可变状态；
- body 在解析前限制大小；
- JSON 输入不允许未知字段还是忽略未知字段，尚未决定；
- 客户端取消后停止继续读取 body；
- 内部错误不泄漏宿主路径、堆栈或敏感数据；
- 每个请求的资源在结束、失败和取消时释放。

## 测试重点

- health 成功；
- greet 成功；
- 空 name；
- 错误 JSON；
- 超大 body；
- GET /greet；
- 未知路径；
- 多个并发请求互不污染；
- 请求取消；
- 响应 JSON 编码失败不泄漏内部错误。

## AI 语义概要目标

```text
module: http-service
routes: GET /health, POST /greet
async entry: handle(Request) -> Response
effects: http.read_body, http.respond
limits: body <= 16 KiB
shared mutable state: none
```

## 暴露的开放问题

- `async`/`await` 是否显式出现在表层语法；
- HTTP 类型来自 WASI HTTP、Sico WIT 还是标准库包装；
- 路由用普通匹配、属性、表还是专用 DSL；
- 路径参数和查询参数的类型化方式；
- JSON derive、反射和 schema 生成；
- 未知 JSON 字段策略；
- body stream、大小限制和取消传播；
- handler 的 `Response` 与 `Result<Response, E>` 边界；
- 错误到状态码的集中映射；
- 日志、trace 和敏感数据标记；
- Runtime 如何限制并发、CPU 和响应时间；
- async Component/WASI 版本兼容。
