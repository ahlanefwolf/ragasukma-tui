use std::io::BufRead;
use std::process::{Command, Stdio};

/// Launch browser for OAuth flow while redirecting stdout and stderr
/// (such as Chrome zygote process errors and sandbox warnings) directly to tracing.
pub fn open_browser_oauth(url: &str) {
    tracing::info!("Opening OAuth authorization URL in browser: {url}");

    #[cfg(target_os = "linux")]
    let mut cmd = Command::new("xdg-open");
    #[cfg(target_os = "linux")]
    cmd.arg(url);

    #[cfg(target_os = "macos")]
    let mut cmd = Command::new("open");
    #[cfg(target_os = "macos")]
    cmd.arg(url);

    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    };

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    match cmd.spawn() {
        Ok(mut child) => {
            if let Some(stdout) = child.stdout.take() {
                std::thread::spawn(move || {
                    let reader = std::io::BufReader::new(stdout);
                    for line in reader.lines().map_while(Result::ok) {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            tracing::debug!("[Browser stdout] {trimmed}");
                        }
                    }
                });
            }

            if let Some(stderr) = child.stderr.take() {
                std::thread::spawn(move || {
                    let reader = std::io::BufReader::new(stderr);
                    for line in reader.lines().map_while(Result::ok) {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            if trimmed.to_lowercase().contains("zygote")
                                || trimmed.to_lowercase().contains("error")
                            {
                                tracing::error!("[Browser/Zygote] {trimmed}");
                            } else {
                                tracing::warn!("[Browser stderr] {trimmed}");
                            }
                        }
                    }
                });
            }

            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(e) => {
            tracing::warn!("Primary browser command failed ({e}), falling back to `open` crate...");
            let mut launched = false;
            for mut fallback_cmd in open::commands(url) {
                fallback_cmd.stdout(Stdio::piped());
                fallback_cmd.stderr(Stdio::piped());
                if let Ok(mut child) = fallback_cmd.spawn() {
                    if let Some(stderr) = child.stderr.take() {
                        std::thread::spawn(move || {
                            let reader = std::io::BufReader::new(stderr);
                            for line in reader.lines().map_while(Result::ok) {
                                let trimmed = line.trim();
                                if !trimmed.is_empty() {
                                    if trimmed.to_lowercase().contains("zygote")
                                        || trimmed.to_lowercase().contains("error")
                                    {
                                        tracing::error!("[Browser/Zygote] {trimmed}");
                                    } else {
                                        tracing::warn!("[Browser stderr] {trimmed}");
                                    }
                                }
                            }
                        });
                    }
                    std::thread::spawn(move || {
                        let _ = child.wait();
                    });
                    launched = true;
                    break;
                }
            }
            if !launched {
                tracing::error!("Failed to launch browser for OAuth: {e}");
            }
        }
    }
}
