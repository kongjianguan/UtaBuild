//! utabuild-cli 库入口，公开所有子模块

/// 文件缓存模块（基于磁盘的 JSON 缓存）
pub mod cache;
/// 内存缓存模块（基于 moka 的高性能缓存）
pub mod cache_manager;
/// 命令行处理模块
pub mod commands;
/// 日志模块
pub mod logger;
/// LRC/YRC 歌词解析器
pub mod lrc_parser;
/// 数据模型定义
pub mod models;
/// NetEase EAPI 加密工具
pub mod ne_crypto;
/// NetEase 歌词源
pub mod ne_source;
/// 输出结构定义
pub mod output;
/// 跨平台路径抽象
pub mod platform;
/// QQMusic QMC 解密
pub mod qm_decrypt;
/// QRC 歌词解析器
pub mod qrc_parser;
/// 罗马音转平假名工具
pub mod romaji;
/// Ruby 注音对齐算法
pub mod ruby_align;
/// UtaTen 搜索器
pub mod searcher;

pub use cache_manager::{CacheManager, CacheStats, LyricsCache, SearchCache, SearchResultEntry};
pub use models::{
    LyricElement, LyricsSearchResponse, SearchPagination, SearchResponse, SearchResult,
};
pub use searcher::{
    parse_artist_info, ArtistInfo, ArtworkSourcePreference, LyricSourcePreference, UtaTenSearcher,
};
pub use ne_source::NeteaseSource;
