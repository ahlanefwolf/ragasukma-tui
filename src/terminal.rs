use color_eyre::Result;
use crossterm::{
    event::{
        DisableBracketedPaste, EnableBracketedPaste, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{Stdout, stdout};
use std::panic;

pub type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

/// True when the terminal accepted the keyboard-enhancement (kitty keyboard
/// protocol) request. Only then can the app report modifier info distinctly
/// (e.g. the Alt+J newline shortcut) instead of collapsing it into a raw key.
static KITTY_FLAGS_ACTIVE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Whether enhanced key reporting is active (for diagnostics / debug overlay).
pub fn kitty_flags_active() -> bool {
    KITTY_FLAGS_ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
}

/// Install custom panic hook to always restore terminal before crashing
pub fn install_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore();
        tracing::error!("FATAL PANIC: {:#?}", panic_info);
        let panic_msg = format!("\n[FATAL PANIC] {:#?}\n", panic_info);

        #[cfg(debug_assertions)]
        {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("tui_panic.log")
            {
                use std::io::Write;
                let _ = file.write_all(panic_msg.as_bytes());
            }
        }

        // Always write panic log to system cache dir
        let cache_logs = ragasukma_core::config::get_app_cache_dir().join("logs");
        let _ = std::fs::create_dir_all(&cache_logs);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(cache_logs.join("tui_panic.log"))
        {
            use std::io::Write;
            let _ = file.write_all(panic_msg.as_bytes());
        }

        original_hook(panic_info);
    }));
}

/// Initialize terminal in raw mode and alternate screen, with panic hooks installed.
pub fn init() -> Result<TuiTerminal> {
    install_panic_hook();

    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableBracketedPaste)?;

    // Try to enable the kitty keyboard protocol so modifier info is reported
    // distinctly (needed for the Alt+J newline shortcut).
    // We request DISAMBIGUATE_ESCAPE_CODES only: it does NOT turn Ctrl+H into
    // Backspace or alter text input; it just adds modifier info to raw keys.
    let ok = matches!(
        execute!(out, PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)),
        Ok(())
    );
    KITTY_FLAGS_ACTIVE.store(ok, std::sync::atomic::Ordering::Relaxed);
    tracing::info!("keyboard enhancement (kitty protocol) active: {ok}");

    let backend = CrosstermBackend::new(out);
    let terminal = Terminal::new(backend)?;

    Ok(terminal)
}

/// Restore terminal to normal mode and leave alternate screen.
pub fn restore() -> Result<()> {
    ratatui::restore();
    let mut out = stdout();
    if KITTY_FLAGS_ACTIVE.load(std::sync::atomic::Ordering::Relaxed) {
        let _ = execute!(out, PopKeyboardEnhancementFlags);
        KITTY_FLAGS_ACTIVE.store(false, std::sync::atomic::Ordering::Relaxed);
    }
    let _ = execute!(out, LeaveAlternateScreen, DisableBracketedPaste);
    let _ = disable_raw_mode();
    Ok(())
}
