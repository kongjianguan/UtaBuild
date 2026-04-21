use clap::{Parser, Subcommand};
use std::path::PathBuf;
use utabuild_cli::commands::{handle_history, handle_search, HistoryAction};

#[derive(Parser)]
#[command(name = "utabuild-cli")]
#[command(about = "UtaBuild CLI - 歌词搜索与管理工具", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "搜索歌词")]
    Search {
        #[arg(short, long, help = "歌曲标题")]
        title: Option<String>,

        #[arg(short, long, help = "歌手名")]
        artist: Option<String>,

        #[arg(short, long, default_value = "1", help = "页码，默认为 1")]
        page: u32,

        #[arg(short, long, help = "选择缓存中的结果索引")]
        select: Option<u32>,

        #[arg(short, long, value_name = "PATH", help = "输出到指定路径文件")]
        output: Option<String>,

        #[arg(short = 'd', long, help = "按默认格式输出 (${artist} - ${title}.json)")]
        output_default: bool,

        #[arg(long, help = "启用日志")]
        log: bool,

        #[arg(long, value_name = "PATH", help = "指定日志文件路径")]
        log_path: Option<PathBuf>,

        #[arg(long, value_name = "PATH", help = "指定缓存目录")]
        cache_dir: Option<PathBuf>,
    },
    #[command(about = "管理搜索历史")]
    History {
        #[command(subcommand)]
        action: HistoryAction,

        #[arg(long, value_name = "PATH", help = "指定缓存目录")]
        cache_dir: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search {
            title,
            artist,
            page,
            select,
            output,
            output_default,
            log,
            log_path,
            cache_dir,
        } => {
            if output.is_some() && output_default {
                eprintln!("错误: --output 和 --output-default 不能同时使用");
                std::process::exit(1);
            }
            let effective_log_path = if log {
                log_path.or_else(|| Some(std::path::PathBuf::from("utabuild-cli.log")))
            } else {
                log_path
            };
            handle_search(
                title,
                artist,
                page,
                select,
                effective_log_path,
                cache_dir,
                output,
                output_default,
            )
            .await?;
        }
        Commands::History { action, cache_dir } => {
            handle_history(action, cache_dir).await?;
        }
    }

    Ok(())
}
