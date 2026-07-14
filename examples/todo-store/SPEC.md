# Todo store specification

> 状态：代表性程序草案
> 目的：验证集合、CRUD、可选值、持久化版本和冲突

## 功能

管理一个持久化 Todo 列表，支持：

- 新增；
- 重命名；
- 标记完成；
- 删除；
- 按 ID 查询；
- 保存与加载。

## 领域模型

```text
TodoId = opaque Text
Todo { id, title, completed, revision }
TodoList { schemaVersion, items }
```

每次成功修改使对应 Todo 的 `revision` 增加一。更新命令必须携带期望 revision，防止覆盖其他修改。

## 正式错误

```text
EmptyTitle
AlreadyExists(id)
NotFound(id)
AlreadyCompleted(id)
RevisionConflict(expected, actual)
UnsupportedSchema(version)
StorageFailure
```

## 不变量

- ID 唯一；
- title 去除两端空白后不能为空；
- completed 只能从 false 变为 true；
- 删除不存在项目失败；
- revision 冲突不得写入；
- 不认识的持久化 schema 不自动猜测。

## 测试重点

- 连续增加两个项目；
- 重复 ID；
- 空标题；
- 重命名并增加 revision；
- 完成项目；
- 重复完成；
- 旧 revision 更新冲突；
- 删除；
- 保存后加载保持相同语义；
- 未知 schema 版本失败。

## AI 语义概要目标

```text
module: todo-store
commands: Add, Rename, Complete, Delete
state: TodoList
invariants: unique id, non-empty title, monotonic revision
effects: storage.read, storage.write only in load/save
```

## 暴露的开放问题

- 集合更新、查找和唯一性检查的标准写法；
- `Option` 与 `Result` 的组合；
- `record with` 更新和嵌套集合更新；
- 命令枚举是否适合表达 CRUD；
- revision/乐观锁属于业务还是存储抽象；
- schema 版本和迁移如何声明；
- 序列化格式是否进入语言语义；
- ID 生成应由纯逻辑、Runtime 还是能力提供；
- completed 是否应是 Bool 还是状态枚举；
- AI 如何区分数据不变量与界面校验；
- 保存失败后的内存状态和磁盘状态如何协调。
