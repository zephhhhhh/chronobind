use log::{Log, Metadata, Record};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct TuiLogLine {
    /// The log message.
    content: String,
    /// The log level message.
    level: log::Level,
}

impl TuiLogLine {
    /// Get the content of the log line.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the log level of the log line.
    #[must_use]
    pub const fn level(&self) -> log::Level {
        self.level
    }
}

// Global logger that outputs to the TUI debug window
pub struct TuiLogger {
    logs: Mutex<Vec<TuiLogLine>>,
}

impl TuiLogger {
    /// Maximum number of log lines to keep in memory.
    pub const MAX_LOG_SIZE: usize = 1000;
}

impl Log for TuiLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata())
            && let Ok(mut logs) = self.logs.lock()
        {
            let message = format!("[{}] {}", record.level(), record.args());
            for line in message.lines() {
                logs.insert(
                    0,
                    TuiLogLine {
                        content: line.to_string(),
                        level: record.level(),
                    },
                );
            }
            // Remove oldest logs (from the back) when max size is reached
            while logs.len() > Self::MAX_LOG_SIZE {
                logs.pop();
            }
        }
    }

    fn flush(&self) {}
}

/// Global TUI logger instance.
pub static TUI_LOGGER: TuiLogger = TuiLogger {
    logs: Mutex::new(Vec::new()),
};

/// Access TUI debug logs with a closure, returning None if the lock is poisoned.
pub fn with_debug_logs<R>(f: impl FnOnce(&[TuiLogLine]) -> R) -> Option<R> {
    TUI_LOGGER.logs.lock().ok().map(|logs| f(&logs))
}

/// Initialize the TUI logger with the specified maximum log level.
/// # Panics
/// This function will panic if the logger fails to initialize.
pub fn init_tui_logger(max_level: log::LevelFilter) {
    // Initialize the TUI logger
    log::set_logger(&TUI_LOGGER)
        .map(|()| log::set_max_level(max_level))
        .unwrap();
}
