# Version Bump & Release Tag Guide

> **Purpose**: 只在准备产出一个可发布快照时，统一多端版本号，并让 git tag 指向已经提交的 release commit。

---

## When to Use

- [ ] 一批用户可感知改动已经完成，准备作为一个版本收口
- [ ] 当前工作区不再混有未纳入本次 release 的临时改动
- [ ] 本轮最低发布校验已通过（至少 `npm run lint` 与 `npm run build`，如涉及 Tauri/Rust 再补对应检查）
- [ ] 现在要做的是“发布快照”，不是普通开发提交、文档修订或仓库清理

---

## Release Boundary

以下情况通常**不**单独 bump 版本或打 release tag：

- 纯文档改动
- Trellis / Git 边界整理
- 仅内部重构，且没有计划作为一个独立版本发布
- 仍处于多任务并行开发中，尚未整理出稳定发布批次

以下情况通常**应该**考虑 bump 版本并准备 tag：

- bug 修复、UI 改进、体验优化已经组成一个准备发布的补丁批次
- 新功能、新页面、显著用户可见能力已经完成并准备对外
- 兼容性或数据格式发生变化，需要明确版本边界

---

## Version Bump Decision Matrix

| 级别 | 示例 | 适用场景 | 备注 |
|------|------|---------|------|
| **Patch** | `0.2.1 -> 0.2.2` | bug 修复、UI 打磨、小范围体验优化、可发布的小批次收尾 | 默认选择 |
| **Minor** | `0.2.1 -> 0.3.0` | 新功能、新页面、显著用户可见变更 | 功能发布 |
| **Major** | `0.2.1 -> 1.0.0` | 破坏性 API / 数据格式变更、重大架构切换 | 必须先与用户确认 |

---

## Files to Update

版本号必须在以下 3 处保持一致：

| 文件 | 字段 |
|------|------|
| `package.json` | `"version"` |
| `src-tauri/Cargo.toml` | `[package]` 下 `version` |
| `src-tauri/tauri.conf.json` | 顶层 `"version"` |

---

## Tag Rules

- Tag 只用于标记**可发布快照**
- Tag 必须指向**已经提交**的 release commit，而不是未提交工作区
- Tag 名称统一为 `vX.Y.Z`
- Patch 可使用轻量 tag：`git tag v0.2.2`
- Minor / Major 建议使用附注 tag：`git tag -a v0.3.0 -m "v0.3.0: <summary>"`

---

## Required Order

1. 整理发布范围，只保留本次 release 相关改动
2. 通过发布前校验
3. 选择版本级别（Patch / Minor / Major）
4. 同步更新 3 个版本文件
5. 提交 release commit，使版本号修改进入 Git 历史
6. 给**该提交**打 `vX.Y.Z` tag

---

## Don't: Common Mistakes

### Don't: 在未提交状态下打 tag

```bash
# Wrong
# 先改版本号文件，但还没 commit
git tag v0.2.2

# 结果：tag 仍然指向旧提交，不包含 0.2.2 的版本号修改
```

```bash
# Correct
# 1. 修改 3 个版本文件
# 2. git commit -m "release: v0.2.2"
git tag v0.2.2
```

### Don't: 为非发布性改动单独打 release tag

- `chore:` 仓库清理
- spec / docs 小修
- Trellis 任务或工作流维护

这些改动可以提交，但默认不单独形成版本发布点。
