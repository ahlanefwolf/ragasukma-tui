# 📋 Rencana Pengembangan Agent View Bertahap (Phased Rollout Plan)

Dokumen ini memetakan arsitektur, strategi implementasi, dan pembagian fase pengembangan modul **`ragasukma-tui/src/agent_view`** dengan mengacu pada fitur, UX, dan kapabilitas yang telah matang pada:
- **Web Frontend**: `@ai-voice/web/src/components/agent` (`AgentTab`, `AgentChatPane`, `ToolCards`, `SkillsAndMcpTabs`, `AgentWorkspaceHeader`, dll.)
- **Rust Backend**: `@ai-voice/src/agent` & `@ragasukma-core/src/agent` (`orchestrator`, `engine`, `context_builder`, `tools`, `diff approvals`, `mcp`).

---

## 🎯 Visi & Tujuan
Membangun antarmuka autonomous coding agent berbasis terminal (Ratatui TUI) yang:
1. **Ultra Responsif & Ringan**: Nol overhead browser, start instan, navigasi 100% keyboard-driven.
2. **Kaya Visual & Informasi**: Render bubble percakapan, reasoning stream, expandable tool execution card, file context tree, serta live status MCP & Skills.
3. **Aman & Terkendali**: Mendukung interactive diff approval review sebelum tool menulis ke disk atau menjalankan terminal command.

---

## 🗺️ Matriks Komparasi Komponen

| Komponen GUI (`ai-voice/web`) | Backend Ragasukma (`ai-voice`/`ragasukma-core`) | Target Modul TUI (`ragasukma-tui/src/agent_view`) |
| :--- | :--- | :--- |
| `AgentSidebar.tsx` (Sessions + Folders) | `agent::service::list_sessions`, workspace scan | `agent_view/ui.rs` (Left Pane: Sessions & File Tree) |
| `AgentChatPane.tsx` & `MessageList.tsx` | `agent::engine::stream_turn`, SSE/mpsc events | `agent_view/ui.rs` (Center Pane: Chat Stream & Reasoning) |
| `ToolCards.tsx` (Status, Args, Output diff) | `agent::tools::ToolCallInfo`, `diagnostics` | `agent_view/ui.rs` & `types.rs` (`ToolCallInfo`, accordion) |
| `SkillsAndMcpTabs.tsx` | `agent::mcp::list_servers`, `skills` registry | `agent_view/ui.rs` (Right Pane: Inspector & MCP tabs) |
| `AgentWorkspaceHeader.tsx` | `llm::providers`, model settings | `agent_view/ui.rs` (Header bar info, tokens & model select) |
| `AgentModals.tsx` (Diff Approval) | `agent::executor::ApprovalRequest` | `agent_view/diff_modal.rs` / `ui.rs` (Diff review modal) |
| `fileTags.ts` & `useWorkspaceFiles.ts` | `context_builder::scan_workspace_files` | `agent_view/controller.rs` (`@file` autocomplete popup) |

---

## 🚀 Rincian Fase Implementasi

```
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 1: Workspace Context & Multi-Session Persistence (Foundations)  │
└───────────────────────────────────┬────────────────────────────────────┘
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 2: Rich Chat Streaming, Reasoning & Markdown Code Blocks         │
└───────────────────────────────────┬────────────────────────────────────┘
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 3: Interactive Tool Execution Cards & Inline Diff Approvals      │
└───────────────────────────────────┬────────────────────────────────────┘
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 4: Model Parameters, Skills & MCP Server Inspector               │
└───────────────────────────────────┬────────────────────────────────────┘
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 5: Async Agent Engine Integration & Turn Cancellation            │
└───────────────────────────────────┬────────────────────────────────────┘
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 6: Keyboard Ergonomics, Compact Adaptations & Polish             │
└────────────────────────────────────────────────────────────────────────┘
```

---

### 📦 Phase 1: Workspace Context & Multi-Session Persistence (Foundations)
**Fokus**: Memperkuat struktur data state sesi, navigasi folder proyek, dan integrasi SQLite storage.

- [x] **1.1 Multi-Root Workspace Tree**:
  - Dukungan menampilkan multiple roots / subfolder (misal: `ragasukma-core`, `ragasukma-tui`, `ai-voice`).
  - Fitur expand / collapse direktori (`Space` atau `Enter`).
  - Indikator icon file berbasis ekstensi (`🦀 .rs`, `📦 .toml`, `🏷️ .json`, `📝 .md`).
  - Background recursive file scanner dengan batasan kedalaman dan ignore filter (`.git`, `target`, `node_modules`).
- [x] **1.2 Session Management (SQLite Persisted)**:
  - Sinkronisasi `ChatSession` ke SQLite database lokal via `core_bridge`.
  - CRUD sesi:
    - `n`: Sesi baru (Generate session ID & judul default timestamp).
    - `r`: Rename judul sesi yang aktif.
    - `d`: Konfirmasi hapus sesi beserta riwayat pesannya.
    - `j`/`k` & `Enter`: Switch antar sesi aktif dengan instan load pesan.
- [x] **1.3 File Tagging Quick-Picker (`@file`)**:
  - Deteksi trigger `@` saat user mengetik di prompt input.
  - Tampilkan floating popover list file workspace yang match substring pencarian.
  - Tekan `Tab` atau `Enter` untuk autocomplete `@path/to/file.rs` ke dalam buffer prompt.

---

### 💬 Phase 2: Rich Chat Streaming, Reasoning & Markdown Code Blocks
**Fokus**: Pengalaman visual percakapan modern, streaming token responsif, dan pemisahan reasoning.

- [x] **2.1 Token Streaming Pipeline**:
  - Buffer streaming token halus tanpa kedipan layar (*flicker-free rendering* via ratatui state).
  - Auto-scroll ke pesan terbaru saat token masuk, dengan opsi manual scroll-up freeze (*user reading mode*) dan hotkey `g`/`G` navigasi.
  - Indikator animated spinner (`⠋ ⠙ ⠹...`) saat Assistant sedang streaming respons.
- [x] **2.2 Visualisasi Deep Reasoning / Thought Block**:
  - Render blok khusus `💭 Deep Thought / Reasoning` dengan style redup (muted gold / gray border).
  - Accordion toggle untuk collapse / expand isi reasoning dengan tombol pintas (`t`).
- [x] **2.3 Markdown & Syntax Highlighting**:
  - Fenced code block detection (```rust, ```json, ```bash, ```toml, ```sql).
  - Line numbering dan syntax highlight sederhana untuk keyword, type, string, comment, dan punctuation.
  - Border dan prefix pembeda antara pesan User (Cyan), Assistant (Purple/Accent), dan System/Tool (Gold/Gray).

---

### 🛠️ Phase 3: Interactive Tool Execution Cards & Inline Diff Approvals
**Fokus**: Visualisasi eksekusi tool agent (`workspace_read_file`, `workspace_write_file`, `workspace_run_command`, dll.) dan keamanan modifikasi kode.

- [x] **3.1 Interactive Tool Call Cards**:
  - Card representasi eksekusi tool seperti `ToolCards.tsx`:
    - Header: Tool name, status badge (`⏳ RUNNING`, `✓ SUCCESS`, `✗ FAILED`), durasi eksekusi.
    - Args Section: Preview parameter input dalam bentuk JSON ringkas / formatted.
    - Output Section: Cuplikan output (misal: baris file terbaca, stdout perintah terminal).
  - Tombol expand/collapse detail tool call (`o` / `Enter` / `1-9` saat cursor di tool card).
- [x] **3.2 Tool Output Modal Inspector**:
  - Mini terminal output stream viewer stdout/stderr dengan exit code badge.
  - Scroll & filtering per status (`ToolFilter::All`, `Running`, `Success`, `Failed`).
- [x] **3.3 Interactive Diff Approval Review**:
  - Saat tool ingin melakukan modifikasi file (`write_file` / `edit_file`) atau terminal command yang membutuhkan izin:
    - Muncul modal overlay Diff Viewer (`ApprovalRequest`).
    - Menampilkan *unified diff* warna hijau (+) dan merah (-).
    - Aksi interaktif: `[y] Approve`, `[n] Reject`, `[Esc] Cancel`.

---

### 🎛️ Phase 4: Multi-File Context Manager & Token Gauge Meter
**Fokus**: Multi-file context workspace pinning, token gauge progress bar dinamis, dan context telemetry.

- [x] **4.1 Workspace Context Pinner & File Explorer**:
  - Integrasi pin/unpin file langsung dari Workspace File tree menggunakan hotkey `p`.
  - Indikator pin badge `[📌]` di file explorer dan status badge `[MOD]`, `[RO]` di context list.
  - Context files list inspector di panel kanan lengkap dengan estimasi token per file (`~xxx t`).
- [x] **4.2 Dynamic Context Token Gauge Meter & Progress Bar**:
  - Ratatui `Gauge` progress bar visual yang color-coded (Hijau <60%, Kuning 60%-85%, Merah >85%).
  - Telemetry breakdown detail: `Prompt/Sys`, `Chat History`, `Pinned Files`, `Remaining Quota`.
- [x] **4.3 Context Actions & Hotkeys**:
  - Hotkey `p` untuk toggle pin file terpilih di Files panel.
  - Hotkey `x` di Inspector untuk remove file dari context.
  - Navigasi `j`/`k` untuk memilih context file di Inspector.

---

### ⚡ Phase 5: Async Agent Engine Integration & Turn Cancellation
**Fokus**: Menghubungkan antarmuka TUI langsung dengan `ragasukma-core::agent::engine` & `orchestrator`.

- [ ] **5.1 Real Event Stream Wiring**:
  - Menghubungkan event transmitter `mpsc::Sender<AppEvent>` ke agent runner background task.
  - Handling event mutasi runtime:
    - `AgentEvent::Thinking(chunk)`
    - `AgentEvent::TextDelta(chunk)`
    - `AgentEvent::ToolStart(call_id, tool_name, args)`
    - `AgentEvent::ToolResult(call_id, status, output)`
    - `AgentEvent::TurnFinished(stats)`
- [ ] **5.2 Interupsi & Pembatalan Turn**:
  - Tombol `Esc` atau `Ctrl+C` saat agent sedang jalan untuk mengirim sinyal cancel (`tokio::select!` / `CancellationToken`).
  - Status agent berubah menjadi `Interrupted` secara graceful tanpa merusak riwayat chat.
- [ ] **5.3 Error Boundary & Auto-Retry**:
  - Deteksi rate limit, context length exceeded, atau network timeout dengan pesan error yang jelas dan opsi `[r] Retry Turn`.

---

### 🎨 Phase 6: Keyboard Ergonomics, Compact Adaptations & Polish
**Fokus**: Penyempurnaan ergonomi navigasi keyboard, responsivitas ukuran terminal, dan standardisasi kode.

- [x] **6.1 Ergonomi Keybindings**:
  - VIM-friendly navigation: `h`/`j`/`k`/`l` untuk scroll dan pindah list.
  - Panel Switching: `Tab` / `BackTab` untuk siklus fokus (`Sessions -> Files -> Chat -> Input -> Inspector`).
  - Hotkey langsung:
    - `Ctrl+F`: Fokus ke File Tree.
    - `Ctrl+I` / `i`: Fokus ke Prompt Input.
    - `Ctrl+P`: Buka Model / Parameter switcher.
    - `Ctrl+Enter` / `Ctrl+S`: Kirim prompt ke agent.
- [x] **6.2 Responsive Layout Adaptation**:
  - Bila lebar terminal < 120 kolom:
    - Otomatis sembunyikan Pane Kanan (Inspector).
    - Panel Kiri (Files/Sessions) dapat di-toggle buka/tutup via `Ctrl+B`.
  - Bila lebar terminal < 80 kolom:
    - Mode single pane (fokus penuh pada Chat + Input).
- [x] **6.3 QA & Strict Clippy Check**:
  - Jalankan `cargo clippy --bin ragasukma-tui` dengan strict flags.
  - Verifikasi unit test formatting, markdown parser, dan state transitions.

---

## 📌 Checklist Status Implementasi

| Fase | Deskripsi | Status |
| :---: | :--- | :---: |
| **Phase 1** | Workspace Context & Multi-Session Persistence | ✅ Completed |
| **Phase 2** | Rich Chat Streaming, Reasoning & Markdown Blocks | ✅ Completed |
| **Phase 3** | Interactive Tool Execution Cards & Inline Diff | ✅ Completed |
| **Phase 4** | Model Parameters, Skills & MCP Server Inspector | ✅ Completed |
| **Phase 5** | Async Agent Engine Integration & Turn Runner | ✅ Completed |
| **Phase 6** | Keyboard Ergonomics, Responsive & Polish | ✅ Completed |
