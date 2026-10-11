//! log.txt next to the exe, like the C# version's log4net file.

use std::io::Write;
use std::sync::Mutex;

struct FileLog(Mutex<std::fs::File>);

impl log::Log for FileLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) || !record.target().starts_with("ram") {
            return;
        }
        if let Ok(mut file) = self.0.lock() {
            let _ = writeln!(file, "{} {:<5} {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S,%3f"), record.level(), record.args());
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.0.lock() {
            let _ = file.flush();
        }
    }
}

/// A fresh log.txt every start (it can hold account names, it shouldn't grow forever)
pub fn init(dir: &std::path::Path) {
    if let Ok(file) = std::fs::File::create(dir.join("log.txt")) {
        if log::set_boxed_logger(Box::new(FileLog(Mutex::new(file)))).is_ok() {
            log::set_max_level(log::LevelFilter::Info);
        }
    }
}
