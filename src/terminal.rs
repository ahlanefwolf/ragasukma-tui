use color_eyre::Result;
use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Stdout};
use std::panic;

pub type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

/// Install custom panic hook to always restore terminal before crashing
pub fn install_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore();
        tracing::error!("FATAL PANIC: {:#?}", panic_info);
        let panic_msg = format!("\n[FATAL PANIC] {:#?}\n", panic_info);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("ragasukma-tui.log")
        {
            let _ = std::io::Write::write_all(&mut file, panic_msg.as_bytes());
        }
        original_hook(panic_info);
    }));
}

/// Initialize terminal in raw mode and alternate screen, with panic hooks installed.
pub fn init() -> Result<TuiTerminal> {
    install_panic_hook();

    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture, cursor::Hide)?;

    let backend = CrosstermBackend::new(out);
    let terminal = Terminal::new(backend)?;

    Ok(terminal)
}

/// Restore terminal to normal mode and leave alternate screen.
pub fn restore() -> Result<()> {
    ratatui::restore();
    let mut out = stdout();
    let _ = execute!(out, LeaveAlternateScreen, DisableMouseCapture, cursor::Show);
    let _ = disable_raw_mode();
    Ok(())
}
