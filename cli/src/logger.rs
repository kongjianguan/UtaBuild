//! 日志模块：提供文件日志和 tracing 日志初始化功能

use crate::platform::{ensure_dir_exists, get_log_path};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

/// 日志文件最大大小（30KB），超过后轮转
const MAX_LOG_SIZE: u64 = 30 * 1024;

/// 文件日志记录器，支持开关控制和日志轮转
pub struct Logger {
    /// 是否启用日志
    enabled: Mutex<bool>,
    /// 日志文件路径
    file_path: Mutex<PathBuf>,
}

impl Logger {
    /// 创建新的日志记录器，使用默认路径
    pub fn new() -> Self {
        Logger {
            enabled: Mutex::new(false),
            file_path: Mutex::new(get_log_path()),
        }
    }

    /// 创建指定路径的日志记录器
    pub fn with_path(path: PathBuf) -> Self {
        Logger {
            enabled: Mutex::new(false),
            file_path: Mutex::new(path),
        }
    }

    /// 启用日志记录
    pub fn enable(&self) {
        if let Ok(mut enabled) = self.enabled.lock() {
            *enabled = true;
        }
    }

    /// 禁用日志记录
    pub fn disable(&self) {
        if let Ok(mut enabled) = self.enabled.lock() {
            *enabled = false;
        }
    }

    /// 检查日志是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled.lock().map(|e| *e).unwrap_or(false)
    }

    /// 设置日志文件路径
    pub fn set_path(&self, path: PathBuf) {
        if let Ok(mut file_path) = self.file_path.lock() {
            *file_path = path;
        }
    }

    /// 写入日志内容（含日志轮转逻辑）
    fn write_log(&self, content: &str) {
        if !self.is_enabled() {
            return;
        }

        let file_path = match self.file_path.lock() {
            Ok(path) => path.clone(),
            Err(_) => return,
        };

        // 检查文件大小，超过限制则清空重写
        if let Ok(metadata) = std::fs::metadata(&file_path) {
            if metadata.len() >= MAX_LOG_SIZE {
                if let Ok(mut file) = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(&file_path)
                {
                    let _ = file.write_all(b"[LOG ROTATED - Size limit reached]\n");
                }
            }
        }

        // 追加写入日志
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
        {
            let _ = file.write_all(content.as_bytes());
        }
    }

    /// 记录 HTTP 请求日志
    pub fn log_request(&self, method: &str, url: &str, params: Option<&str>) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let params_str = params.unwrap_or("None");
        let log_entry = format!(
            "[{}] REQUEST: {} {}\n  Params: {}\n",
            timestamp, method, url, params_str
        );
        self.write_log(&log_entry);
    }

    /// 记录请求并返回计时器
    pub fn log_request_with_timer(&self, method: &str, url: &str, params: Option<&str>) -> Instant {
        self.log_request(method, url, params);
        Instant::now()
    }

    /// 记录 HTTP 响应日志
    pub fn log_response(
        &self,
        status: u16,
        url: &str,
        duration_ms: u64,
        response_preview: Option<&str>,
    ) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let preview = response_preview.unwrap_or("N/A");
        let preview_truncated: String = preview.chars().take(200).collect();
        let log_entry = format!(
            "[{}] RESPONSE: {} {} - {}ms\n  Status: {}\n  Preview: {}\n",
            timestamp,
            url,
            if status < 400 { "SUCCESS" } else { "FAILED" },
            duration_ms,
            status,
            preview_truncated
        );
        self.write_log(&log_entry);
    }

    /// 记录错误日志
    pub fn log_error(&self, context: &str, error: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_entry = format!("[{}] ERROR: {}\n  Message: {}\n", timestamp, context, error);
        self.write_log(&log_entry);
    }

    /// 记录信息日志
    pub fn log_info(&self, message: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_entry = format!("[{}] INFO: {}\n", timestamp, message);
        self.write_log(&log_entry);
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

/// 初始化 tracing 日志系统（指定日志级别，输出到 stderr）
pub fn init_logger(level: &str) {
    let level = match level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_level(true)
                .with_filter(tracing_subscriber::filter::LevelFilter::from(level)),
        )
        .init();
}

/// 初始化 tracing 日志系统（输出到日志文件，路径由字符串指定）
pub fn init_logger_with_path(path: Option<&str>) {
    let log_path = path.map(PathBuf::from).unwrap_or_else(get_log_path);

    ensure_dir_exists(
        &log_path
            .parent()
            .unwrap_or(&PathBuf::from("."))
            .to_path_buf(),
    )
    .unwrap_or(());

    let file = OpenOptions::new().create(true).append(true).open(&log_path);

    if let Ok(file) = file {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_thread_ids(false)
                    .with_level(true)
                    .with_writer(std::sync::Mutex::new(file))
                    .with_filter(tracing_subscriber::filter::LevelFilter::INFO),
            )
            .init();
    }
}

/// 初始化 tracing 日志系统（输出到日志文件，路径由 PathBuf 指定）
pub fn init_logger_with_pathbuf(path: Option<PathBuf>) {
    let log_path = path.unwrap_or_else(get_log_path);

    ensure_dir_exists(
        &log_path
            .parent()
            .unwrap_or(&PathBuf::from("."))
            .to_path_buf(),
    )
    .unwrap_or(());

    let file = OpenOptions::new().create(true).append(true).open(&log_path);

    if let Ok(file) = file {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_thread_ids(false)
                    .with_level(true)
                    .with_writer(std::sync::Mutex::new(file))
                    .with_filter(tracing_subscriber::filter::LevelFilter::INFO),
            )
            .init();
    }
}
