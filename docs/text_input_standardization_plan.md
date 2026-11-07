# Rencana Implementasi: Standarisasi Reactive Text Input System di Terminal

> **Status:** Draft / Siap Dieksekusi  
> **Target:** `@ragasukma-tui/`  
> **Tujuan Utama:** Menghapus duplikasi kode string & kursor di seluruh view/modal, menyatukan keyboard handling ke dalam komponen terpadu `TextInput`, menjamin isolasi event reaktif saat input aktif, dan menyediakan pengalaman pengguna (UX) terminal standar (CLI/Readline shortcuts).

---

## 1. Latar Belakang & Identifikasi Masalah

Saat ini, penanganan input teks di `ragasukma-tui` mengalami redundansi dan fragmentasi yang tersebar di banyak berkas:

1. **State & Kursor Terduplikasi**:
   - `custom_llm_view`: Menggunakan field string mentah (`form_prefix`, `form_name`, `form_base_url`, `form_api_key`) tanpa kursor independen pada tiap field.
   - `mcp_server_view`: Menggunakan `form_name`, `form_json`, serta tracking manual `name_cursor: usize` dan `json_cursor: usize`.
   - `agent_view`: Menggunakan `input_buffer`, `cursor_pos`, ditambah modal-modal internal (`add_folder_modal_path`, `create_project_name`, `model_query`).
2. **Explosion Varian Action di `app/types.rs`**:
   Banyak varian `Action` dibuat hanya untuk fungsi dasar mengetik karakter dan backspace di masing-masing konteks:
   - `AgentInsertChar`, `AgentBackspace`, `AgentInputDelete`
   - `FormChar`, `FormBackspace`, `FormPaste`
   - `AgentAddFolderChar`, `AgentAddFolderBackspace`
   - `AgentCreateProjectChar`, `AgentCreateProjectBackspace`
   - `AgentModelModalChar`, `AgentModelModalBackspace`
3. **Logika Render Kursor Terpisah**:
   25 |    Masing-masing UI view menghitung manual posisi koordinat visual `frame.set_cursor_position(Position::new(x, y))` dengan rumus offset border dan scroll horizontal yang rawan meleset (misal saat teks melebihi lebar kotak).
   26 | 4. **Penanganan Baris Baru (Newline) yang Terfragmentasi**:
   27 |    - Di `agent_view`: Prompt multiline menggunakan `Alt+J` untuk `\n`, sedangkan `Enter` biasa mengirim pesan (submit), dan variasi `Shift+Enter`/`Ctrl+Enter` dijadikan *dead key* (swallowed) untuk mencegah pesan terkirim prematur akibat ambiguitas terminal emulator standar.
   28 |    - Di `mcp_server_view`: Field JSON Config mengartikan `Enter` sebagai `\n` (`FormNewline`), sedangkan submit dilakukan lewat tombol Save terpisah.
   29 |    - Di form dan modal lain: Seluruh input bersifat single-line, namun belum ada proteksi seragam jika pengguna mem-paste teks yang mengandung karakter newline (`\r\n` atau `\n`).
   30 | 5. **Minimnya Navigasi Kata / Readline Shortcuts**:
   31 |    Pengguna terminal terbiasa dengan shortcut standar seperti `Ctrl+W` (hapus kata), `Ctrl+U` (hapus hingga awal baris), dan `Home`/`End` yang belum seragam di seluruh modal.

---

## 2. Arsitektur Target (Design Specifications)

### 2.1 Komponen Inti: `TextInput` (`src/ui/text_input.rs`)

Sebuah struct mandiri yang mengelola buffer teks, kursor, scroll, placeholder, serta penanganan event:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputResult {
    /// Karakter diketik, dihapus, atau kursor dipindahkan
    Consumed,
    /// Pengguna menekan Enter (submit)
    Submit,
    /// Pengguna menekan Alt+J / Shift+Enter untuk newline di mode multiline
    NewlineInserted,
    /// Pengguna menekan Tab atau Down (pindah ke field berikutnya)
    NextField,
    /// Pengguna menekan BackTab (Shift+Tab) atau Up (pindah ke field sebelumnya)
    PrevField,
    /// Pengguna menekan Esc (batalkan / tutup form)
    Cancelled,
    /// Tombol tidak dikenali / diabaikan oleh input field
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputMode {
    /// Field satu baris (modal pencarian, nama project, API key, base URL, dll).
    /// Karakter '\n' difilter/dibersihkan saat paste; Enter selalu memicu InputResult::Submit.
    SingleLine,
    /// Field multi baris dengan perilaku Submit on Enter (khusus Prompt Agent).
    /// Bare Enter memicu InputResult::Submit; Alt+J (atau Shift+Enter jika didukung) memasukkan '\n'.
    MultiLinePrompt,
    /// Field multi baris editor kode/konfigurasi (JSON/Markdown di MCP Server).
    /// Enter memasukkan karakter '\n'; Submit dilakukan via tombol Save atau shortcut form terpisah.
    MultiLineEditor,
}

#[derive(Debug, Clone)]
pub struct TextInput {
    pub value: String,
    pub cursor: usize,          // Byte offset kursor pada string
    pub scroll: usize,          // Horizontal character offset untuk teks panjang
    pub placeholder: String,    // Placeholder saat value kosong
    pub is_focused: bool,       // Status apakah sedang menerima keyboard input
    pub is_password: bool,      // Masking karakter (misal API key: "••••••")
    pub max_length: Option<usize>,
    pub mode: TextInputMode,    // Mode penanganan newline dan Enter
}
```

### 2.2 Kebijakan Standarisasi Newline & Paste Terminal

| Komponen & Lokasi | Mode Input | Perilaku `Enter` | Perilaku Newline (`Alt+J` / `Shift+Enter`) | Perilaku Paste Multiline |
| :--- | :--- | :--- | :--- | :--- |
| **Agent Prompt** (`agent_view`) | `MultiLinePrompt` | **Submit Prompt** | **Insert `\n`** (`Alt+J` deterministik, `Shift+Enter` via enhanced protocol) | Seluruh baris dipertahankan dan disisipkan |
| **MCP JSON/Markdown Config** (`mcp_server_view`) | `MultiLineEditor` | **Insert `\n`** | **Insert `\n`** | Seluruh baris dipertahankan dan disisipkan |
| **Custom LLM Form** (`custom_llm_view`) | `SingleLine` | Next Field / Submit | *Diabaikan (Dead Key)* | Karakter `\r` / `\n` diganti dengan spasi atau dibuang |
| **Modal Add Folder / Project** (`agent_view`) | `SingleLine` | Konfirmasi Path/Name | *Diabaikan (Dead Key)* | `\r` dan `\n` dibuang demi integritas path file/folder |
| **Modal Model Picker Filter** (`agent_view`) | `SingleLine` | Pilih Model Sorotan | *Diabaikan (Dead Key)* | `\r` dan `\n` dibuang |

#### Mengapa `Alt+J` Digunakan Sebagai Newline Deterministik?
1. Pada mode mentah terminal standar (`raw mode`), sebagian besar emulator mengirimkan byte `0x0D` (`\r`) baik saat pengguna menekan `Enter` biasa maupun `Shift+Enter`.
2. Jika aplikasi mengandalkan `Shift+Enter` tanpa protokol lanjutan, kombinasi tombol tersebut berisiko memicu pengiriman pesan yang belum selesai.
3. Di `ragasukma-tui`, variasi kombinasi `Shift+Enter` dan `Ctrl+Enter` pada terminal dasar sengaja di-*swallow* (dimatikan) agar pesan tidak terkirim secara tidak sengaja.
4. `Alt+J` mengirimkan escape sequence `ESC + 'j'` yang 100% konsisten di semua emulator OS (Windows Terminal, Linux XTerm, macOS Terminal) tanpa dependensi protokol eksternal.
5. Pada Fase 5, deteksi protokol modern (*Kitty Keyboard Protocol*) akan ditambahkan sebagai enhancement untuk mendeteksi `Shift+Enter` fisik bila terminal mendukungnya.
```

### 2.2 Kemampuan Manipulasi Teks & Readline Shortcuts
Method `TextInput` mencakup:
- **Penulisan**: `insert_char(c: char)`, `paste(text: &str)` (sanitasi newline jika single-line).
- **Penghapusan**:
  - `backspace()`: Hapus 1 karakter UTF-8 di belakang kursor.
  - `delete_forward()`: Hapus 1 karakter di depan kursor.
  - `delete_word_backward()` (`Ctrl+W`): Hapus 1 kata ke belakang.
  - `clear_to_start()` (`Ctrl+U`): Hapus teks dari awal baris ke posisi kursor saat ini.
  - `clear_to_end()` (`Ctrl+K`): Hapus teks dari posisi kursor ke akhir baris.
  - `clear()`: Kosongkan seluruh buffer.
- **Navigasi Kursor**:
  - `cursor_left()`, `cursor_right()`
  - `cursor_home()` (`Home`, `Ctrl+A`)
  - `cursor_end()` (`End`, `Ctrl+E`)
  - `word_left()` (`Alt+Left`, `Alt+B`), `word_right()` (`Alt+Right`, `Alt+F`)
- **Event Dispatcher**:
  ```rust
  impl TextInput {
      pub fn handle_key(&mut self, key: KeyEvent) -> InputResult {
          // Mengolah tombol sesuai modifier dan menghasilkan InputResult
      }
  }
  ```

### 2.3 Widget Helper Render Terpadu
```rust
pub fn render_text_input(
    frame: &mut Frame,
    area: Rect,
    input: &TextInput,
    title: &str,
    theme: &Theme,
) {
    // 1. Gambar Block border dengan warna aktif jika input.is_focused
    // 2. Tampilkan teks atau placeholder (jika kosong dengan gaya redup)
    // 3. Hitung visual offset kursor
    // 4. Jika input.is_focused, panggil frame.set_cursor_position(cursor_pos)
}
```

---

## 3. Urutan Fase Pengerjaan (Step-by-Step Implementation)

```
┌────────────────────────────────────────────────────────┐
│ Fase 1: Core Component & Unit Tests                   │
│ (src/ui/text_input.rs: struct, logic, readline, tests) │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│ Fase 2: Ratatui UI Widget & Precise Cursor Rendering   │
│ (Viewport clipping, scrolling, password masking)       │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│ Fase 3: Migrasi Modal Sederhana (Quick Wins)           │
│ (Add Folder Modal, Create Project Modal, Model Search) │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│ Fase 4: Migrasi Form Tab View                          │
│ (Custom LLM View & MCP Server View)                    │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│ Fase 5: Standarisasi Multi-line & Prompt Agent View    │
│ (Prompt History Up/Down, Tag query autocomplete hook)  │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│ Fase 6: Pembersihan Action Enum & Validasi Akhir       │
│ (Hapus dead code action di types.rs, cargo test)       │
└────────────────────────────────────────────────────────┘
```

---

### Fase 1: Core Component & Unit Tests
**Target Lokasi:** `src/ui/text_input.rs`, `src/ui.rs`

- [ ] Definisikan struct `TextInput` dan enum `InputResult`.
- [ ] Implementasikan operasi dasar string berorientasi batas karakter UTF-8 (`is_char_boundary`).
- [ ] Implementasikan Readline shortcuts:
  - `Ctrl+W`: Hapus kata sebelumnya.
  - `Ctrl+U`: Hapus hingga awal baris.
  - `Ctrl+K`: Hapus hingga akhir baris.
  - `Ctrl+A` / `Home`: Pindah kursor ke awal.
  - `Ctrl+E` / `End`: Pindah kursor ke akhir.
- [ ] Implementasikan `handle_key(&mut self, key: KeyEvent) -> InputResult`.
- [ ] Buat comprehensive unit tests:
  - Input karakter ASCII dan UTF-8 multibyte.
  - Edge case kursor di indeks 0 dan di akhir string.
  - Paste teks panjang dan pergeseran kursor yang tepat.
  - Readline shortcut deletion behaviors.

---

### Fase 2: Ratatui UI Widget & Precise Cursor Rendering
**Target Lokasi:** `src/ui/text_input.rs`

- [ ] Implementasikan fungsi render:
  `pub fn render_text_input(frame: &mut Frame, area: Rect, input: &TextInput, title: &str, theme: &Theme)`
- [ ] Tambahkan kalkulasi horizontal scroll: jika panjang teks melebihi lebar area input, kursor tetap terlihat di viewport aktif dengan auto-scroll horizontal.
- [ ] Tampilkan placeholder jika buffer kosong dan tidak sedang difokuskan (atau redup saat difokuskan).
- [ ] Terapkan mask untuk field bertipe password/API key (`•` atau `*`).
- [ ] Otomatis set `frame.set_cursor_position()` hanya bila `input.is_focused == true`.

---

### Fase 3: Migrasi Modal Input Sederhana (Quick Wins)
**Target Lokasi:** `src/agent_view/states/modal.rs`, `src/agent_view/controller.rs`, `src/agent_view/ui/modals.rs`

- [ ] **Add Folder Modal**:
  - Ganti `add_folder_modal_path: String` dengan `folder_path_input: TextInput`.
  - Hapus action `AgentAddFolderChar` dan `AgentAddFolderBackspace`.
  - Pasang `render_text_input` di dialog render modal.
- [ ] **Create Project Modal**:
  - Ganti `create_project_name: String` dengan `project_name_input: TextInput`.
  - Hapus action `AgentCreateProjectChar` dan `AgentCreateProjectBackspace`.
- [ ] **Model Search Modal**:
  - Ganti `model_query: String` dengan `model_search_input: TextInput`.
  - Hapus action `AgentModelModalChar` dan `AgentModelModalBackspace`.
- [ ] Jalankan verifikasi `cargo check` & `cargo test` untuk memastikan modal berfungsi normal.

---

### Fase 4: Migrasi Form Tab View (`custom_llm_view` & `mcp_server_view`)
**Target Lokasi:** `src/custom_llm_view/`, `src/mcp_server_view/`

- [ ] **Custom LLM View**:
  - Refactor field form modal menjadi `TextInput`:
    - `form_prefix: TextInput`
    - `form_name: TextInput`
    - `form_base_url: TextInput`
    - `form_api_key: TextInput` (dengan `is_password = true`)
  - Navigasi antar field menggunakan kembalian `InputResult::NextField` dan `InputResult::PrevField`.
  - Gunakan `render_text_input` pada form layout.
- [ ] **MCP Server View**:
  - Ganti `form_name`, `form_json`, `name_cursor`, `json_cursor` dengan `name_input: TextInput` dan `json_input: TextInput`.
  - Hilangkan state kursor manual.
- [ ] Pangkas routing key di controller masing-masing tab sehingga tidak lagi mem-parsing tombol karakter satu per satu secara manual.

---

### Fase 5: Standarisasi Multi-line & Prompt Agent View
**Target Lokasi:** `src/agent_view/states/method_input.rs`, `src/agent_view/controller.rs`

- [ ] Evaluasi penggabungan `input_buffer` agent ke `TextInput` yang mendukung multi-line:
  - Pertahankan shortcut khusus terminal: `Alt+J` untuk newline.
  - Deteksi opsional `Shift+Enter` jika terminal mendukung protokol keyboard modern.
- [ ] **Prompt History Navigation**:
  - Tambahkan `history: Vec<String>` dan `history_index: Option<usize>`.
  - Navigasi `Up` pada buffer kosong atau baris paling atas untuk memuat riwayat prompt sebelumnya.
  - Navigasi `Down` untuk kembali ke prompt terbaru / draft saat ini.
- [ ] Integrasikan autocomplete tag popover (`@folder`, `#file`) agar terpicu mulus setelah perubahan buffer di `TextInput`.

---

### Fase 6: Pembersihan Action Enum & Validasi Akhir
**Target Lokasi:** `src/app/types.rs`, `src/app/controller.rs`

- [ ] Bersihkan varian-varian `Action` usang di `src/app/types.rs`.
- [ ] Pastikan fungsi global `app.is_text_input_active()` tetap konsisten mengembalikan `true` ketika salah satu `TextInput.is_focused` aktif.
- [ ] Jalankan suite testing lengkap (`cargo test`) dan pastikan seluruh test suite lolos tanpa warning regresi.

---

## 4. Keuntungan Setelah Selesai

1. **Pengurangan Beban Kode (Code Reduction)**: Ratusan baris kode manipulasi teks manual dan kalkulasi kursor visual terhapus.
2. **Pengalaman Pengguna (UX) Seragam**: Seluruh input terminal di aplikasi mendukung shortcut standar (`Ctrl+W`, `Ctrl+U`, `Home`, `End`, arrow keys, paste) secara konsisten.
3. **Ketahanan Kode (Robustness)**: Mengeliminasi potensi bug slicing boundary UTF-8 dan kursor yang tidak sinkron di modal maupun form.
