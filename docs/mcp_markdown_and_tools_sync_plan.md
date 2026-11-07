# Rencana Implementasi: MCP Server Markdown Config & AI-Driven Tools Sync

> **Status:** Draft / Siap Dikerjakan  
> **Target:** `ragasukma-core` & `ragasukma-tui`  
> **Tujuan Utama:** Mengubah konfigurasi MCP dari JSON kaku menjadi format Markdown (`.md`), menambahkan tabel `mcp_server_tools`, dan mengimplementasikan Runner LLM untuk membaca/mempelajari tools serta melakukan sinkronisasi otomatis.

---

## 1. Latar Belakang & Motivasi

Protokol MCP (*Model Context Protocol*) di ekosistem open-source memiliki ragam implementasi dan belum sepenuhnya terstandarisasi di lapangan:
- Banyak server MCP kustom tidak memiliki JSON-RPC schema yang lengkap atau valid.
- LLM jauh lebih adaptif memahami instruksi dan skema parameter dalam format **Markdown** daripada *deeply-nested JSON schema*.
- Pengguna membutuhkan fleksibilitas dalam mendokumentasikan server, endpoint, batasan otorisasi, dan perintah eksekusi dalam format dokumen manusiawi.
- Dengan melibatkan LLM model runner saat fase *discovery* (pemilihan model mirip dengan model selector di `agent_view`), LLM bertindak sebagai **Semantic Compiler/Adapter** yang membaca konfigurasi Markdown server, mengidentifikasi daftar tool, dan menyimpannya ke tabel `mcp_server_tools`.

---

## 2. Desain Data Model & Migrasi

### 2.1 Migrasi Baru: `0010_mcp_server_tools.sql` (`ragasukma-core`)

Tabel baru dibuat sederhana namun kuat dengan relasi idempoten:

```sql
-- 0010_mcp_server_tools.sql
-- Simpan hasil discovery tools MCP berbasis Markdown

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS mcp_server_tools (
    server_id TEXT NOT NULL REFERENCES mcp_servers(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    config TEXT NOT NULL, -- Format Markdown: deskripsi, schema/argumen, panduan pemanggilan
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (server_id, name)
);

CREATE INDEX IF NOT EXISTS idx_mcp_server_tools_server_id ON mcp_server_tools(server_id);
```

### 2.2 Format Markdown pada `mcp_servers.config`

Kolom `config` di tabel `mcp_servers` yang sebelumnya menyimpan JSON mentah akan diisi format Markdown.

Contoh format Markdown untuk Server:
````markdown
# GitHub MCP Server

Server untuk berinteraksi dengan GitHub API dan repository management.

## Runtime Execution
```exec
transport: stdio
command: npx
args:
  - "-y"
  - "@modelcontextprotocol/server-github"
env:
  GITHUB_PERSONAL_ACCESS_TOKEN: "${GITHUB_TOKEN}"
```

## Available Capabilities & Notes
- Menangani pull request, issue tracker, dan manipulasi git tree.
````

### 2.3 Format Markdown pada `mcp_server_tools.config`

Setiap tool hasil sinkronisasi disimpan sebagai Markdown:
````markdown
# search_repositories

Cari repository GitHub berdasarkan query teks atau topik.

## Parameters
- `query` (string, required): Query pencarian repository.
- `page` (number, optional): Nomor halaman paginasi (default 1).
- `per_page` (number, optional): Jumlah item per halaman (maks 100).

## Execution Contract
```json
{
  "method": "tools/call",
  "name": "search_repositories"
}
```
````

---

## 3. Alur Kerja Sistem (Workflow)

```
[User TUI: mcp_server_view]
        │
        ├── 1. Input/Edit Server Config (.md format)
        │
        ├── 2. Tekan tombol [Sync Tools]
        │         │
        │         ▼
        │   [Pilih Model Runner (mirip agent_view)]
        │         │
        │         ▼
        │   [Runner Task dijalankan di background]
        │   - LLM membaca format Markdown mcp_server
        │   - Handshake / inspect daftar tools
        │   - LLM memformat setiap tool menjadi spec Markdown
        │         │
        │         ▼
        └── 3. Upsert data ke tabel `mcp_server_tools`
```

---

## 4. Urutan Pengerjaan Fase demi Fase

### Fase 1: Database Migration & Core Entities (`ragasukma-core`)
- [ ] **1.1** Buat file migrasi `ragasukma-core/migrations/0010_mcp_server_tools.sql`.
- [ ] **1.2** Tambahkan struct entitas `McpServerTool` di `ragasukma-core/src/agent/models.rs`.
- [ ] **1.3** Implementasikan query repository:
  - `upsert_mcp_server_tool(pool, server_id, name, config)`
  - `list_mcp_server_tools(pool, server_id)`
  - `delete_mcp_server_tool(pool, server_id, name)`
  - `delete_mcp_server_tools_by_server(pool, server_id)`
- [ ] **1.4** Perbarui parser helper di `McpServer` agar dapat mengekstrak blok eksekusi baik dari JSON legacy maupun blok ` ```exec ` / frontmatter YAML pada Markdown.
- [ ] **1.5** Jalankan `cargo test -p ragasukma-core` untuk memastikan migrasi dan entitas lolos verifikasi.

### Fase 2: TUI Editor Update (`ragasukma-tui/src/mcp_server_view`)
- [ ] **2.1** Ubah UI input field dari `JsonConfig` menjadi `MarkdownConfig` pada `types.rs` dan `state.rs`.
- [ ] **2.2** Hapus validasi kaku `serde_json::from_str` saat menyimpan konfigurasi server, ganti dengan validasi non-empty teks Markdown.
- [ ] **2.3** Sesuaikan tampilan preview list server (ganti preview JSON keys menjadi snippet/summary teks Markdown).
- [ ] **2.4** Sediakan tab / modal untuk melihat daftar tools yang tersinkronisasi di server yang dipilih (`mcp_server_tools`).

### Fase 3: Tool Discovery Engine & Model Runner (`ragasukma-core`)
- [ ] **3.1** Buat modul `ragasukma-core/src/agent/mcp/discovery.rs`:
  - Mengambil daftar tools (baik via native handshake RPC maupun analisis dokumentasi via LLM).
  - Prompt template untuk LLM runner: *"Pelajari dokumentasi MCP server berikut, lalu hasilkan daftar tool dalam format Markdown terstruktur untuk masing-masing tool."*
- [ ] **3.2** Integrasikan runner dengan client inference yang sudah ada (menggunakan model provider yang dipilih).
- [ ] **3.3** Fungsi sync yang menerima respon LLM, mem-parsing per-tool, dan melakukan upsert ke database `mcp_server_tools`.

### Fase 4: TUI Sync Modal & Model Selector (`ragasukma-tui`)
- [ ] **4.1** Tambahkan aksi/tombol `[S] Sync Tools` di `mcp_server_view`.
- [ ] **4.2** Tampilkan popup/modal pemilihan model runner (mengambil daftar model aktif yang tersedia seperti pada `agent_view`).
- [ ] **4.3** Tampilkan status progress sinkronisasi (Spinner / Toast notification).
- [ ] **4.4** Muat ulang daftar tools di view setelah proses sinkronisasi selesai.

### Fase 5: Chat Loop & Runtime Integration (`ragasukma-core`)
- [ ] **5.1** Saat sesi agent chat dimulai, load tools dari `mcp_server_tools` aktif ke dalam sistem prompt context (sebagai instruksi Markdown).
- [ ] **5.2** Uji eksekusi tool dari LLM: LLM memanggil tool -> core memvalidasi dengan contract -> mengeksekusi -> mengembalikan hasil.

### Fase 6: Pengujian, Hygiene & Verifikasi Menyeluruh
- [ ] **6.1** Unit tests di `ragasukma-core` untuk parser config Markdown dan sync tool.
- [ ] **6.2** UI / State tests di `ragasukma-tui` untuk form input Markdown dan integrasi event sync.
- [ ] **6.3** Verifikasi build dan linting:
  ```bash
  cargo clippy --workspace --all-targets
  cargo test --workspace
  ```
