# 课表周视图与导入能力

## Goal

补齐课表周视图所需的后端数据契约，并支持从教务系统复制文本导入课程与考试数据，使前端可以按周切换显示课程，并在课表中叠加考试事件。

## What I already know

* 用户提供了课表复制样例：`_TEMP/_import_class_ref.txt`
* 复制内容是制表符分隔的文本，但一门课程会展开成多行，教师和上课时间会跨多行出现
* 当前课程数据模型缺少结构化周次字段，现状无法可靠支撑“按周切换”
* 当前考试数据模型缺少结束时间与学期字段，现状不适合叠加到周课表中
* 当前前端页面已支持课程/考试基本 CRUD，但这轮用户要求主要由后端完成，前端修改将交由 Claude Code 对接
* 用户提供了考试复制样例：`_TEMP/_import_final_ref.txt`

## Assumptions

* 本次由我主要完成后端逻辑，前端只输出对接说明，不继续提交前端代码
* 课程周次需要以结构化字段持久化，而不是只拼接在地点文本中
* 考试在周课表中以单次事件处理，不参与课程的周次循环逻辑
* 导入时需要尽量避免明显重复数据

## Requirements

* 课程数据模型新增结构化周次字段与学期起始日期字段
* 考试数据模型新增考试结束时间字段与学期字段
* 支持解析 `_TEMP/_import_class_ref.txt` 并生成带周次规则的课程记录
* 支持解析 `_TEMP/_import_final_ref.txt` 并生成考试记录
* 提供后端命令，供前端按学期与目标周查询“该周课程 + 该周考试”
* 导入命令对无法识别的文本返回明确错误，而不是静默失败
* 同步导出/导入链路包含新增字段，避免多端同步时丢失周次和考试范围信息

## Acceptance Criteria

* [x] 数据库迁移后，课程与考试表包含周视图所需新增字段
* [x] `_TEMP/_import_class_ref.txt` 能导入为带结构化周次信息的课程记录
* [x] `_TEMP/_import_final_ref.txt` 能导入为带开始/结束时间的考试记录
* [x] 后端可基于给定学期、学期开始日期和周次，返回该周应显示的课程与考试
* [x] 同步导入/导出不丢失新增字段

## Technical Approach

* 在 Rust 后端新增/扩展模型、迁移、CRUD 与同步字段
* 在后端新增课程导入解析与考试导入解析命令
* 在后端新增“周视图事件查询”命令，统一返回课程与考试事件
* 前端只消费新的结构化字段和查询命令，不再自行从地点文本猜周次

## Out of Scope

* 前端交互与视觉实现
* 自动覆盖或删除用户已有课程数据
* 复杂的学期配置 UI

## Technical Notes

* 参考文件：
  * `_TEMP/_import_class_ref.txt`
  * `_TEMP/_import_final_ref.txt`
* 主要影响文件：
  * `src-tauri/src/db/migrations.rs`
  * `src-tauri/src/db/models.rs`
  * `src-tauri/src/db/courses.rs`
  * `src-tauri/src/db/exams.rs`
  * `src-tauri/src/commands/courses.rs`
  * `src-tauri/src/commands/exams.rs`
  * `src-tauri/src/sync/exporter.rs`
