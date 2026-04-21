//! 跨平台路径抽象
//!
//! 提供统一的路径获取函数，兼容Windows/macOS/Linux/Android/iOS

use std::path::PathBuf;

/// 获取应用缓存目录
///
/// - Windows: `%LOCALAPPDATA%\utabuild`
/// - macOS: `~/Library/Caches/utabuild`
/// - Linux: `~/.cache/utabuild`
/// - Android: `/data/data/com.utabuild.app/cache`
pub fn get_cache_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        PathBuf::from("/data/data/com.utabuild.app/cache")
    }

    #[cfg(not(target_os = "android"))]
    {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("utabuild")
    }
}

/// 获取应用配置/数据目录
///
/// - Windows: `%APPDATA%\utabuild`
/// - macOS: `~/Library/Application Support/utabuild`
/// - Linux: `~/.local/share/utabuild`
/// - Android: `/data/data/com.utabuild.app/files`
pub fn get_data_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        PathBuf::from("/data/data/com.utabuild.app/files")
    }

    #[cfg(not(target_os = "android"))]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("utabuild")
    }
}

/// 获取日志文件路径
pub fn get_log_path() -> PathBuf {
    get_cache_dir().join("utabuild.log")
}

/// 确保目录存在
pub fn ensure_dir_exists(path: &PathBuf) -> anyhow::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}
