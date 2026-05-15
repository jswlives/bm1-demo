# Claude Code 上下文优化指南

## 问题

每次新对话启动时，Claude 需要重新扫描代码结构，消耗大量上下文窗口和 Token。本文档说明如何通过分层文档策略减少这种浪费。

## 核心原则

**CLAUDE.md 保持精简，只放高频使用的索引信息；详细信息放在 docs 下按需读取。**

CLAUDE.md 每次对话自动加载，内容越多消耗 Token 越多。只放 Claude 每次都需要的核心信息（构建命令、架构概要、命令注册表），详细内容通过 `docs/code-structure.md` 按需加载。

## 文档分层策略

```
CLAUDE.md                     ← 每次对话自动加载，保持精简
  ├── 构建命令
  ├── 架构概要 & 数据流
  ├── 命令注册表（小型表格）
  ├── 新增命令步骤
  └── Reference Docs → 指向 docs/ 下的详细文档

docs/code-structure.md        ← 按需读取，详细内容
  ├── 完整文件树（含每个文件的说明）
  ├── 核心 API 签名
  └── Proto 定义

docs/context-optimization-guide.md  ← 本文档，策略说明
```

## 维护指南

### CLAUDE.md — 只更新以下内容

- 新增 proto 命令 → 更新命令注册表和 next oneof number
- 架构变更 → 更新数据流或 Key Design Decisions
- 新增构建命令 → 更新 Build & Test Commands

### docs/code-structure.md — 以下变更时更新

- 新增/删除源文件 → 更新文件树
- 修改公开 API 签名 → 更新 API 签名部分
- 新增 proto message → 更新 Proto Definitions 部分

### 不需要更新的情况

- 修改函数内部实现（签名未变）
- 新增测试用例
- 修改注释
- 修改依赖版本

## 提示词技巧

| 写法 | 效果 |
|------|------|
| "修改 `bm1-server/src/handler/add_money.rs`" | Claude 直接定位文件，零扫描 |
| "看看 handler 那边的代码" | Claude 需要先扫描 handler 目录 |
| "在 LoginHandler 中添加字段检查" | Claude 从 CLAUDE.md 已知位置，直接读取 |
| "帮我加个新命令" | Claude 参考 CLAUDE.md 中的步骤执行 |
| "先读一下 docs/code-structure.md" | 手动指示 Claude 加载详细结构 |

## Memory 系统

Claude 的持久化记忆（`~/.claude/projects/` 下的 memory 文件）在跨对话时自动加载，作为 CLAUDE.md 的补充。适合存储：
- 项目决策背景（为什么选择某种设计）
- 用户的协作偏好
- 容易过期的项目状态

## 检查清单

如果一次新对话中 Claude 需要执行超过 3 次文件探索操作（Glob/Grep/Read 用于了解结构而非修改），说明文档可能需要补充：

- [ ] docs/code-structure.md 文件树中缺少新增的文件
- [ ] docs/code-structure.md API 签名与实际代码不一致
- [ ] CLAUDE.md 命令注册表缺少新命令
- [ ] CLAUDE.md next oneof field number 未更新
