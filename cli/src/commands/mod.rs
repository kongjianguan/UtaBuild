pub mod search;
pub mod history;

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum HistoryAction {
    #[command(about = "列出搜索历史")]
    List,
    #[command(about = "使用历史记录搜索")]
    Use {
        #[arg(help = "历史记录索引")]
        index: u32,
    },
    #[command(about = "清除搜索历史")]
    Clear,
}

pub async fn handle_search(
    title: Option<String>,
    artist: Option<String>,
    page: u32,
    select: Option<u32>,
    log_path: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
    output: Option<String>,
    output_default: bool,
) -> anyhow::Result<()> {
    if log_path.is_some() {
        crate::logger::init_logger_with_pathbuf(log_path);
    }
    search::execute(title, artist, page, select, cache_dir, output, output_default).await
}

pub async fn handle_history(action: HistoryAction, cache_dir: Option<PathBuf>) -> anyhow::Result<()> {
    match action {
        HistoryAction::List => history::list(cache_dir.as_ref()),
        HistoryAction::Use { index } => history::use_record(index, cache_dir.as_ref()).await,
        HistoryAction::Clear => history::clear(cache_dir.as_ref()),
    }
}
