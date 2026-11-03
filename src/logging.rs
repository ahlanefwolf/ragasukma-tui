use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Multi-writer struct to duplicate log output to both local working directory and global config dir
struct MultiWriter {
    file_local: Option<std::fs::File>,
    file_global: Option<std::fs::File>,
}

impl std::io::Write for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(ref mut f) = self.file_local {
            let _ = f.write_all(buf);
        }
        if let Some(ref mut f) = self.file_global {
            let _ = f.write_all(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(ref mut f) = self.file_local {
            let _ = f.flush();
        }
        if let Some(ref mut f) = self.file_global {
            let _ = f.flush();
        }
        Ok(())
    }
}

/// Initialize a non-blocking file-based tracing subscriber.
/// This logs all errors, warnings, and diagnostic information to `ragasukma-tui.log`
/// in both the current working directory and ~/.config/ragasukma/logs/ragasukma-tui.log.
pub fn init_logging() -> WorkerGuard {
    let local_log_path = PathBuf::from("ragasukma-tui.log");
    let app_dir = ragasukma_core::config::get_app_config_dir();
    let log_dir = app_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let global_log_path = log_dir.join("ragasukma-tui.log");

    // Open logs in append mode so past warnings and errors are not lost across runs
    let file_local = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&local_log_path)
        .ok();

    let file_global = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&global_log_path)
        .ok();

    let multi_writer = MultiWriter {
        file_local,
        file_global,
    };

    let (non_blocking, guard) = tracing_appender::non_blocking(multi_writer);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,ragasukma_tui=trace,ragasukma_core=trace")
    });

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true),
        );

    subscriber.init();

    // Capture panics and log them as tracing errors
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unhandled panic payload".to_string()
        };

        tracing::error!(
            target: "ragasukma_tui::panic",
            location = %location,
            "CRITICAL PANIC: {payload}"
        );

        default_panic(info);
    }));

    tracing::info!(
        "=== Ragasukma TUI logging session started (target: {:?} and {:?}) ===",
        local_log_path,
        global_log_path
    );

    guard
}
