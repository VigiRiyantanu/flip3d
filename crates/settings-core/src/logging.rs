// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct FileLogger {
    _log_path: PathBuf,
    file: Mutex<Option<File>>,
}

pub fn local_app_data_dir() -> PathBuf {
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
        });
    local_app_data.join("Flip3D")
}

pub fn log_dir() -> PathBuf {
    crate::paths::AppPaths::get().log_dir().to_path_buf()
}

pub fn get_log_file_path(app_name: &str) -> PathBuf {
    log_dir().join(format!("{}.log", app_name))
}

impl FileLogger {
    pub fn init(app_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let dir = log_dir();
        let _ = fs::create_dir_all(&dir);
        let log_path = get_log_file_path(app_name);

        // Rotate if larger than 5 MB
        if let Ok(metadata) = fs::metadata(&log_path) {
            if metadata.len() > 5 * 1024 * 1024 {
                let backup = dir.join(format!("{}.old.log", app_name));
                let _ = fs::remove_file(&backup);
                let _ = fs::rename(&log_path, &backup);
            }
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        let logger = Box::new(FileLogger {
            _log_path: log_path,
            file: Mutex::new(Some(file)),
        });

        let _ = log::set_boxed_logger(logger);
        log::set_max_level(log::LevelFilter::Debug);

        // Setup panic hook
        let app_name_clone = app_name.to_string();
        std::panic::set_hook(Box::new(move |panic_info| {
            let thread = std::thread::current();
            let thread_name = thread.name().unwrap_or("<unnamed>");
            let location = panic_info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "unknown location".to_string());
            let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                *s
            } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                s.as_str()
            } else {
                "Box<dyn Any>"
            };

            log::error!(
                "FATAL PANIC in [{}] thread '{}' at {}: {}",
                app_name_clone,
                thread_name,
                location,
                payload
            );
        }));

        Ok(())
    }
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Debug
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let now = std::time::SystemTime::now();
        let time_str = match now.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => {
                let secs = d.as_secs();
                let millis = d.subsec_millis();
                format!("{}.{:03}", secs, millis)
            }
            Err(_) => "0.000".to_string(),
        };

        let msg = format!("[{}][{}] {}\n", time_str, record.level(), record.args());

        if let Ok(mut guard) = self.file.lock() {
            if let Some(f) = guard.as_mut() {
                let _ = f.write_all(msg.as_bytes());
                let _ = f.flush();
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut guard) = self.file.lock() {
            if let Some(f) = guard.as_mut() {
                let _ = f.flush();
            }
        }
    }
}
