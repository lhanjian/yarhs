//! Log writer module
//!
//! Provides thread-safe log writing to files or stdout/stderr.
//! Supports runtime reconfiguration of log file paths.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

/// Global log writer instance
static LOG_WRITER: OnceLock<LogWriter> = OnceLock::new();

/// Log output target
enum LogTarget {
    /// Write to stdout
    Stdout,
    /// Write to stderr
    Stderr,
    /// Write to file
    File(Mutex<File>),
}

/// Thread-safe log writer
pub struct LogWriter {
    /// Access log target
    access: Mutex<LogTarget>,
    /// Error log target
    error: Mutex<LogTarget>,
}

impl LogWriter {
    /// Create a new log writer with optional file paths
    fn new(access_log_file: Option<&str>, error_log_file: Option<&str>) -> io::Result<Self> {
        let access = match access_log_file {
            Some(path) => {
                let file = open_log_file(path)?;
                LogTarget::File(Mutex::new(file))
            }
            None => LogTarget::Stdout,
        };

        let error = match error_log_file {
            Some(path) => {
                let file = open_log_file(path)?;
                LogTarget::File(Mutex::new(file))
            }
            None => LogTarget::Stderr,
        };

        Ok(Self {
            access: Mutex::new(access),
            error: Mutex::new(error),
        })
    }

    /// Write to access log
    pub fn write_access(&self, message: &str) {
        let target = self.access.lock().unwrap();
        write_to_target(&target, message);
    }

    /// Write to error log
    pub fn write_error(&self, message: &str) {
        let target = self.error.lock().unwrap();
        write_to_target(&target, message);
    }

    /// Write info message (to access log target)
    pub fn write_info(&self, message: &str) {
        let target = self.access.lock().unwrap();
        write_to_target(&target, message);
    }

    /// Update access log file path (for runtime reconfiguration)
    pub fn set_access_log_file(&self, path: Option<&str>) -> io::Result<()> {
        let mut target = self.access.lock().unwrap();
        *target = match path {
            Some(p) => {
                let file = open_log_file(p)?;
                LogTarget::File(Mutex::new(file))
            }
            None => LogTarget::Stdout,
        };
        Ok(())
    }

    /// Update error log file path (for runtime reconfiguration)
    pub fn set_error_log_file(&self, path: Option<&str>) -> io::Result<()> {
        let mut target = self.error.lock().unwrap();
        *target = match path {
            Some(p) => {
                let file = open_log_file(p)?;
                LogTarget::File(Mutex::new(file))
            }
            None => LogTarget::Stderr,
        };
        Ok(())
    }
}

/// Open or create a log file for appending
fn open_log_file(path: &str) -> io::Result<File> {
    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    OpenOptions::new().create(true).append(true).open(path)
}

/// Write message to log target
fn write_to_target(target: &LogTarget, message: &str) {
    match target {
        LogTarget::Stdout => {
            println!("{message}");
        }
        LogTarget::Stderr => {
            eprintln!("{message}");
        }
        LogTarget::File(file) => {
            if let Ok(mut f) = file.lock() {
                let _ = writeln!(f, "{message}");
            }
        }
    }
}

/// Initialize the global log writer
///
/// This should be called once at application startup.
/// Returns error if log files cannot be opened.
pub fn init(access_log_file: Option<&str>, error_log_file: Option<&str>) -> io::Result<()> {
    let writer = LogWriter::new(access_log_file, error_log_file)?;
    LOG_WRITER.set(writer).map_err(|_| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Log writer already initialized",
        )
    })
}

/// Get the global log writer
///
/// Panics if `init()` has not been called.
pub fn get() -> &'static LogWriter {
    LOG_WRITER
        .get()
        .expect("Log writer not initialized. Call logger::writer::init() first.")
}

/// Check if the log writer has been initialized
pub fn is_initialized() -> bool {
    LOG_WRITER.get().is_some()
}
