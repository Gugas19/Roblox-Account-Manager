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

/// A fresh log.txt every start (it can hold account names, it shouldn't grow forever). `keep` adds to the previous
/// one instead: the first start after an update, so how the update went stays in the log.
pub fn init(dir: &std::path::Path, keep: bool) {
    let path = dir.join("log.txt");
    let file = if keep { std::fs::OpenOptions::new().create(true).append(true).open(&path) } else { std::fs::File::create(&path) };
    if let Ok(file) = file {
        if log::set_boxed_logger(Box::new(FileLog(Mutex::new(file)))).is_ok() {
            log::set_max_level(log::LevelFilter::Info);
        }
    }
}
