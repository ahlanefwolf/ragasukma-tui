# Ragasukma TUI 🖥️⚡

> Terminal User Interface (TUI) canggih bertenaga **Ratatui** untuk autonomous agent runtime, tool registry, dan multi-provider LLM engine yang terhubung langsung dengan [`ragasukma-core`](../ragasukma-core).

Diadaptasi dari antarmuka Web Dashboard AI-Voice (`ai-voice/web`) ke format konsol terminal berkinerja tinggi, ringan, dan keyboard-first.

---

## 🌟 Highlight Fitur

- ⚡ **Native Rust Performance**: Dibangun menggunakan `ratatui` + `crossterm` dan asynchronous runtime `tokio`.
- 🤖 **Autonomous Agent Workspace**: Chat stream real-time, inspectable tool calls, file tree explorer, dan context token counter.
- 📊 **Multi-Provider Quota Dashboard**: Monitoring visual (gauge bar, usage stats, limit RPM/TPM) untuk **Antigravity**, **Codex**, dan **Claude**.
- 🔌 **Custom OpenAI-Compatible Manager**: Registrasi, konfigurasi, toggle, dan modifikasi endpoint LLM eksternal (OpenRouter, local Ollama, vLLM, dll.) secara interaktif langsung dari terminal.
- ⌨️ **Vim-Inspired Keybindings**: Navigasi cepat dengan keyboard tanpa perlu mouse.

---

## 📐 Wireframe Tampilan Terminal (ASCII Mockup)

### 1. Agent Chat & Tool Execution Pane
```text
┌─ [1] Agent Workspace ── [2] Antigravity ── [3] Codex ── [4] Claude ── [5] Custom LLM ── [?] Help ───┐
│ 📁 Workspace: /workspace/ai-project         Model: claude-3-7-sonnet-thought   Status: [Streaming]   │
├──────────────────────┬──────────────────────────────────────────────────────────────────────────────┤
│ [Files]              │ [User]: Implement recursive directory walker with ignore patterns            │
│  ├─ src/             │                                                                              │
│  │  ├─ main.rs       │ [Assistant]: I will inspect the current directory structure first.           │
│  │  └─ lib.rs        │                                                                              │
│  ├─ Cargo.toml       │ ┌─ 🛠️ Tool Call: workspace_list_dir [SUCCESS] ─────────────────────────────┐ │
│  └─ README.md        │ │ path: "src" | depth: 2                                                    │ │
│                      │ │ Result: Found 2 files, 0 subdirectories.                                  │ │
│ [Sessions]           │ └───────────────────────────────────────────────────────────────────────────┘ │
│  ● Session #4 (curr) │                                                                              │
│  ○ Session #3        │ Here is the plan:                                                            │
│  ○ Session #2        │ 1. Add `ignore` crate to Cargo.toml...                                       │
├──────────────────────┴──────────────────────────────────────────────────────────────────────────────┤
│ > [Prompt Input]: create a test suite for file walker...                                            │
├─────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [Tab]: Switch Panes | [1-5]: Switch Tabs | [Ctrl+S]: Run Turn | [Esc]: Normal Mode | [q]: Quit      │
└─────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### 2. Quota & Providers View (Antigravity / Codex / Claude)
```text
┌─ [1] Agent Workspace ── [2] Antigravity ── [3] Codex ── [4] Claude ── [5] Custom LLM ── [?] Help ───┐
│ Provider: Codex Accounts (2 Active)                                          Last Synced: 12:45:00   │
├─────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ ┌─ [Account: primary_work] ───────────────────┐ ┌─ [Account: fallback_dev] ──────────────────────┐  │
│ │ Email: dev@company.com                      │ │ Email: personal@domain.com                     │  │
│ │ Status: ACTIVE                              │ │ Status: ACTIVE                                 │  │
│ │ Quota Remaining: [████████████░░░░] 75%     │ │ Quota Remaining: [████████████████░] 92%       │  │
│ │ TPM Limit: 80,000 / 100,000                 │ │ TPM Limit: 12,000 / 50,000                     │  │
│ │ Reset In: 4h 12m                            │ │ Reset In: 18h 05m                              │  │
│ └─────────────────────────────────────────────┘ └────────────────────────────────────────────────┘  │
│                                                                                                     │
│ [Models Status]                                                                                     │
│  Model ID                 Context Window    Input Cost    Output Cost   State                       │
│  gpt-4o                   128,000           $5.00/1M      $15.00/1M     Available                   │
│  o3-mini                  200,000           $1.10/1M      $4.40/1M      Available                   │
│                                                                                                     │
├─────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [r]: Refresh Quota | [a]: Add Account | [Tab]: Switch Tabs | [j/k]: Scroll | [q]: Quit              │
└─────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### 3. Custom OpenAI-Compatible LLM Tab
```text
┌─ [1] Agent Workspace ── [2] Antigravity ── [3] Codex ── [4] Claude ── [5] Custom LLM ── [?] Help ───┐
│ Registered Custom Providers (2 configured)                                                          │
├─────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Prefix   Name            Base URL                              Status     Actions                   │
│ ─────────────────────────────────────────────────────────────────────────────────────────────────── │
│ 9r       OpenRouter      https://openrouter.ai/api/v1          [ENABLED]  [Edit] [Disable] [Delete] │
│ ol       Local Ollama    http://localhost:11434/v1             [ENABLED]  [Edit] [Disable] [Delete] │
│                                                                                                     │
│ ┌─ [Add / Edit Provider] ────────────────────────────────────────────────────────────────────────┐  │
│ │ Prefix:   [ 9r                                                        ]                         │  │
│ │ Name:     [ OpenRouter                                                ]                         │  │
│ │ Base URL: [ https://openrouter.ai/api/v1                              ]                         │  │
│ │ API Key:  [ sk-or-v1-********************************                 ]                         │  │
│ │ Enabled:  [X] Yes                                                                               │  │
│ │                                                                                                 │  │
│ │ < Enter: Save Provider >    < Esc / c: Cancel >                                                 │  │
│ └─────────────────────────────────────────────────────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [n]: New Provider | [e]: Edit Selected | [d]: Delete | [Space]: Toggle Enable | [q]: Quit           │
└─────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## ⌨️ Global Keybindings

| Shortcut | Aksi |
| :--- | :--- |
| `1` - `5` | Pindah Tab secara langsung (Agent, Antigravity, Codex, Claude, Custom LLM) |
| `Tab` / `BackTab` | Pindah fokus antar panel / widget aktif |
| `j` / `k` atau `↑` / `↓` | Navigasi vertikal list / scroll chat |
| `i` | Masuk mode Input teks (pada chat prompt atau form) |
| `Esc` | Keluar dari input mode / tutup modal popup |
| `Enter` | Submit prompt (saat fokus input) atau Set Default Provider |
| `x` | Toggle detail Tool Call Accordion (pada Agent Workspace) |
| `n` | Sesi baru (Tab 1) / Tambah Provider Custom LLM baru (Tab 5) |
| `e` / `d` / `Space` | Edit / Hapus / Toggle Enable Provider terpilih (Tab 5) |
| `r` | Refresh status kuota provider (Tab 2-4) / List models (Tab 5) |
| `?` / `F1` | Buka / Tutup Interactive Help Cheatsheet Modal |
| `q` / `Ctrl+C` | Keluar dari aplikasi (graceful shutdown crossterm) |

---

## 🚀 Cara Menjalankan & Build

### Prasyarat
- **Rust Toolchain**: `cargo`, `rustc` (edisi 2021)
- Terminal emulator modern dengan dukungan 256 colors / truecolor (e.g. Alacritty, Kitty, WezTerm, iTerm2, Windows Terminal).

### Menjalankan Development Mode
```bash
# Dari folder ragasukma-tui
cargo run --bin ragasukma-tui
```

### Build Binary Release Optimized
```bash
cargo build --release --bin ragasukma-tui

# Binary executable siap pakai akan berada di:
# target/release/ragasukma-tui
```

### Menjalankan Unit Tests & Code Quality Check
```bash
# Unit & integration testing
cargo test

# Strict Clippy lint check
cargo clippy --bin ragasukma-tui -- -W dead_code -W unused -W clippy::redundant_clone -W clippy::panic
```

---

## 📚 Dokumentasi Lanjutan

- [**Arsitektur Sistem (ARCHITECTURE.md)**](docs/ARCHITECTURE.md)
- [**Roadmap Fase Pengerjaan (ROADMAP_PHASES.md)**](docs/ROADMAP_PHASES.md)
