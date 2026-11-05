# Audit Codebase dan Rencana Perbaikan Bertahap (Phases 1-5)

Dokumen ini berisi hasil audit teknis mendalam terhadap arsitektur dan kode sumber `ragasukma-tui` serta roadmap langkah perbaikan terstruktur yang dibagi ke dalam 5 fase.

---

## Ringkasan Hasil Audit

| Kategori | Tingkat Keparahan | Lokasi Utama | Status / Catatan |
|---|---|---|---|
| **1. Memory & Resource Leak** | Sedang - Tinggi | `src/core_bridge/worker.rs`, `src/event.rs`, `src/agent_view/state.rs` | Infinite loop task tanpa exit condition; task `EventHandler` tanpa `Drop`; pesan chat unbounded di memori. |
| **2. Race Condition** | Kritis | `src/core_bridge/worker.rs:15-125`, `src/core_bridge/worker.rs:500-532` | **Double Subscription**: global subscriber dan per-turn forwarder aktif bersamaan; resubscribe di setiap event loop. |
| **3. Dead Code** | Rendah | `src/agent_view/update.rs:110-120` | Action tag query (`AgentTagQueryNext`, dll.) ada di state handler tetapi belum memiliki binding keyboard penuh. |
| **4. Redundant Code** | Sedang | `src/core_bridge/worker.rs` | Duplikasi logika besar antara `ExecuteTurn` dan `RetryTurn`; mapping event session diulang di 3 tempat. |
| **5. Potensi Panic & Crash** | Sedang | `src/main.rs`, `src/terminal.rs` | Ketiadaan panic hook kustom untuk merestorasi raw mode terminal jika terjadi panic tak terduga di loop aplikasi. |

---

## Detail Analisis Temuan

### 1. Memory Leak & Resource Leak
1. **Unbounded Task pada `spawn_session_subscriber` (`src/core_bridge/worker.rs`)**:
   - Loop `tokio::spawn(async move { loop { ... } })` tidak mengecek `sub_tx.is_closed()`. Jika UI atau bridge di-drop/reconnect, task subscriber lama tetap hidup di background Tokio runtime dan terus menjalankan polling/subscribing.
2. **Ketiadaan Implementasi `Drop` pada `EventHandler` (`src/event.rs`)**:
   - Field `_task: JoinHandle<()>` tidak di-abort saat instance `EventHandler` keluar dari scope. Jika handler diinisialisasi ulang, task lama crossterm tetap aktif.
3. **Riwayat Chat Tanpa Batas (Unbounded Buffer)**:
   - `agent.messages: Vec<ChatMessage>` dan `tool_calls` bertambah secara linear tanpa strategi truncation, pagination, atau eviction limit. Pada sesi yang sangat panjang dengan banyak tool call bermuatan data besar, konsumsi RAM akan terus meningkat.

### 2. Race Condition & Concurrency
1. **Double Event Delivery (Duplikasi Streaming)**:
   - `spawn_session_subscriber` berjalan secara kontinu untuk sesi aktif. Di saat yang sama, blok `ExecuteTurn` dan `RetryTurn` mem-spawn `stream_forwarder` lokal yang juga mendengarkan `get_session_sender(&sid_exec)`.
   - Kedua subscriber mengirimkan event `Delta`, `ThoughtDelta`, `StatusChange`, dan `StepInfo` ke `stream_tx` yang sama. Akibatnya, UI menerima token ganda atau token stream saling tumpang tindih (*interleaved*).
2. **Event Dropping / Skipping akibat Resubscribe Berulang**:
   - Di dalam loop `spawn_session_subscriber`, pemanggilan `get_session_sender(&cur).subscribe()` dijalankan di setiap iterasi loop penerimaan event. Kursor broadcast channel di-reset ke tail terkini, sehingga event beruntun (*burst*) yang masuk di antara iterasi berpotensi terlewat.

### 3. Dead Code & Unused Handlers
1. **Unmapped Action Variants**:
   - Varian `Action::AgentTagQueryNext`, `Action::AgentTagQueryPrev`, dan `Action::AgentApplyTag` sudah diimplementasikan di `agent_view/update.rs`, namun belum dipetakan ke keybindings aktif di `src/event.rs`.
2. **Clippy Strict Lints**:
   - Default compiler tidak memunculkan warning, namun lints ketat seperti `-W dead_code -W unused` perlu dipasang di CI pipeline agar mencegah fungsi yang sudah tidak dipakai terakumulasi.

### 4. Redundant Code
1. **Duplikasi Eksekusi Turn (`ExecuteTurn` vs `RetryTurn`)**:
   - Penyiapan `stream_forwarder`, pemanggilan `ragasukma_core::agent::executor::execute_session_turn`, pencatatan waktu (`turn_start`), perekaman penggunaan token (`record_turn_usage`), serta penanganan error sama persis antara kedua cabang komando.
2. **Duplikasi Mapping Enum**:
   - Logika translasi `AgentSessionEvent` -> `CoreBridgeEvent` disalin di tiga tempat berbeda.

### 5. Potensi Panic & Terminal State Corruption
1. **Terminal Raw Mode Pasca Panic**:
   - Di `src/main.rs`, fungsi `ratatui::restore()` hanya dipanggil saat keluar normal dari `app.run()`. Apabila terjadi runtime panic di thread utama (misal rendering glitch atau indexing tak terduga), proses crash sebelum raw mode dinonaktifkan, menyebabkan terminal terminal host rusak/tidak responsif terhadap tombol keyboard biasa.

---

## Roadmap Fase Perbaikan (Phase 1 s/d Phase 5)

```
[ Phase 1 ] Kritis: Perbaikan Race Condition & Deduplikasi Stream
     │
     ▼
[ Phase 2 ] Stabilitas: Eliminasi Resource Leak & Task Cancellation
     │
     ▼
[ Phase 3 ] Ketahanan: Terminal Safety, Panic Hook & Error Handling
     │
     ▼
[ Phase 4 ] Refactoring: Konsolidasi Redundan (Turn Execution & Event Mapper)
     │
     ▼
[ Phase 5 ] Pembersihan & Integrasi: Dead Code, Keybinding Wiring, Strict Lint
```

---

### Phase 1: Perbaikan Concurrency & Stream Race Condition (Prioritas Tertinggi)

**Tujuan**: Menghilangkan duplikasi token streaming pada antarmuka TUI dan menjamin integritas urutan event.

1. **Konsolidasi Mekanisme Subscription**:
   - Pilih satu arsitektur stream:
     - **Opsi Terpilih (Rekomendasi)**: Hapus `stream_forwarder` lokal di dalam `ExecuteTurn` dan `RetryTurn`. Serahkan seluruh penerusan event sesi kepada subscriber terpusat (`spawn_session_subscriber`), ATAU sebaliknya (gunakan subscriber per-turn dengan lifecycle jelas dan hilangkan subscriber loop liar).
2. **Hilangkan Resubscribe Berulang dalam Loop**:
   - Buat receiver broadcast satu kali di awal sesi atau saat session ID berubah, bukan di setiap iterasi loop:
     ```rust
     let mut broadcast_rx = get_session_sender(&cur).subscribe();
     while let Ok(evt) = broadcast_rx.recv().await {
         // Kirim event ke sub_tx
     }
     ```
3. **Validasi**:
   - Jalankan prompt streaming panjang; pastikan tidak ada teks yang muncul dobel atau karakter melompat di TUI.

---

### Phase 2: Eliminasi Memory & Resource Leak

**Tujuan**: Mencegah task background berjalan tanpa batas dan mengontrol konsumsi memori chat history.

1. **Task Cancellation & Exit Guard pada Subscriber**:
   - Tambahkan pengecekan penutupan channel di loop background:
     ```rust
     if sub_tx.is_closed() {
         break;
     }
     ```
2. **Implementasikan `Drop` untuk `EventHandler`**:
   - Di `src/event.rs`, implementasikan trait `Drop`:
     ```rust
     impl Drop for EventHandler {
         fn drop(&mut self) {
             self._task.abort();
         }
     }
     ```
3. **Ring Buffer / Eviction Policy untuk Chat History**:
   - Terapkan batas maksimum pesan aktif di memori (misal 200 pesan terakhir). Pesan terdahulu tetap tersimpan aman di basis data SQLite melalui `ragasukma-core`.

---

### Phase 3: Ketahanan Panic & Terminal Recovery Safety

**Tujuan**: Menjamin terminal pengguna selalu kembali ke kondisi normal meskipun terjadi panic yang tidak terduga.

1. **Pemasangan Custom Panic Hook**:
   - Di awal `main.rs`, pasang panic hook sebelum loop aplikasi berjalan:
     ```rust
     let original_hook = std::panic::take_hook();
     std::panic::set_hook(Box::new(move |panic_info| {
         let _ = ratatui::restore();
         let _ = crossterm::terminal::disable_raw_mode();
         let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
         original_hook(panic_info);
     }));
     ```
2. **Penanganan Panic Unwind pada Run Loop**:
   - Bungkus eksekusi `app.run(&mut terminal)` dengan `std::panic::catch_unwind` atau `tokio::select!` agar proses teardown selalu dieksekusi.

---

### Phase 4: Refactoring & Eliminasi Kode Redundan

**Tujuan**: Menyederhanakan pemeliharaan kode dan mencegah inkonsistensi logika antar-fitur.

1. **Ekstraksi Helper Eksekusi Turn (`execute_turn_pipeline`)**:
   - Satukan logika `ExecuteTurn` dan `RetryTurn` di `src/core_bridge/worker.rs` menjadi satu fungsi async terpusat:
     ```rust
     async fn execute_turn_pipeline(
         session_id: String,
         model_override: Option<String>,
         prompt_for_usage: String,
         pool: SqlitePool,
         worker_tx: mpsc::UnboundedSender<CoreBridgeEvent>,
     ) -> Result<String, String>
     ```
2. **Ekstraksi Mapper Event**:
   - Buat fungsi konversi murni:
     ```rust
     fn map_core_event_to_bridge(sid: &str, evt: AgentSessionEvent) -> Option<CoreBridgeEvent>
     ```
   - Gunakan fungsi ini di seluruh pipeline bridge.

---

### Phase 5: Pembersihan Dead Code, Integrasi Keybinding & Strict Linting

**Tujuan**: Memastikan seluruh kode aktif, terhubung ke UI, dan dijaga oleh automation checks.

1. **Integrasi Keybinding Tag Navigation**:
   - Hubungkan varian `Action::AgentTagQueryNext`, `Action::AgentTagQueryPrev`, dan `Action::AgentApplyTag` ke mapping tombol di `src/event.rs` (misal saat auto-complete file/tool aktif).
2. **Aktivasi Strict Clippy Checks**:
   - Tambahkan skrip verifikasi otomatis sesuai panduan `code_quality_and_testing_guide.md`:
     ```bash
     cargo clippy --bin ragasukma-tui -- \
       -W dead_code \
       -W unused \
       -W clippy::redundant_clone \
       -W clippy::await_holding_lock \
       -W clippy::mutex_atomic \
       -W clippy::rc_buffer \
       -W clippy::panic \
       -W clippy::unwrap_used
     ```
3. **Pembersihan Dependency & Import yang Tidak Digunakan**:
   - Lakukan audit berkala terhadap import yang redundant di tiap modul TUI.

---

## Matriks Verifikasi Pasca Perbaikan

| Fase | Metode Verifikasi | Indikator Sukses |
|---|---|---|
| **Phase 1** | Streaming prompt benchmark via bridge | Token muncul sekuensial tunggal tanpa duplikasi teks di TUI. |
| **Phase 2** | Profiling thread/task via tokio-console / logging | Jumlah task subscriber Tokio tidak bertambah saat session berganti. |
| **Phase 3** | Injeksi `panic!("test")` terkontrol | Terminal prompt host kembali normal (tidak terkunci di raw mode). |
| **Phase 4** | `cargo clippy` & diff lines of code | Pengurangan baris kode duplikat >100 baris di `worker.rs`. |
| **Phase 5** | Interaksi hotkey di antarmuka TUI | Navigasi tag dan input filter responsif sesuai hotkeys reference. |
