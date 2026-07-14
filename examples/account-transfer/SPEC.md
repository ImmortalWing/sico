# Account transfer specification

> 状态：代表性程序草案
> 目的：验证领域类型、记录更新、契约、事务和存储能力

## 功能

在两个不同、有效且已开启的账户之间转移正数金额。转账必须满足：

- 转出与转入账户不同；
- 金额大于零；
- 两个账户都存在且处于 `Open`；
- 转出账户余额足够；
- 两个账户的写入必须原子提交；
- 成功前后总金额不变。

## 领域模型

```text
AccountId = opaque Text
Money = decimal monetary value

Account {
  id: AccountId
  balance: Money
  status: Open | Closed
}

TransferChange {
  from: Account
  to: Account
}
```

## 正式错误

```text
SameAccount
InvalidAmount(amount)
AccountNotFound(id)
AccountClosed(id)
InsufficientBalance(available, requested)
StorageFailure
```

## 逻辑分层

- `apply_transfer` 是纯函数，验证业务规则并返回两个新账户；
- `load_account` 将存储错误和不存在映射为领域错误；
- `transfer` 在一个事务中读取、计算并写回；
- 事务失败时不得只保存一个账户。

## 核心契约

```text
when apply_transfer returns ok(change):
  change.from.id == from.id
  change.to.id == to.id
  change.from.balance == from.balance - amount
  change.to.balance == to.balance + amount
  change.from.balance + change.to.balance
    == from.balance + to.balance
```

## 测试

| 场景 | 结果 |
|---|---|
| 余额 100 转 30 | 余额变为 70 和 30 |
| 相同账户 | `SameAccount` |
| 金额为 0 或负数 | `InvalidAmount` |
| 转出账户关闭 | `AccountClosed` |
| 转入账户关闭 | `AccountClosed` |
| 余额不足 | `InsufficientBalance` |
| 任一账户不存在 | `AccountNotFound` |
| 第二次写入失败 | 整个事务回滚 |

## AI 语义概要目标

```text
module: account-transfer
purpose: atomically move money between two accounts
invariants: positive amount, distinct accounts, balance conserved
errors: 6
effects: storage.read, storage.write, storage.transaction
state writes: Account.balance for two accounts
```

## 暴露的开放问题

- `Money`、小数精度、货币单位、舍入和溢出语义；
- opaque/newtype 的语法与运行时表示；
- 记录构造和不可变更新语法；
- 前置条件、后置条件和 `old` 值如何表达；
- 契约由编译器证明、测试生成还是运行时检查；
- 存储接口属于 WASI、Sico WIT 还是标准库；
- 通用事务如何跨 Component 边界表达；
- 事务中的异步、取消和重试语义；
- 存储错误如何转换为领域错误；
- 权限声明应在 import、效果还是 `.sapp` 清单中出现；
- 两账户并发转账的隔离级别和死锁处理；
- AI 影响分析如何识别“总余额守恒”不变量。
