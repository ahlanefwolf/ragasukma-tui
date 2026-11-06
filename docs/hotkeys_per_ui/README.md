# 🎹 Hotkeys Reactive per UI — RagaSukma TUI

Dokumentasi lengkap **semua hotkey reaktif** per tampilan (UI), dipisah per **tab 1–6**,
per **focus card**, per **modal**, dan per **popup**.

> Semua hotkey di bawah **diverifikasi langsung** dari source code controller
> (`*_view/controller.rs`) & footer hints (`*_view/ui*.rs`), BUKAN tebakan.

---

## 📑 Tab Mapping

| Tombol | Tab | View | Dokumentasi |
|--------|-----|------|-------------|
| `1` | Agent | `agent_view` | [tab1_agent.md](tab1_agent.md) |
| `2` | Antigravity | `provider_view` | [tab2_antigravity.md](tab2_antigravity.md) |
| `3` | Codex | `provider_view` | [tab3_codex.md](tab3_codex.md) |
| `4` | Claude | `provider_view` | [tab4_claude.md](tab4_claude.md) |
| `5` | Custom LLM | `custom_llm_view` | [tab5_custom_llm.md](tab5_custom_llm.md) |
| `6` | MCP Servers | `mcp_server_view` | [tab6_mcp_servers.md](tab6_mcp_servers.md) |

> ⚠️ **Catatan status bar:** label di status bar menampilkan `[1-6]` tapi
> **Antigravity/Codex/Claude adalah satu view yang sama (`provider_view`)** dengan
> isi yang berbeda. `1-5` = switch tab, `6` = MCP. Lihat detail di tiap file tab.

---

## 🌐 Global Hotkeys (semua tab)

Dispatcher: `src/app/controller.rs` → `handle_global_key`

| Hotkey | Aksi | Keterangan |
|--------|------|------------|
| `1`–`6` | Pindah tab | `1`=Agent … `6`=MCP Servers |
| `Ctrl+C` | Quit aplikasi | **Tab 1 (Agent) & tab 2-4/custom/mcp berbeda:** di Agent `Ctrl+C` quit; di tab lain `q` juga quit |
| `q` | Quit aplikasi | Hanya aktif di tab **2–6** (tab 1 Agent: `q` = ketik biasa di input) |

> Perilaku `q`/`Ctrl+C` bergantung view; detail per tab ada di masing-masing file.

---

## 🗂️ Peta Struktur

```
Tiap tab punya:
  ├─ Focus Cards        (navigasi Tab / Shift+Tab antar card)
  ├─ Hotkeys per Card   (reaktif sesuai card yang fokus)
  ├─ Modals             (pilihan model, add folder, project, dll)
  ├─ Popups             (tag picker / tag query)
  ├─ Forms / Dialogs    (input multiline, config form)
  └─ Confirm Dialogs    (approve/reject, delete confirm)
```

---

## 🔑 Konvensi Tampilan

- **Focus Card** = panel yang sedang fokus (ditandai badge di footer, mis. `[📁 FOLDERS]`).
- **INSERT vs NORMAL** = mode input chat agent (seperti Vim). `Esc` keluar INSERT.
- `j/k` = alias `↓/↑` (Vim style) — berlaku di semua navigasi list/card.
- Hotkey yang **hanya muncul saat kondisi tertentu** (mis. `Alt+S` saat agent running) ditandai dengan **⚡ conditional**.

---

## Sumber Kebenaran

| File | Fungsi |
|------|--------|
| `src/agent_view/controller.rs` | Hotkey tab 1 |
| `src/provider_view/controller.rs` | Hotkey tab 2/3/4 |
| `src/custom_llm_view/controller.rs` | Hotkey tab 5 |
| `src/mcp_server_view/controller.rs` | Hotkey tab 6 |
| `src/app/controller.rs` | Global fallback (`1-6`, `q`, `Ctrl+C`) |
| `*/ui*.rs` → `get_footer_key_hints` | Sumber hint footer yang ditampilkan |
