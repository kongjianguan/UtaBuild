//! 命令模块
//!
//! 本模块定义了 CLI 的子命令枚举和对应的处理函数，包括搜索歌词、通过 URL 获取歌词以及管理历史记录等功能。

pub mod history;
pub mod search;

use clap::Subcommand;
use std::path::PathBuf;

/// 历史操作子命令枚举
#[derive(Subcommand)]
pub enum HistoryAction {
    /// 列出搜索历史
    #[command(about = "列出搜索历史")]
    List,
    /// 使用历史记录中的条目重新搜索
    #[command(about = "使用历史记录搜索")]
    Use {
        /// 历史记录索引（从 0 开始）
        #[arg(help = "历史记录索引")]
        index: u32,
    },
    /// 清除所有搜索历史
    #[command(about = "清除搜索历史")]
    Clear,
}

#[allow(clippy::too_many_arguments)]
/// 处理搜索命令
///
/// 根据提供的标题、艺术家等参数执行歌词搜索。
///
/// - `title`: 可选的歌曲标题
/// - `artist`: 可选的艺术家名称
/// - `page`: 分页页码
/// - `select`: 可选的选择索引，用于直接选取某个搜索结果
/// - `log_path`: 可选的日志文件路径
/// - `cache_dir`: 可选的缓存目录路径
/// - `output`: 可选的输出文件路径
/// - `format`: 输出格式（"json" 或 "html"）
pub async fn handle_search(
    title: Option<String>,
    artist: Option<String>,
    page: u32,
    select: Option<u32>,
    log_path: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
    output: Option<String>,
    format: String,
) -> anyhow::Result<()> {
    if log_path.is_some() {
        crate::logger::init_logger_with_pathbuf(log_path);
    }
    search::execute(title, artist, page, select, cache_dir, output, format).await
}

/// 处理通过 URL 获取歌词的命令
///
/// 跳过搜索步骤，直接通过歌词 URL 获取歌词内容。
///
/// - `url`: 可选的歌词 URL
/// - `output`: 可选的输出文件路径
/// - `format`: 输出格式（"json" 或 "html"）
/// - `log_path`: 可选的日志文件路径
/// - `cache_dir`: 可选的缓存目录路径
pub async fn handle_url_lyrics(
    url: Option<String>,
    output: Option<String>,
    format: String,
    log_path: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    if log_path.is_some() {
        crate::logger::init_logger_with_pathbuf(log_path);
    }
    search::execute_from_url(url, output, format, cache_dir).await
}

/// 处理历史记录命令
///
/// 根据指定的操作类型执行对应的历史记录处理逻辑。
///
/// - `action`: 历史操作类型（List / Use / Clear）
/// - `cache_dir`: 可选的缓存目录路径
pub async fn handle_history(
    action: HistoryAction,
    cache_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    match action {
        HistoryAction::List => history::list(cache_dir.as_ref()),
        HistoryAction::Use { index } => history::use_record(index, cache_dir.as_ref()).await,
        HistoryAction::Clear => history::clear(cache_dir.as_ref()),
    }
}
