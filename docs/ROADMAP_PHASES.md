# Roadmap & Fase Pengerjaan Ragasukma TUI 🗺️

Dokumen ini memetakan tahapan implementasi bertahap (*phased rollout*) dari awal hingga rilis stabil `ragasukma-tui`.

---

## 📋 Ikhtisar Fase

| Fase | Nama Fase | Fokus Utama | Target Deliverable |
| :---: | :--- | :--- | :--- |
| **Phase 1** | **Scaffolding & Event Loop** | Setup proyek Rust, crossterm raw mode, tokio loop, panic hooks | Binary dasar dapat jalan, responsive resize & graceful exit |
| **Phase 2** | **Navigation & Frame Harness** | Header tabs, status bar, routing antar tampilan, state machine | Navigasi tab 1-5, keyboard navigation, base theme styling |
| **Phase 3** | **Provider & Quota Dashboards** | Porting logic Antigravity, Codex, dan Claude views | Tampilan card akun, gauges kuota, model table, refresh key |
| **Phase 4** | **Custom LLM Management** | Porting `CustomLlmTab.tsx` ke TUI table & interactive form | Form CRUD provider (prefix, name, url, key), SQLite persistence |
| **Phase 5** | **Agent Chat & Tools Workspace** | Porting `AgentTab`, `ToolCards`, file explorer, streaming chat | Interactive chat pane, markdown streaming, inspectable tool calls |
| **Phase 6** | **Polish, Modals & Release** | Help overlay, error boundaries, memory & clippy check | Produksi binary siap rilis, clippy clean, dokumentasi user |

---

## 🛠️ Rincian per Fase

### 🚀 Phase 1: Scaffolding & Terminal Harness
**Tujuan**: Membangun fondasi runtime TUI yang kokoh, bebas panic leak di terminal, dan mengintegrasikan asynchronous event channel.

- [x] Inisialisasi `ragasukma-tui/Cargo.toml` dengan dependensi:
  - `ratatui` (v0.29+)
  - `crossterm` (v0.28+)
  - `tokio` (runtime async)
  - `ragasukma-core` (path crate local)
  - `color-eyre` / `anyhow` (error handling & panic safety)
  - `tui-textarea` atau internal text editor input widget
- [x] Implementasi initialization & cleanup:
  - `terminal::enable_raw_mode()` & `EnterAlternateScreen`
  - Panic hook untuk merestorasi terminal saat crash (mencegah shell rusak)
  - Signal handling (`Ctrl+C`, `SIGTERM`)
- [x] Implementasi Core Event Loop:
  - Background task pembaca event crossterm (`Event::Key`, `Event::Resize`, `Event::Mouse`)
  - Periodic tick sender (250ms tick rate untuk animasi & polling)
  - MPSC channel dispatcher

---

### 🧭 Phase 2: Layout Base, Navigation & Status Bar
**Tujuan**: Mengimplementasikan skeleton UI umum yang membungkus semua tab dan navigasi aplikasi.

- [x] Membuat root layout Ratatui:
  - **Top Bar**: Judul aplikasi, Tab Switcher (`[1] Agent`, `[2] Antigravity`, `[3] Codex`, `[4] Claude`, `[5] Custom LLM`, `[?] Help`), status connection badge.
  - **Main Viewport Area**: Dinamis berganti sesuai tab aktif.
  - **Bottom Status Bar**: Mode indikator (`NORMAL`, `INPUT`, `BUSY`), info model aktif, token budget, shortcut hints.
- [x] State Machine Navigasi:
  - Routing via hotkeys `1` s/d `5` dan `Tab` / `Shift+Tab`.
  - Penanganan fokus keyboard (`FocusManager`).
- [x] Theme & Styling Palette:
  - Cyan theme matching antarmuka `ai-voice/web` (Cyan, Surface Dark, Card Border, Gray Muted, Emerald untuk active/success, Red/Amber untuk errors).

---

### 📊 Phase 3: Provider & Quota Dashboards
**Tujuan**: Mengonversi tampilan visual kuota provider dari web ke terminal widgets.

- [x] **Antigravity View**:
  - Account cards: Nama akun, email, tier plan.
  - Quota visualizer: `ratatui::widgets::Gauge` untuk persentase sisa kuota.
  - TPM / RPM bar chart / statistics table.
- [x] **Codex View**:
  - Quota reset countdown indicator.
  - Model status table (daftar model OpenAI yang tersedia & rate limit).
- [x] **Claude View**:
  - Quota and cost statistics.
  - Session token counter & threshold warnings.
- [x] Provider Actions:
  - Tombol / hotkey `r` untuk memicu fetch kuota real-time secara asynchronous di background.

---

### 🔌 Phase 4: Custom LLM Manager Tab
**Tujuan**: Menghadirkan kapabilitas manajemen endpoint kustom OpenAI-compatible seperti pada `CustomLlmTab.tsx`.

- [x] Tabel Provider Terdaftar (`ratatui::widgets::Table`):
  - Kolom: `Prefix`, `Account Name`, `Base URL`, `Status` (Enabled/Disabled).
  - Seleksi baris dengan `j`/`k` atau tombol panah.
- [x] Aksi Cepat:
  - `Space`: Toggle enable / disable akun.
  - `d`: Konfirmasi hapus akun.
  - `e`: Masuk mode edit akun terpilih.
  - `n`: Masuk mode tambah akun baru.
- [x] Interactive Input Form Widget:
  - Input field interaktif: `Prefix`, `Name`, `Base URL`, `API Key`, `Enabled`.
  - Validasi prefix (karakter valid `a-z0-9_-`, tidak boleh bentrok dengan prefix sistem `ag`, `cx`, `cc`).
  - Integrasi dengan SQLite DB via repository `ragasukma-core`.

---

### 🤖 Phase 5: Autonomous Agent Workspace & Tool Inspection
**Tujuan**: Mengimplementasikan antarmuka utama chat agent, file explorer, dan visualisasi eksekusi tool.

- [x] **Split-Screen Workspace Layout**:
  - Kolom Kiri: Workspace File Tree Explorer (`useWorkspaceFiles.ts` port) + Session list.
  - Kolom Kanan / Utama: Chat Stream Panel + Tool Call Accordion + Multiline Prompt Input.
- [x] **Chat History & Markdown Streaming**:
  - Menampilkan bubble pesan User dan Assistant.
  - Streaming token rendering secara halus saat LLM merespons.
  - Formatting markdown (bold, code blocks, lists).
- [x] **Tool Call Execution Card** (seperti `ToolCards.tsx`):
  - Menampilkan blok tool call saat agent memanggil tools (`workspace_read_file`, `workspace_write_file`, `workspace_run_command`, dll.).
  - Indikator status warna: `Running` (kuning/berkedip), `Success` (hijau), `Error` (merah).
  - Expandable view untuk melihat argumen input dan output hasil eksekusi tool.
- [x] **Interactive Prompt Bar**:
  - Mode input teks multiline dengan shortcut `Ctrl+S` untuk submit.
  - Tombol batal / interrupt turn agent jika sedang berjalan.

---

### 💎 Phase 6: Interactive Modals, Polish & Optimization
**Tujuan**: Penyempurnaan UX, handling edge cases, dan quality assurance.

- [x] **Modal Overlay Layer**:
  - Quick Help / Keybinding cheatsheet (`?`).
  - Error alert dialog popup saat koneksi/tool gagal.
  - Confirmation dialog popup saat aksi destruktif (misal clear conversation atau delete custom provider).
- [x] **Code Quality & Linting**:
  - Menjalankan `cargo clippy --bin ragasukma-tui` dengan strict flags (sesuai skill Code Check).
  - Memastikan tidak ada memory leak atau unhandled panic.
- [x] **Pengujian & Dokumentasi Pengguna**:
  - Menulis panduan instalasi dan binary build di `README.md`.

---

## 🚀 Integrasi Lib `ragasukma-core` (Completed)

Dokumentasi rancangan teknis dan roadmap detail integrasi:
👉 **[`docs/PHASE2_INTEGRATION_CORE.md`](PHASE2_INTEGRATION_CORE.md)**

Status Milestone Integrasi Core:
- [x] **Milestone 2.1**: Cargo Linkage & Database Bootstrap (`sqlx` + SQLite auto-migration).
- [x] **Milestone 2.2**: Channel Bridge & Background Worker (`CoreBridge`, `CoreBridgeCommand`, `BridgeSessionEvent`).
- [x] **Milestone 2.3**: Live Agent Workspace Chat Execution (`execute_session_turn` + token & thought stream via `append_chunk` & `append_thought`).
- [x] **Milestone 2.4**: Real Quota & Custom LLM Persistence (Sinkronisasi Antigravity, Codex, Claude & SQLite providers).
- [x] **Milestone 2.5**: Error Handling, Broadcast Listener & Terminal Loop Safety.