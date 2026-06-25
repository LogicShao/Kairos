//! Sync protocol v2: 单文件 JSON 快照同步。
//!
//! 设计决策:
//! - 合并键: sync_id (跨设备 UUID)，v1 兼容回退到 SQLite id
//! - 胜负判定: LWW (Last-Writer-Wins)，比较 effective_timestamp
//! - 墓碑: deleted_at 非空即已删除，正常查询过滤 WHERE deleted_at IS NULL
//! - ETag: 条件上传 (If-Match) 防止覆盖更新，412 冲突时重试一次
//! - 重试: 仅重试一次，再次失败返回错误让用户稍后操作

pub mod exporter;
pub mod ids;
pub mod webdav;
