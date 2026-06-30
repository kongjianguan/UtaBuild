//! CLI 入口：定义命令行参数结构并分发子命令

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use utabuild_cli::commands::{handle_history, handle_search, handle_url_lyrics, HistoryAction};

/// UtaBuild CLI 命令行参数结构体
#[derive(Parser)]
#[command(name = "utabuild-cli")]
#[command(about = "UtaBuild CLI - 歌词搜索与管理工具", long_about = None)]
#[command(version)]
struct Cli {
    /// 子命令
    #[command(subcommand)]
    command: Commands,
}

/// 支持的命令枚举
#[derive(Subcommand)]
enum Commands {
    /// 搜索歌词
    #[command(about = "搜索歌词")]
    Search {
        /// 歌曲标题
        #[arg(short, long, help = "歌曲标题")]
        title: Option<String>,

        /// 歌手名
        #[arg(short, long, help = "歌手名")]
        artist: Option<String>,

        /// 直接从 UtaTen 歌词 URL 获取（跳过搜索）
        #[arg(long, help = "直接从 UtaTen 歌词 URL 获取（跳过搜索）")]
        url: Option<String>,

        /// 页码，默认为 1
        #[arg(short, long, default_value = "1", help = "页码，默认为 1")]
        page: u32,

        /// 选择缓存中的结果索引
        #[arg(short, long, help = "选择缓存中的结果索引")]
        select: Option<u32>,

        /// 输出到指定路径文件
        #[arg(short, long, value_name = "PATH", help = "输出到指定路径文件")]
        output: Option<String>,

        /// 按默认格式输出 (${artist} - ${title}.json)
        #[arg(short = 'd', long, help = "按默认格式输出 (${artist} - ${title}.json)")]
        output_default: bool,

        /// 启用日志
        #[arg(long, help = "启用日志")]
        log: bool,

        /// 指定日志文件路径
        #[arg(long, value_name = "PATH", help = "指定日志文件路径")]
        log_path: Option<PathBuf>,

        /// 指定缓存目录
        #[arg(long, value_name = "PATH", help = "指定缓存目录")]
        cache_dir: Option<PathBuf>,
    },
    /// 管理搜索历史
    #[command(about = "管理搜索历史")]
    History {
        /// 历史操作动作（列出/使用/清除）
        #[command(subcommand)]
        action: HistoryAction,

        /// 指定缓存目录
        #[arg(long, value_name = "PATH", help = "指定缓存目录")]
        cache_dir: Option<PathBuf>,
    },
}

/// 程序入口：解析命令行参数并分发给对应的处理函数
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search {
            title,
            artist,
            url,
            page,
            select,
            output,
            output_default,
            log,
            log_path,
            cache_dir,
        } => {
            if url.is_some() {
                // URL 模式：跳过搜索直接获取歌词
                return handle_url_lyrics(
                    url,
                    output,
                    output_default,
                    log_path,
                    cache_dir,
                )
                .await;
            }
            // 校验：--output 和 --output-default 不能同时使用
            if output.is_some() && output_default {
                eprintln!("错误: --output 和 --output-default 不能同时使用");
                std::process::exit(1);
            }
            // 如果启用了日志但未指定路径，使用默认路径
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
