# 📚 Ragasukma TUI - Hotkeys, Focus Areas, Modals & Footer Helpers (Tab 1 - Tab 5)

Dokumentasi komprehensif seluruh interaksi antarmuka pengguna (UI), fokus kartu (cards), modal pop-up, hotkey navigasi, dan footer helper context untuk **Tab 1 sampai Tab 5** di `ragasukma-tui`.

---

## 📑 Navigasi Global (Global Shortcuts)

Berlaku di seluruh tab aplikasi:

| Hotkey | Aksi | Keterangan |
| :--- | :--- | :--- |
| `1` | **Tab 1: Agent Workspace** | Tampilan chat, agent inspector, history, dan konfigurasi tool |
| `2` | **Tab 2: Antigravity** | Manajemen akun OAuth & kuota provider Antigravity |
| `3` | **Tab 3: Codex** | Manajemen akun OAuth & kuota provider Codex |
| `4` | **Tab 4: Claude** | Manajemen akun OAuth & kuota provider Claude |
| `5` | **Tab 5: Custom LLM** | Integrasi custom endpoint OpenAI/Ollama |
| `Ctrl+c` / `q` | **Quit Application** | Keluar dari aplikasi (tidak aktif saat berada dalam mode input text/form) |
| `?` | **Toggle Help Modal** | Membuka bantuan & ringkasan shortcut global |

---

## 1️⃣ Tab 1: Agent Workspace (`ragasukma-tui/src/agent_view`)

### 🔲 Focus Areas & Cards
Navigasi antar kartu/panel menggunakan tombol `Tab` (maju) dan `Shift+Tab` / `BackTab` (mundur).

Urutan rotasi fokus (`AgentFocusArea`):
1. **Chat Input** (`AgentFocusArea::ChatInput`)
2. **Messages / Transcript** (`AgentFocusArea::Messages`)
3. **Session List / History** (`AgentFocusArea::SessionList`)
4. **Preset List** (`AgentFocusArea::PresetList`)
5. **Tool Catalog / Cards** (`AgentFocusArea::ToolCatalog`)
6. **Active Subagents Inspector** (`AgentFocusArea::ActiveSubagents`)
7. **Model Telemetry** (`AgentFocusArea::ModelTelemetry`)

---

### ⌨️ Hotkey & Footer Helper per Area di Tab 1

#### 1. Chat Input (`AgentFocusArea::ChatInput`)
* **Footer Helper:**
  `[Enter] Send` | `[Shift+Enter / Alt+Enter] Newline` | `[Tab] Focus` | `[@] Tag Query` | `[/] Slash Command` | `[Ctrl+s] Subagent Modal` | `[Ctrl+t] Tools Modal` | `[Ctrl+o] Presets Modal`
* **Daftar Hotkey:**
  - `Enter`: Mengirim prompt / pesan ke agent.
  - `Shift+Enter` atau `Alt+Enter`: Menambahkan baris baru (newline) pada area input.
  - `@`: Membuka modal **Tag Query** untuk mention subagent, tool, atau model target.
  - `/`: Membuka autocomplete popup **Slash Commands**.
  - `Ctrl+s`: Membuka modal **Subagent Management**.
  - `Ctrl+t`: Membuka modal **Tool Management / Config**.
  - `Ctrl+o`: Membuka modal **Preset Selector**.
  - `Esc`: Menutup pop-up saran autocomplete atau tag query.
  - `Tab`: Pindah fokus ke card/area berikutnya.

#### 2. Messages / Transcript (`AgentFocusArea::Messages`)
* **Footer Helper:**
  `[j/k or ↑/↓] Scroll` | `[g/G] Top/Bottom` | `[Ctrl+d/Ctrl+u] Half Page` | `[c] Copy Last Codeblock` | `[y] Copy Message` | `[Tab] Focus`
* **Daftar Hotkey:**
  - `j` / `Down`: Gulir pesan ke bawah.
  - `k` / `Up`: Gulir pesan ke atas.
  - `g` / `Home`: Lompat langsung ke pesan paling awal (atas).
  - `G` / `End`: Lompat langsung ke pesan terbaru (bawah) dan mengaktifkan auto-scroll.
  - `Ctrl+d` / `PageDown`: Gulir setengah layar ke bawah.
  - `Ctrl+u` / `PageUp`: Gulir setengah layar ke atas.
  - `c`: Menyalin code block terakhir yang digenerate oleh LLM ke clipboard sistem.
  - `y`: Menyalin isi pesan yang sedang dipilih ke clipboard.
  - `Tab`: Pindah fokus ke area Session List.

#### 3. Session List (`AgentFocusArea::SessionList`)
* **Footer Helper:**
  `[j/k] Select` | `[Enter] Load Session` | `[n] New Session` | `[d] Delete Session` | `[r] Rename Session` | `[Tab] Focus`
* **Daftar Hotkey:**
  - `j` / `Down`: Pilih sesi berikutnya di sidebar.
  - `k` / `Up`: Pilih sesi sebelumnya di sidebar.
  - `Enter`: Memuat (load) sesi riwayat percakapan yang dipilih.
  - `n`: Membuat sesi obrolan baru yang bersih (*New Session*).
  - `d`: Konfirmasi dan menghapus sesi yang disorot.
  - `r`: Mengubah nama (rename) judul sesi.
  - `Tab`: Pindah fokus ke Preset List.

#### 4. Preset List (`AgentFocusArea::PresetList`)
* **Footer Helper:**
  `[j/k] Select Preset` | `[Enter] Apply Preset` | `[Space] Details` | `[Tab] Focus`
* **Daftar Hotkey:**
  - `j` / `Down`: Memilih preset berikutnya.
  - `k` / `Up`: Memilih preset sebelumnya.
  - `Enter`: Menerapkan instruksi preset ke sesi aktif.
  - `Space`: Membuka preview detail instruksi preset.
  - `Tab`: Pindah fokus ke Tool Catalog.

#### 5. Tool Catalog / Tool Cards (`AgentFocusArea::ToolCatalog`)
* **Footer Helper:**
  `[j/k] Navigate` | `[Space] Toggle Enable/Disable` | `[Enter/c] Config Modal` | `[Tab] Focus`
* **Daftar Hotkey:**
  - `j` / `Down`: Memilih kartu tool berikutnya.
  - `k` / `Up`: Memilih kartu tool sebelumnya.
  - `Space`: Mengaktifkan / menonaktifkan tool (Toggle On/Off).
  - `Enter` / `c`: Membuka modal konfigurasi parameter tool.
  - `Tab`: Pindah fokus ke Active Subagents Inspector.

#### 6. Active Subagents Inspector (`AgentFocusArea::ActiveSubagents`)
* **Footer Helper:**
  `[j/k] Select` | `[Enter] Inspect Telemetry` | `[k] Terminate Subagent` | `[Tab] Focus`
* **Daftar Hotkey:**
  - `j` / `Down`: Memilih subagent worker.
  - `Enter`: Membuka telemetry dan step log subagent terpilih.
  - `k`: Menghentikan paksa (terminate/kill) proses subagent aktif.
  - `Tab`: Pindah fokus ke Model Telemetry.

#### 7. Model Telemetry (`AgentFocusArea::ModelTelemetry`)
* **Footer Helper:**
  `[j/k] Scroll Logs` | `[r] Reset Stats` | `[Tab] Focus`
* **Daftar Hotkey:**
  - `j` / `Down`: Scroll log metrik penggunaan token dan latensi.
  - `r`: Reset statistik token & latensi.
  - `Tab`: Pindah fokus kembali ke Chat Input.

---

### 🪟 Modals di Tab 1

1. **Tag Query Modal (`@mention`)**
   - **Trigger:** Mengetik `@` di Chat Input.
   - **Navigasi:** `j` / `k` atau tombol panah untuk memilih Subagent, Tool, atau Provider target.
   - **Aksi:** `Enter` atau `Tab` untuk menerapkan tag ke dalam prompt input; `Esc` untuk menutup.
2. **Tool Config Modal**
   - **Trigger:** Tekan `Enter` atau `c` saat fokus di Tool Catalog, atau shortcut `Ctrl+t`.
   - **Navigasi:** `Tab` / `Shift+Tab` untuk berpindah field setting form tool.
   - **Aksi:** `Enter` untuk menyimpan; `Esc` untuk batal/menutup.
3. **Subagent Delegation Modal**
   - **Trigger:** Shortcut `Ctrl+s`.
   - **Navigasi:** `j` / `k` memilih template agent worker.
   - **Aksi:** `Enter` untuk spawn subagent baru; `Esc` untuk menutup.
4. **Preset Selector Modal**
   - **Trigger:** Shortcut `Ctrl+o`.
   - **Navigasi:** `j` / `k` untuk menelusuri preset.
   - **Aksi:** `Enter` untuk menerapkan preset; `Esc` untuk menutup.

---

## 2️⃣-4️⃣ Tab 2, 3, 4: Providers (`ragasukma-tui/src/provider_view`)
*(Tab 2: Antigravity | Tab 3: Codex | Tab 4: Claude)*

### 🔲 Focus Area & Cards
Setiap tab provider terbagi menjadi 2 area/kartu utama yang dapat ditukar menggunakan tombol `Tab`:
1. **Accounts Card** (`ProviderFocusArea::Accounts`) - Menampilkan daftar akun, status login, dan default badge.
2. **Quotas Card** (`ProviderFocusArea::Quotas`) - Menampilkan daftar limit kuota model, sisa persentase, dan reset counter.

---

### ⌨️ Hotkey & Footer Helper per Area di Provider Tabs

#### 1. Accounts Card (`ProviderFocusArea::Accounts`)
* **Footer Helper:**
  `[Tab] Quotas` | `[j/k] Select` | `[Space] Active` | `[a] Add` | `[d] Delete` | `[r] Refresh` | `[1-5] Tabs` | `[q] Quit`
* **Daftar Hotkey:**
  - `Tab`: Pindah fokus ke **Quotas Card**.
  - `j` / `Down`: Memilih akun di bawahnya.
  - `k` / `Up`: Memilih akun di atasnya.
  - `Space`: Toggle status aktif / tidak aktif untuk akun yang dipilih.
  - `Enter`: Menjadikan akun yang dipilih sebagai akun default (*Default Account*).
  - `a`: Membuka browser untuk memulai alur autentikasi OAuth provider (`StartAntigravityOAuth` / `StartCodexOAuth` / `StartClaudeOAuth`).
  - `d`: Menghapus akun yang dipilih beserta kredensial penyimpanannya.
  - `r` / `R`: Refresh data kuota akun yang sedang dipilih.
  - `Ctrl+j` / `Ctrl+Down`: Scroll Quotas Card ke bawah tanpa perlu memindahkan fokus.
  - `Ctrl+k` / `Ctrl+Up`: Scroll Quotas Card ke atas tanpa perlu memindahkan fokus.

#### 2. Quotas Card (`ProviderFocusArea::Quotas`)
* **Footer Helper:**
  `[Tab] Accounts` | `[j/k] Scroll` | `[r] Refresh` | `[1-5] Tabs` | `[q] Quit`
* **Daftar Hotkey:**
  - `Tab` / `BackTab`: Pindah fokus kembali ke **Accounts Card**.
  - `j` / `Down` / `PageDown`: Scroll daftar kuota model ke bawah (step 5 baris).
  - `k` / `Up` / `PageUp`: Scroll daftar kuota model ke atas.
  - `r` / `R`: Refresh kuota model.

---

## 5️⃣ Tab 5: Custom LLM (`ragasukma-tui/src/custom_llm_view`)

### 🔲 Focus Area & Cards
Tab ini terbagi menjadi 2 kartu horizontal yang ditukar dengan tombol `Tab`:
1. **Providers Card** (`CustomLlmFocus::Providers`) - Tabel konfigurasi prefix, nama provider, base URL, dan status ON/OFF.
2. **Models Catalog Card** (`CustomLlmFocus::Models`) - Katalog model yang tersedia untuk prefix provider yang dipilih.

---

### ⌨️ Hotkey & Footer Helper di Tab 5

#### Normal Mode (Tanpa Modal Terbuka)
* **Footer Helper:**
  `[Tab] Switch Focus` | `[j/k] Select` | `[a] Add Provider` | `[e] Edit` | `[Space] Toggle` | `[d] Delete` | `[r] Fetch Models` | `[1-5] Tabs` | `[?] Help` | `[q] Quit`
* **Daftar Hotkey:**
  - `Tab`: Berpindah fokus antara **Providers Card** dan **Models Catalog Card**.
  - `j` / `Down`: Memilih entri berikutnya pada tabel aktif.
  - `k` / `Up`: Memilih entri sebelumnya pada tabel aktif.
  - `a`: Membuka modal **Add Custom Provider** form baru.
  - `e`: Membuka modal **Edit Custom Provider** untuk provider yang sedang dipilih.
  - `Space`: Mengaktifkan / menonaktifkan status auto-routing (`ON` / `OFF`).
  - `d`: Membuka modal konfirmasi hapus provider.
  - `r` / `R`: Fetch / refresh katalog model secara realtime dari endpoint `/v1/models`.

---

### 🪟 Modals di Tab 5

#### 1. Add / Edit Provider Modal (`render_form_modal`)
* **Footer Helper:**
  `[Tab/Shift+Tab] Next/Prev Field` | `[Enter] Submit` | `[Esc] Cancel`
* **Daftar Hotkey Form Modal:**
  - `Tab` / `Down`: Berpindah ke field form berikutnya dengan siklus:
    `Prefix (/custom)` ➔ `Provider Name` ➔ `Base URL Endpoint` ➔ `API Key` ➔ `Provider Enabled Checkbox` ➔ `[Save & Apply]` ➔ `[Cancel]`.
  - `Shift+Tab` / `Up`: Berpindah ke field form sebelumnya.
  - `Space`: Mengubah status centang `[✔]` / `[ ]` jika berada pada field Enabled Checkbox.
  - `Enter`: Submit form dan menyimpan konfigurasi jika berada di input field atau di tombol `[Save & Apply]`.
  - `Esc`: Membatalkan input dan menutup formulir tanpa menyimpan.

#### 2. Confirm Deletion Modal (`render_delete_modal`)
* **Daftar Hotkey Modal Hapus:**
  - `y` / `Enter`: Mengonfirmasi penghapusan custom provider beserta kredensial tersimpan.
  - `n` / `Esc`: Membatalkan penghapusan dan kembali ke kartu provider.
