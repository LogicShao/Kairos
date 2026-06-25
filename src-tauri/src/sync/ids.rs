//! 同步标识符生成：所有 sync_id / device_id / dataset_id 通过此模块生成。
//! 当前使用 UUID v4，未来可替换为其他方案。

/// 生成新的 sync_id（UUID v4 字符串）。
/// 用于: 实体 sync_id 回填、device_id、dataset_id 初始化。
pub fn new_sync_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
