fn main() {
    #[cfg(target_os = "windows")]
    {
        // Enforce Windows console subsystem icon linking via winres
        let mut res = winres::WindowsResource::new();
        res.set_icon("icons/icon.ico");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile winres icon for ragasukma-tui: {e}");
        }
    }
}
