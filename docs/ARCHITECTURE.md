# Arsitektur Ragasukma TUI 🏛️

Dokumen ini mendeskripsikan desain arsitektur, pola concurrency, alur state management, dan integrasi antara antarmuka terminal `ragasukma-tui` dengan engine headless `ragasukma-core`.

---

## 1. Desain Tingkat Tinggi (High-Level Architecture)

`ragasukma-tui` mengadopsi pola **Component-Action Model** (serupa *The Elm Architecture / TCA* yang jamak digunakan dalam ekosistem modern Ratatui) yang dipadukan dengan **Multi-Producer Single-Consumer (MPSC)** channel async Tokio.

```text
               ┌────────────────────────────────────────────────────────┐
               │                     Terminal Host                      │
               │        (crossterm: Raw Mode, Alternate Screen)         │
               └───────────────▲────────────────────────┬───────────────┘
                               │ (Draw Frame)           │ (Key/Mouse Events)
                               │                        ▼
               ┌───────────────┴────────────────────────────────────────┐
               │                  App Event Loop (Main)                 │
               │       - Handles Ticks & Render Scheduling              │
               │       - Dispatches Actions to Active Components        │
               └───────────────▲────────────────────────▲───────────────┘
                               │                        │
       (Agent/LLM Events)      │                        │ (Background Tasks)
                               │                        │
┌──────────────────────────────┴───────┐   ┌────────────┴──────────────────────────┐
│      ragasukma-core Engine           │   │         Provider Telemetry            │
│  - AgentTurn Execution               │   │  - Antigravity / Codex / Claude Quota │
│  - Tool Registry Invocation          │   │  - Token Bucket & Cost Monitoring     │
│  - Context Compression & Rollback    │   │  - SQLite Database Migrations & Sync  │
└──────────────────────────────────────┘   └───────────────────────────────────────┘
```

---

## 2. Event Loop & Pola Concurrency

Untuk memastikan UI terminal tetap responsif pada 60 FPS tanpa freezing saat memanggil LLM atau menjalankan tool yang memakan waktu (misal `workspace_run_command` atau crawling file), sistem memisahkan **Render Loop** dari **Task Execution**.

### Tipe Event (`AppEvent`)

```rust
pub enum AppEvent {
    /// Input keyboard atau mouse dari Crossterm
    Input(crossterm::event::KeyEvent),
    /// Tick timer periodik (untuk spinner, cursor blink, periodic refresh)
    Tick,
    /// Resize terminal window
    Resize(u16, u16),
    /// Event stream dari Agent Runtime (ragasukma-core)
    Agent(AgentEvent),
    /// Update kuota provider LLM
    ProviderQuotaUpdated(ProviderQuotaPayload),
    /// Notifikasi Toast atau Error banner
    Notify(Notification),
}

pub enum AgentEvent {
    TokenChunk(String),
    ToolStarted { call_id: String, tool_name: String, args: serde_json::Value },
    ToolFinished { call_id: String, tool_name: String, result: String, is_err: bool },
    TurnCompleted { total_tokens: usize },
    TurnFailed(String),
}
```

### Channel Architecture

1. **`event_tx` / `event_rx` (`tokio::sync::mpsc::unbounded_channel`)**:
   Menampung seluruh masukan aksi luar, event keyboard, dan update background worker ke event loop utama.
2. **Terminal Guard & Panic Hook**:
   Terminal diinisialisasi dalam `crossterm::terminal::enable_raw_mode()` dan `EnterAlternateScreen`. Hook panic diinstal (`better-panic` atau `color-eyre`) untuk memastikan jika terjadi panic, mode terminal dikembalikan ke mode normal (`disable_raw_mode()` & `LeaveAlternateScreen`) agar shell pengguna tidak rusak/hang.

---

## 3. Komponen & Struktur State

Aplikasi dibagi menjadi modul-modul independen yang mengimplementasikan trait `Component`:

```rust
pub trait Component {
    /// Menangani event input atau sistem, menghasilkan opsi Action lanjutan
    fn handle_event(&mut self, event: &AppEvent) -> Result<Option<Action>>;
    
    /// Update state internal komponen berdasarkan Action
    fn update(&mut self, action: Action) -> Result<()>;
    
    /// Merender komponen ke frame buffer Ratatui
    fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState);
}
```

### Hirarki Komponen Utama

```text
App (Root Coordinator)
 ├── HeaderTabBar (Tabs: Agent, Antigravity, Codex, Claude, Custom LLM)
 ├── Viewport (Active View Container)
 │    ├── AgentWorkspaceView
 │    │    ├── SessionSidebar (List session riwayat + active badge)
 │    │    ├── FileTreeSidebar (Struktur folder workspace aktif)
 │    │    ├── ChatHistoryPane (Daftar bubble pesan markdown & live streaming)
 │    │    ├── ToolInspectionPane (Accordion detail eksekusi tool call)
 │    │    └── PromptInputWidget (Multiline textarea, submit keybinds)
 │    ├── AntigravityView (Card akun, quota gauge, stats token)
 │    ├── CodexView (Card akun, TPM/RPM limits, model availability)
 │    ├── ClaudeView (Card akun, session token counter, cost breakdown)
 │    └── CustomLlmView (Tabel akun terdaftar, Form popup tambah/edit)
 ├── ModalOverlayHost (Help menu, confirmation dialogs, error alerts)
 └── BottomStatusBar (Current mode, active model, token counter, hotkey hints)
```

---

## 4. Integrasi dengan `ragasukma-core`

`ragasukma-tui` menggunakan `ragasukma-core` sebagai dependensi path crate (`ragasukma-core = { path = "../ragasukma-core" }`).

### 1. Eksekusi Autonomous Agent
Saat pengguna mengetik prompt dan menekan `Ctrl+S`:
1. UI TUI mem-bundle riwayat pesan dan prompt aktif menjadi objek turn input.
2. Memanggil asynchronous task `ragasukma_core::agent::executor::execute_session_turn`.
3. Event stream (token streaming, tool call execution) diteruskan langsung via unbounded channel ke UI thread untuk di-render secara incremental.

### 2. Provider Hub & Quota Management
Dashboard Antigravity, Codex, dan Claude membaca konfigurasi dan status token dari `ragasukma_core::llm::providers` dan database SQLite lokal (`ragasukma_core::repository`).

### 3. Custom LLM Persistence
Custom OpenAI-compatible provider disimpan dalam tabel SQLite melalui repository `ragasukma-core`, sehingga konfigurasi provider konsisten dan tersinkronisasi jika pengguna beralih antara UI Web dan TUI Terminal.

---

## 5. Rendering Pipeline & Optimalisasi

- **Partial Invalidation & Diff Rendering**: Memanfaatkan double buffering bawaan Ratatui sehingga hanya sel terminal yang berubah yang ditulis ulang ke `stdout`.
- **Markdown Text Spans**: Parser ringan mengonversi syntax Markdown (inline code, header `#`, bold `**`, list `*`) ke `ratatui::text::Line` dan `Span` dengan warna ANSI terkonfigurasi.
- **Dynamic Resizing**: Setiap kali ukuran terminal berubah (`Event::Resize`), layout grid dihitung ulang secara dinamis menggunakan `ratatui::layout::Layout` dengan flex constraints (`Constraint::Length`, `Constraint::Min`, `Constraint::Percentage`).
