use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
use crate::platform::{get_log_path, ensure_dir_exists};

const MAX_LOG_SIZE: u64 = 30 * 1024;

pub struct Logger {
    enabled: Mutex<bool>,
    file_path: Mutex<PathBuf>,
}

impl Logger {
    pub fn new() -> Self {
        Logger {
            enabled: Mutex::new(false),
            file_path: Mutex::new(get_log_path()),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Logger {
            enabled: Mutex::new(false),
            file_path: Mutex::new(path),
        }
    }

    pub fn enable(&self) {
        if let Ok(mut enabled) = self.enabled.lock() {
            *enabled = true;
        }
    }

    pub fn disable(&self) {
        if let Ok(mut enabled) = self.enabled.lock() {
            *enabled = false;
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.lock().map(|e| *e).unwrap_or(false)
    }

    pub fn set_path(&self, path: PathBuf) {
        if let Ok(mut file_path) = self.file_path.lock() {
            *file_path = path;
        }
    }

    fn write_log(&self, content: &str) {
        if !self.is_enabled() {
            return;
        }

        let file_path = match self.file_path.lock() {
            Ok(path) => path.clone(),
            Err(_) => return,
        };

        if let Ok(metadata) = std::fs::metadata(&file_path) {
            if metadata.len() >= MAX_LOG_SIZE {
                if let Ok(mut file) = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(&file_path)
                {
                    let _ = file.write_all(format!("[LOG ROTATED - Size limit reached]\n").as_bytes());
                }
            }
        }

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
        {
            let _ = file.write_all(content.as_bytes());
        }
    }

    pub fn log_request(&self, method: &str, url: &str, params: Option<&str>) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let params_str = params.unwrap_or("None");
        let log_entry = format!(
            "[{}] REQUEST: {} {}\n  Params: {}\n",
            timestamp, method, url, params_str
        );
        self.write_log(&log_entry);
    }

    pub fn log_request_with_timer(&self, method: &str, url: &str, params: Option<&str>) -> Instant {
        self.log_request(method, url, params);
        Instant::now()
    }

    pub fn log_response(&self, status: u16, url: &str, duration_ms: u64, response_preview: Option<&str>) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let preview = response_preview.unwrap_or("N/A");
        let preview_truncated: String = preview.chars().take(200).collect();
        let log_entry = format!(
            "[{}] RESPONSE: {} {} - {}ms\n  Status: {}\n  Preview: {}\n",
            timestamp, url, if status < 400 { "SUCCESS" } else { "FAILED" }, duration_ms, status, preview_truncated
        );
        self.write_log(&log_entry);
    }

    pub fn log_error(&self, context: &str, error: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_entry = format!(
            "[{}] ERROR: {}\n  Message: {}\n",
            timestamp, context, error
        );
        self.write_log(&log_entry);
    }

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

pub fn init_logger_with_path(path: Option<&str>) {
    let log_path = path
        .map(PathBuf::from)
        .unwrap_or_else(get_log_path);

    ensure_dir_exists(&log_path.parent().unwrap_or(&PathBuf::from(".")).to_path_buf())
        .unwrap_or(());

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path);

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

pub fn init_logger_with_pathbuf(path: Option<PathBuf>) {
    let log_path = path.unwrap_or_else(get_log_path);

    ensure_dir_exists(&log_path.parent().unwrap_or(&PathBuf::from(".")).to_path_buf())
        .unwrap_or(());

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path);

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
