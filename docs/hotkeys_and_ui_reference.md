# 📚 Ragasukma TUI - Hotkeys, Focus Areas, Modals & Footer Helpers (Tab 1 - Tab 5)

Dokumentasi resmi dan akurat sesuai implementasi source code (`ragasukma-tui/src/agent_view`, `ragasukma-tui/src/provider_view`, dan `ragasukma-tui/src/custom_llm_view`).

---

## 📑 Navigasi Global (Global Shortcuts)

Berlaku di seluruh tab aplikasi saat pengguna tidak sedang mengetik di dalam text input / form:

| Hotkey | Aksi | Keterangan |
| :--- | :--- | :--- |
| `1` | Switch ke **Tab 1: Agent Workspace** | Mode obrolan coding agent, tasks, file tree, tool diff review |
| `2` | Switch ke **Tab 2: Antigravity** | Provider OAuth & Quota Antigravity |
| `3` | Switch ke **Tab 3: Codex** | Provider OAuth & Quota Codex |
| `4` | Switch ke **Tab 4: Claude** | Provider OAuth & Quota Claude |
| `5` | Switch ke **Tab 5: Custom LLM** | Endpoint kustom OpenAI-compatible / Ollama |
| `q` | **Quit Application** | Keluar dari aplikasi |
| `Ctrl+c` | **Cancel Turn / Quit** | Menghentikan agent turn jika sedang `is_running`, atau quit aplikasi jika idle |

---

## 1️⃣ Tab 1: Agent Workspace (`ragasukma-tui/src/agent_view`)

### 🔲 Focus Areas (`AgentFocusArea`)

Panel/kartu di Tab 1 terdiri dari:
1. `Folders`: Workspace Folder Manager (atas sidebar)
2. `Tasks`: Riwayat sesi task agent (tab sidebar)
3. `Files`: Explorer file tree workspace (tab sidebar)
4. `Chat`: Transcript pesan percakapan dan output reasoning
5. `Input`: Text prompt input prompt (memiliki Normal & Insert Mode)
6. `ModelTelemetry`: Widget info model & metrik token/latensi
7. `McpTools`: Inspector tool execution & MCP calls

Navigasi fokus panel:
- `Tab`: Pindah fokus ke panel berikutnya (`AgentFocusNextPanel`).
- `Shift+Tab` / `BackTab`: Pindah fokus ke panel sebelumnya (`AgentFocusPrevPanel`).
- `b`: Toggle buka / tutup Sidebar (`AgentToggleSidebar`).
- `m`: Membuka modal pemilihan model (`Select Model Modal`).
- `P`: Membuka modal switcher project (`Project Switcher Modal`).
- `r`: Toggle status reasoning collapse/expand (saat tidak fokus di Files).

---

### ⌨️ Hotkey & Footer Helper per Focus Area di Tab 1

#### 1. Panel: `Folders` (`AgentFocusArea::Folders`)
* **Footer Helper:**
  `[Tab] Focus` | `[j/k or ↑/↓] Select` | `[a] Add` | `[d] Remove` | `[o] Open` | `[P] Projects` | `[b] Sidebar` | `[q] Quit`
* **Daftar Hotkey:**
  - `j` / `Down`: Pilih folder berikutnya.
  - `k` / `Up`: Pilih folder sebelumnya.
  - `a`: Membuka modal dialog **Add Folder to Workspace**.
  - `d`: Menghapus folder yang dipilih dari workspace aktif.
  - `o`: Membuka folder yang dipilih di File Manager sistem OS.
  - `P`: Membuka dialog daftar project (`Project Switcher`).

#### 2. Panel: `Tasks` (`AgentFocusArea::Tasks`)
* **Footer Helper:**
  `[Tab] Focus` | `[j/k or ↑/↓] Select` | `[n] New` | `[d] Delete` | `[s] Files Tab` | `[b] Sidebar` | `[q] Quit`
* **Daftar Hotkey:**
  - `j` / `Down`: Pilih dan langsung switch ke task session berikutnya.
  - `k` / `Up`: Pilih dan langsung switch ke task session sebelumnya.
  - `n`: Membuat task session baru.
  - `d`: Menghapus task session yang dipilih.
  - `s`: Berpindah tab sidebar dari Tasks ke Files.

#### 3. Panel: `Files` (`AgentFocusArea::Files`)
* **Footer Helper:**
  `[Tab] Focus` | `[j/k or ↑/↓] Select` | `[Enter] Expand` | `[Space] Insert Path` | `[a] Add Folder` | `[r] Rescan` | `[s] Tasks Tab` | `[o] Open` | `[q] Quit`
* **Daftar Hotkey:**
  - `j` / `Down`: Sorot file/direktori berikutnya.
  - `k` / `Up`: Sorot file/direktori sebelumnya.
  - `Enter`: Expand / collapse direktori dalam file tree.
  - `Space`: Memasukkan path file terpilih langsung ke text input prompt (`@path/to/file`).
  - `a`: Membuka modal dialog **Add Folder to Workspace**.
  - `r`: Rescan / scan ulang isi direktori file tree.
  - `s`: Berpindah tab sidebar dari Files ke Tasks.
  - `o`: Membuka path file/direktori terpilih di File Manager sistem OS.

#### 4. Panel: `Chat` (`AgentFocusArea::Chat`)
* **Footer Helper:**
  `[Tab] Focus` | `[i/Enter] Write` | `[j/k or ↑/↓] Scroll` | `[m] Model` | `[b] Sidebar` | `[q] Quit`
* **Daftar Hotkey:**
  - `i` atau `Enter`: Masuk ke **Insert Mode** pada text input prompt.
  - `j` / `Down`: Gulir riwayat pesan ke bawah.
  - `k` / `Up`: Gulir riwayat pesan ke atas.
  - `m`: Membuka popup pemilihan model (`Select Model Modal`).
  - `b`: Toggle tampilkan/sembunyikan sidebar.

#### 5. Panel: `Input` (`AgentFocusArea::Input`)

Terdapat 2 mode saat berada di area input:

##### A. Insert Mode (`is_input_focused = true`)
* **Footer Helper:**
  `[Enter] Send` | `[@] Tag File/Folder` | `[Esc] Normal Mode` | `[Tab] Next Focus` | `[Ctrl+C] Stop Agent`
* **Daftar Hotkey:**
  - `Enter`: Kirim prompt ke agent (`SubmitPrompt`).
  - `@`: Membuka popup autocomplete tag file/folder (`Tag Query`).
  - `Esc`: Keluar dari mode pengetikan kembali ke **Normal Mode** (`UnfocusInput`).
  - `Backspace`: Hapus karakter di posisi kursor.
  - `Ctrl+c`: Membatalkan giliran agent yang sedang berjalan (`CancelTurn`), atau keluar aplikasi jika idle.
  - `Tab`: Pindah fokus ke panel berikutnya.
  - `Shift+Tab` / `BackTab`: Pindah fokus ke panel sebelumnya.

##### B. Normal Mode (`is_input_focused = false`)
* **Footer Helper:**
  `[Tab] Focus` | `[i/Enter] Write` | `[m] Model` | `[↑/↓] Scroll Chat` | `[q] Quit`
* **Daftar Hotkey:**
  - `i` atau `Enter`: Masuk ke **Insert Mode** untuk mulai mengetik.
  - `m`: Membuka modal pemilihan model.
  - `j` / `Down`: Gulir riwayat chat ke bawah.
  - `k` / `Up`: Gulir riwayat chat ke atas.

#### 6. Panel: `ModelTelemetry` (`AgentFocusArea::ModelTelemetry`)
* **Footer Helper:**
  `[Tab] Focus` | `[m] Cycle Model` | `[f/F5] Refresh` | `[b] Sidebar` | `[q] Quit`
* **Daftar Hotkey:**
  - `m`: Buka modal ganti model.
  - `f` atau `F5`: Refresh data provider dan daftar model dari backend.

#### 7. Panel: `McpTools` (`AgentFocusArea::McpTools`)
* **Footer Helper:**
  `[Tab] Focus` | `[j/k] Select` | `[t] Filter` | `[x] Expand/Collapse` | `[q] Quit`
* **Daftar Hotkey:**
  - `j` / `Down`: Gulir daftar pemanggilan tool ke bawah.
  - `k` / `Up`: Gulir daftar pemanggilan tool ke atas.
  - `t`: Ganti filter status tool (`All` ➔ `RunningOnly` ➔ `FailedOnly`).
  - `x`: Toggle expand / collapse detail diff & output tool yang dipilih.

---

### 🪟 Modals & Popups di Tab 1

#### 1. Add Folder Modal (`add_folder_modal`)
- **Trigger:** Tekan `a` pada panel `Folders` atau `Files`.
- **Hotkey:**
  - `Karakter`: Mengetik path direktori.
  - `Backspace`: Menghapus karakter.
  - `Tab`: Buka native dialog browse folder OS.
  - `Enter`: Submit dan tambahkan folder ke workspace.
  - `Esc`: Batal dan tutup modal.

#### 2. Project Switcher Modal (`project_modal`)
- **Trigger:** Tekan `P` (Shift+p).
- **Hotkey:**
  - `j` / `Down`: Sorot project berikutnya.
  - `k` / `Up`: Sorot project sebelumnya.
  - `Enter`: Switch ke project terpilih.
  - `n`: Buka dialog pembuatan project baru (`Create Project Modal`).
  - `Esc`: Tutup modal.

#### 3. Create Project Modal (`create_project_modal`)
- **Trigger:** Tekan `n` saat berada di Project Switcher Modal.
- **Hotkey:**
  - `Karakter`: Ketik nama project baru.
  - `Backspace`: Hapus karakter.
  - `Enter`: Submit & buat project baru.
  - `Esc`: Batal dan tutup modal.

#### 4. Select Model Modal (`model_modal`)
- **Trigger:** Tekan `m` di Chat, Input (Normal Mode), atau Telemetry.
- **Hotkey:**
  - `j` / `Down` / `Tab`: Sorot model berikutnya.
  - `k` / `Up` / `BackTab`: Sorot model sebelumnya.
  - `Karakter`: Filter nama model.
  - `Backspace`: Hapus filter karakter.
  - `Enter`: Terapkan model yang dipilih ke agent workspace.
  - `Esc`: Batal & tutup modal.

#### 5. Tag Query Autocomplete Popup (`tag_query`)
- **Trigger:** Mengetik karakter `@` saat sedang berada di `Insert Mode` input prompt.
- **Hotkey:**
  - `Down`: Sorot hasil file/folder autocomplete berikutnya.
  - `Up`: Sorot hasil file/folder autocomplete sebelumnya.
  - `Enter` atau `Tab`: Masukkan tag path file yang dipilih ke dalam input prompt.
  - `Esc`: Tutup popup autocomplete tag.

---

## 2️⃣-4️⃣ Tab 2, 3, 4: Providers (`ragasukma-tui/src/provider_view`)
*(Tab 2: Antigravity | Tab 3: Codex | Tab 4: Claude)*

### 🔲 Focus Areas & Cards
Setiap tab provider terbagi menjadi 2 area/kartu utama yang dapat ditukar dengan tombol `Tab`:
1. **Accounts Card** (`ProviderFocusArea::Accounts`) - Menampilkan daftar akun, status login, dan default badge.
2. **Quotas Card** (`ProviderFocusArea::Quotas`) - Menampilkan limit kuota model, sisa persentase, dan reset counter.

---

### ⌨️ Hotkey & Footer Helper per Area di Provider Tabs

#### 1. Accounts Card (`ProviderFocusArea::Accounts`)
* **Footer Helper:**
  `[Tab] Quotas` | `[j/k] Select` | `[Space] Active` | `[a] Add` | `[d] Delete` | `[r] Refresh` | `[1-5] Tabs` | `[q] Quit`
* **Daftar Hotkey:**
  - `Tab`: Pindah fokus ke **Quotas Card**.
  - `j` / `Down`: Sorot akun berikutnya.
  - `k` / `Up`: Sorot akun sebelumnya.
  - `Space`: Toggle status aktif / tidak aktif untuk akun yang dipilih (`ToggleAccountActive`).
  - `Enter`: Menjadikan akun yang dipilih sebagai akun default (`SetAccountActive`).
  - `a`: Membuka browser untuk memulai alur OAuth login provider (`StartAntigravityOAuth` / `StartCodexOAuth` / `StartClaudeOAuth`).
  - `d`: Menghapus akun yang dipilih beserta kredensial penyimpanannya (`ConfirmDelete`).
  - `r` / `R`: Refresh data kuota akun yang sedang dipilih (`RefreshQuota`).
  - `Ctrl+j` / `Ctrl+Down`: Scroll daftar Quota ke bawah tanpa harus memindahkan fokus.
  - `Ctrl+k` / `Ctrl+Up`: Scroll daftar Quota ke atas tanpa harus memindahkan fokus.

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

### 🔲 Focus Areas & Cards
Tab ini terbagi menjadi 2 kartu horizontal yang ditukar dengan tombol `Tab`:
1. **Providers Card** (`CustomLlmFocus::Providers`) - Tabel konfigurasi prefix, nama provider, base URL endpoint, dan status ON/OFF.
2. **Models Catalog Card** (`CustomLlmFocus::Models`) - Tabel model yang tersedia untuk provider yang sedang dipilih.

---

### ⌨️ Hotkey & Footer Helper di Tab 5

#### Normal Mode (Tanpa Modal Terbuka)
* **Footer Helper:**
  `[Tab] Switch Focus` | `[j/k] Select` | `[a] Add Provider` | `[e] Edit` | `[Space] Toggle` | `[d] Delete` | `[r] Fetch Models` | `[1-5] Tabs` | `[q] Quit`
* **Daftar Hotkey:**
  - `Tab`: Berpindah fokus antara **Providers Card** dan **Models Catalog Card**.
  - `j` / `Down`: Memilih entri berikutnya pada tabel aktif.
  - `k` / `Up`: Memilih entri sebelumnya pada tabel aktif.
  - `a`: Membuka formulir modal **Add Custom Provider** (`CustomLlmModal::Add`).
  - `e`: Membuka formulir modal **Edit Custom Provider** untuk provider yang sedang dipilih (`CustomLlmModal::Edit`).
  - `Space`: Mengaktifkan / menonaktifkan status auto-routing (`ON` / `OFF`).
  - `d`: Membuka dialog konfirmasi hapus provider (`CustomLlmModal::DeleteConfirm`).
  - `r` / `R`: Fetch / query katalog model secara realtime dari endpoint `/v1/models`.

---

### 🪟 Modals di Tab 5

#### 1. Add / Edit Provider Modal (`render_form_modal`)
* **Footer Helper:**
  `[Tab/Shift+Tab] Next/Prev Field` | `[Enter] Submit` | `[Esc] Cancel`
* **Daftar Hotkey Form Modal:**
  - `Tab` / `Down`: Berpindah ke input field berikutnya dengan urutan siklus:
    `Prefix (/custom)` ➔ `Provider Name` ➔ `Base URL Endpoint` ➔ `API Key` ➔ `Provider Enabled Checkbox` ➔ `[Save & Apply]` ➔ `[Cancel]`.
  - `Shift+Tab` / `Up`: Berpindah ke field form sebelumnya.
  - `Space`: Mengubah status centang `[✔]` / `[ ]` jika berada pada field Enabled Checkbox.
  - `Enter`: Submit form dan menyimpan konfigurasi jika berada di input field atau di tombol `[Save & Apply]`.
  - `Esc`: Membatalkan input dan menutup formulir modal tanpa menyimpan.

#### 2. Confirm Deletion Modal (`render_delete_modal`)
* **Daftar Hotkey Modal Hapus:**
  - `y` / `Enter`: Mengonfirmasi penghapusan custom provider beserta kredensial tersimpan.
  - `n` / `Esc`: Membatalkan penghapusan dan kembali ke kartu provider.


