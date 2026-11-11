# Rencana Arsitektur & Implementasi: Dynamic Session MCP & In-Memory Tool Registry

> **Status:** Revised Architecture (Dynamic On-Demand)  
> **Target:** `ragasukma-core` & `ragasukma-tui`  
> **Tujuan Utama:** Menghapus ketergantungan statis SQLite untuk daftar tools (`mcp_server_tools`) dan status aktif (`is_enabled`), beralih ke katalog global ramping (`mcp_servers`), state dinamis per sesi (`session_mcp_servers`), serta pengelolaan tools murni di memori (RAM).

---

## 1. Latar Belakang & Perubahan Arsitektur

### Masalah Desain Sebelumnya
1. **Redundansi Status (`is_enabled` di `mcp_servers`):**
   - Kolom `is_enabled` di level server global menimbulkan konflik status saat multi-sesi. Status aktif/nonaktif seharusnya dikontrol secara dinamis per sesi obrolan (*on-demand dynamic indexing*).
2. **Stale Schema & Redundant Table (`mcp_server_tools`):**
   - Menyimpan tools ke SQLite memicu skema basi jika server MCP upstream berubah, serta menimbulkan masalah parsing command/markdown yang rumit.
   - Standar industri MCP (Claude Desktop, Cursor, Zed) tidak menyimpan tools ke database, melainkan menampungnya di **RAM** selama sesi berlangsung.
3. **Kekakuan Junction Project (`project_mcp_servers`):**
   - Pengaturan per-project menambah kompleksitas relasi dan tidak fleksibel. MCP lebih tepat dikelola pada level **Sesi Chat** dengan kredensial (`env`) terisolasi per sesi.

---

## 2. Desain Data Model & Migrasi Database

### 2.1 Tabel Dihapus (Deprecated / Dropped)
- ❌ **`mcp_server_tools`**: Dihapus total. Tools dikelola di RAM.
- ❌ **`project_mcp_servers`**: Dihapus total. Digantikan oleh level sesi.

### 2.2 Tabel `mcp_servers` (Katalog Global / Blueprint Murni)
Kolom `is_enabled` dihapus. Tabel ini hanya menyimpan metadata dasar koneksi server:

```sql
CREATE TABLE IF NOT EXISTS mcp_servers (
    id TEXT PRIMARY KEY,                  -- e.g. 'cartesia', 'playwright', 'figcraft'
    type TEXT NOT NULL,                   -- 'stdio' | 'http'
    config TEXT NOT NULL,                 -- JSON string konfigurasi koneksi (url, command, args)
    handshake INTEGER NOT NULL DEFAULT 1, -- 1: wajib handshake initialize & session-id, 0: direct
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
```

### 2.3 Tabel `session_mcp_servers` (State Dinamis & Kredensial Per Sesi)
Menyimpan server apa saja yang aktif pada sesi chat tertentu beserta variabel rahasia (`env`) khusus sesi tersebut:

```sql
CREATE TABLE IF NOT EXISTS session_mcp_servers (
    session_id TEXT NOT NULL,             -- ID sesi chat aktif
    server_id TEXT NOT NULL REFERENCES mcp_servers(id) ON DELETE CASCADE,
    env TEXT NOT NULL DEFAULT '{}',       -- JSON string key-value rahasia/token sesi (e.g. {"CARTESIA_API_KEY": "sk_car_xxx"})
    is_active INTEGER NOT NULL DEFAULT 1, -- 1: aktif di sesi ini, 0: dinonaktifkan sementara
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (session_id, server_id)
);

CREATE INDEX IF NOT EXISTS idx_session_mcp_servers_session ON session_mcp_servers(session_id);
```

---

## 3. In-Memory Registry (RAM-Based Tools Management)

Daftar tools dari server MCP tidak disimpan di SQLite, melainkan ditampung di memori (RAM) pada layer Agent Engine selama sesi chat aktif:

```rust
pub struct SessionMcpRegistry {
    /// Mapping: session_id -> server_id -> Daftar schema tool terkini
    pub active_tools: Arc<RwLock<HashMap<String, HashMap<String, Vec<McpToolDefinition>>>>>,
    /// Mapping: session_id -> server_id -> Mcp-Session-Id (untuk Streamable HTTP/SSE)
    pub http_sessions: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
}
```

### Keuntungan:
1. **Always Fresh:** Saat server dihubungkan ke sesi, agent melakukan `initialize` $\rightarrow$ `tools/list`. Schema selalu paling mutakhir dari server upstream.
2. **Native JSON Schema:** Langsung diteruskan ke LLM function calling tanpa kompromi parsing string atau escaping shell.
3. **Zero Waste:** Menutup sesi otomatis membersihkan RAM tanpa meninggalkan sampah di database.

---

## 4. Alur Kerja Dinamis via Chat Session (Dynamic On-Demand via LLM)

```
[User Chat Session]
        │
        ├── 1. User minta kapabilitas eksternal (misal: "Bacakan teks ini pakai Cartesia TTS, key: sk_car_xxx")
        │
        ├── 2. Agent mengecek ketersediaan di `mcp_servers` & `session_mcp_servers`
        │         │
        │         ├── Jika belum aktif: Agent mengaktifkan & meng-inject env ke `session_mcp_servers`
        │         └── Jika server belum ada: Agent dapat mendaftarkan blueprint baru ke `mcp_servers`
        │
        ├── 3. Agent melakukan handshake koneksi (stdio spawn / HTTP initialize)
        │         │
        │         ▼
        │   [Load `tools/list` langsung ke RAM Session Registry]
        │
        ├── 4. Tool dieksekusi secara Native (JSON-RPC)
        │
        └── 5. User minta menonaktifkan tool -> Agent set `is_active = 0` di `session_mcp_servers` & buang dari RAM
```

---

## 5. Rencana Fase Pengerjaan (Action Items)

### Fase 1: Database Migration & Schema Cleanup (`ragasukma-core`)
- [x] **1.1** Buat migrasi SQL baru:
  - Hapus kolom `is_enabled` dari `mcp_servers` (atau recreat/alter table mcp_servers).
  - Buat tabel `session_mcp_servers` (session_id, server_id, env, is_active).
  - Drop tabel `mcp_server_tools` dan `project_mcp_servers` jika ada.
- [x] **1.2** Perbarui model struct di `ragasukma-core`:
  - Update `McpServer` struct (hapus field `is_enabled`, pastikan memiliki `id`, `type`, `config`, `handshake`).
  - Tambahkan struct `SessionMcpServer` (`session_id`, `server_id`, `env`, `is_active`).
  - Hapus entitas `McpServerTool` dan query repository terkait `mcp_server_tools`.

### Fase 2: In-Memory Registry Engine (`ragasukma-core`)
- [x] **2.1** Buat `SessionMcpRegistry` di `ragasukma-core/src/agent/mcp/`:
  - Menyimpan daftar tool di RAM (`HashMap`).
  - Mengelola lifecycle session ID HTTP/SSE dan child process stdio per sesi.
- [x] **2.2** Implementasikan handshake RPC universal:
  - Stdio process handler (JSON-RPC 2.0).
  - Streamable HTTP/SSE handler (Header session ID negotiation).

### Fase 3: Agent Management Tools (On-Demand Dynamic Index)
- [x] **3.1** Buat tool internal untuk Agent LLM:
  - Dynamic on-demand activation via `activate_extension(extension_name)`.
  - Global blueprint registration via `mcp_register_server(server_id, type, command_or_url, args)`.
  - Session credential injection & lifecycle toggle via `mcp_configure_session(server_id, env, is_active)`.
  - System catalog discovery via `mcp_list_available()`.
- [x] **3.2** Integrasikan dengan prompt context agent agar agent memahami kapan harus mengaktifkan/menonaktifkan MCP tools (`Available MCP Extensions (Dormant)`).

### Fase 4: Penyesuaian TUI (`ragasukma-tui`)
- [x] **4.1** Sesuaikan `mcp_server_view`:
  - Menghapus tab dedicated terpisah `[6] MCP Servers` dan modul `mcp_server_view` yang sudah usang.
  - Memusatkan integrasi MCP langsung di sidebar Agent Workspace.
- [x] **4.2** Pada chat view, sediakan indikator server MCP yang sedang aktif di sesi tersebut (sidebar ter-sinkronisasi langsung dengan session aktif).

### Fase 5: Pengujian & Verifikasi
- [x] **5.1** Unit test untuk migrasi dan CRUD `session_mcp_servers`.
- [x] **5.2** Integration test handshake stdio dan HTTP streamable.
- [x] **5.3** Verifikasi `cargo test --workspace` (lulus di `ragasukma-core` dan `ragasukma-tui`).
