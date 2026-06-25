# Type Safety

> Kairos 前端类型契约规范。重点是让多个 AI session 对 Tauri IPC 数据结构保持同一理解。

---

## Overview

Kairos 前端使用 TypeScript，运行时数据来自 Rust Tauri commands。`src/types/*.ts` 是前端对后端 IPC 入参和返回值的契约镜像，不是独立领域模型。

核心原则：

- **契约唯一来源**：Rust command / model 是运行时事实，TypeScript 类型必须对齐返回结构。
- **最小注释**：只给跨层字段、日期格式、同步字段、空值语义和枚举取值写注释。
- **不本地猜测**：组件不重新定义 IPC payload，不用局部 `as { ... }` 解析后端字段。

---

## Type Organization

- `src/types/task.ts`、`course.ts`、`exam.ts`：镜像 `src-tauri/src/db/models.rs` 中对应实体，以及 Tauri command 的 create/update/filter payload。
- `src/types/sync.ts`：镜像 `src-tauri/src/db/models.rs::SyncConfig` 和 `src-tauri/src/sync/exporter.rs::{SyncStats, SyncResult}`。
- `src/types/schedule.ts`：镜像 `src-tauri/src/schedule.rs` 的 week/calendar response。
- `src/types/pomodoro.ts`：镜像 `src-tauri/src/timer.rs::PomodoroState` 和 `commands/pomodoro.rs::PomodoroConfigData`。
- `src/types/course-import.ts`：镜像 `src-tauri/src/importers.rs::ImportTextResult`。

组件和 hooks 应 import 这些共享类型，不在本地复制同名接口。

---

## Tauri IPC Contracts

调用 `invoke<T>()` 时必须传入返回类型：

```ts
const tasks = await invoke<Task[]>("get_all_tasks", { filters })
```

类型定义规则：

- 后端返回实体字段应出现在前端返回类型里，即使当前 UI 不渲染该字段。
- 命令入参可以保留 optional 字段，因为 Rust command 通过 `#[serde(default)]` 接收缺省值。
- `null` 表示后端明确返回空值，例如 SQLite nullable 字段。
- `undefined` 只用于前端可省略的命令入参，不用于后端返回字段。
- 日期字符串必须在注释中说明格式或来源，尤其 RFC3339、`YYYY-MM-DD`、`HH:mm`。

---

## Comment Requirements

导出的 IPC 接口满足以下任一条件时，需要类型级注释：

- 对应 Rust model / response / command payload。
- 被多个组件复用。
- 字段跨越数据库、后端 service、Tauri IPC、前端组件边界。

字段满足以下任一条件时，需要字段级注释：

- `sync_id`、`deleted_at`、`remote_etag`、`device_id`、`dataset_id` 等同步协议字段。
- 日期、时间、周次、星期、颜色、tags 等格式容易被误解的字段。
- `null` 和空字符串有不同业务含义。
- 字段名不足以说明取值范围，例如 `kind`、`source_link`、`conflicts`。

不要给 `id`、`title`、`name` 这类自解释字段写重复注释。

---

## Validation Boundary

当前项目没有引入 Zod/Yup 等运行时校验库。校验边界如下：

- 后端负责数据库约束、命令默认值、同步合并和时间计算。
- 前端负责表单输入约束和显示前的简单格式化。
- 前端不得把未知外部 JSON 直接 cast 成业务类型；如果将来引入外部导入或配置解析，必须先定义类型守卫或后端解析入口。

---

## Common Patterns

使用 literal union 表达后端固定字符串：

```ts
export type TaskStatus = "todo" | "in_progress" | "done"
```

用 `string | null` 表示后端 nullable 字段：

```ts
due_date: string | null
```

用 optional 表示命令入参可省略：

```ts
status?: TaskStatus
```

---

## Forbidden Patterns

- 禁止 `any`、`as any`、`@ts-expect-error` 掩盖契约不一致。
- 禁止在组件内复制 `Task`、`Course`、`Exam`、`SyncConfig` 等共享契约。
- 禁止用局部 cast 读取 IPC payload 字段，例如 `(payload as { id: number }).id`。
- 禁止把后端返回字段从前端类型中省略，只因为当前 UI 暂时不用。
- 禁止把 nullable 返回字段写成 optional 字段。

---

## Checklist

修改 IPC 类型前确认：

- [ ] 已查看对应 Rust struct / command payload。
- [ ] 返回类型包含后端实际返回字段。
- [ ] optional 和 nullable 使用一致。
- [ ] 高风险字段有注释，普通字段没有噪音注释。
- [ ] 组件通过共享类型 import，不重新定义 payload。
