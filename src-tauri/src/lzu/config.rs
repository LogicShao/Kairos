//! LZU 本地配置读取。

use std::path::PathBuf;

/// 读取后端专用配置，优先使用进程环境变量，缺失时回退到 `src-tauri/.env.local`。
pub fn read_secret(env_name: &str) -> Result<String, crate::lzu::error::LzuError> {
    match std::env::var(env_name) {
        Ok(value) => Ok(value),
        Err(_) => {
            load_local_env_file();
            std::env::var(env_name).map_err(|_| {
                crate::lzu::error::LzuError::Config(format!("缺少 {env_name} 环境变量"))
            })
        }
    }
}

fn load_local_env_file() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env.local");
    if path.exists() {
        if let Err(err) = dotenvy::from_path(&path) {
            log::warn!("读取 src-tauri/.env.local 失败: {err}");
        }
    }
}
