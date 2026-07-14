# Cross-platform notes UI specification

> 状态：代表性程序草案
> 目的：验证 UI 状态、事件、效果、生命周期、存储和跨平台宿主接口

## 功能

一个最小跨平台笔记应用：

- 显示笔记列表；
- 输入新笔记；
- 添加笔记；
- 切换完成状态；
- 删除笔记；
- 自动保存；
- 启动时加载；
- 显示加载、保存和错误状态。

同一个 `.sapp` 应在桌面和 Android Player 中运行。

## 架构

```text
init   → 初始 Model + Load effect
update → Model × Event → Model + Effects
view   → Model → View
handle → Effect → Event
```

`update` 和 `view` 是纯函数，存储只发生在 `handle`。

## 状态

```text
Model {
  notes: List<Note>
  draft: Text
  status: Loading | Ready | Saving | Failed(message)
}
```

## 事件

```text
DraftChanged(text)
AddPressed
TogglePressed(id)
DeletePressed(id)
Loaded(result)
Saved(result)
AppResumed
```

## 效果

```text
LoadNotes
SaveNotes(notes)
```

## 约束

- 空白标题不能添加；
- UI 事件不能直接访问存储；
- 保存失败保留内存状态并显示错误；
- 较旧保存完成事件不能覆盖较新状态；
- Android 暂停/恢复不得重复丢失数据；
- 应用只能访问自己的隔离存储；
- UI 树不能携带原生控件指针；
- UI 外观可平台适配，业务状态语义必须一致。

## 测试重点

- 初始加载成功与失败；
- 输入并添加；
- 空标题；
- toggle；
- delete；
- 保存成功与失败；
- 连续快速修改时旧保存结果返回；
- Android 暂停恢复；
- 桌面键盘与 Android 触摸产生相同领域事件；
- 无存储权限；
- 大列表渲染和事件限额。

## AI 语义概要目标

```text
app: notes-ui
state: Model
events: 7
effects: LoadNotes, SaveNotes
capabilities: ui, app-storage
pure: init/update/view
effect boundary: handle
platforms: desktop, android
```

## 暴露的开放问题

- UI 采用保留模式、即时模式还是声明式树；
- Model/Event/Effect 是否是标准库模式还是语言结构；
- UI 节点、属性、布局和样式类型；
- 事件 handler 如何避免闭包捕获隐藏状态；
- effect 执行顺序、取消、去重和版本；
- 异步保存竞态与 stale result；
- 生命周期如何映射 Android 与桌面；
- UI 与存储 WIT 接口；
- View diff 在 Runtime 还是应用侧；
- 列表 key、可访问性和输入法；
- 测试如何使用虚拟 UI 和虚拟存储；
- app 入口 exports 如何声明；
- UI 逻辑是否应有专门语义索引和事件流图。
