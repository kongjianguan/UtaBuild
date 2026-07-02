use tokio::sync::Mutex;
use utabuild_cli::{CacheManager, UtaTenSearcher};

/// 应用状态
pub struct AppState {
    pub searcher: Mutex<UtaTenSearcher>,
    pub lsp_logging_enabled: Mutex<bool>,
}

/// 初始化搜索器
pub fn create_searcher() -> UtaTenSearcher {
    let cache_manager = CacheManager::new();
    UtaTenSearcher::new(cache_manager)
}
