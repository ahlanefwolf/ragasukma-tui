# Refactor Plan — Core Bridge & Runtime Hygiene

> Status: **Fase 0, 1, 2, 3, 4 selesai** — polling 50 ms hilang, diganti `watch` channel +
> cache status bersama, task lifecycle dikelola via RAII (`TaskGroup` & `ActiveTurns`),
> dan linting `clippy::unwrap_used`/`clippy::expect_used` aktif di `lib.rs` dan `main.rs`.
> Fase 5 masih backlog.
> Sumber analisis: pembacaan `src/core_bridge/worker.rs` (baris 40–215, 212–700),
> `src/core_bridge/client.rs`, `src/event.rs`, `src/service.rs`, plus audit
> `grep` untuk `unwrap` / `expect` / `panic!` / `unimplemented!` / `todo!` /
> `allow(dead_code)` di seluruh `src/`.
> Clippy/build Fase 0 telah dijalankan dan diverifikasi hijau.

---

## Ringkasan temuan

| # | Temuan | Kategori | Terbukti? |
|---|--------|----------|-----------|
| T1 | `active_sid` punya dua sumber kebenaran (`Arc<Mutex<String>>` + variabel lokal) | Race logika | Ya (kode terbaca) |
| T2 | Subscriber polling 50 ms hanya untuk sinkronisasi `String` | Inefisiensi | Ya |
| T3 | `JoinHandle` task tidak disimpan / tidak di-`abort` eksplisit | Lifecycle | Ya |
| T4 | `.expect("failed to build tokio runtime")` di `service.rs:15` | Panic path | Ya |
| T5 | `active_sid` ganda di `CreateProject` (tulis dua tempat) | Redundansi | Ya |
| T6 | `last_turn_request` — ~~mungkin dead state~~ | **Dipakai** | Ya (dipakai `RetryTurn`) |
| T7 | Klon `SqlitePool` berlapis (`pool_clone` → `pool_exec` → `pool_ref`) | Redundansi | Ya |
| T8 | Pola `let _ = tx.send(...)` diduplikasi di quota handler | Redundansi | Ya |

**Tidak ditemukan** (jangan diasumsikan ada): `Box::leak`, `mem::forget`,
siklus `Rc`/`Arc`, `unsafe` di kode yang diperiksa. Tidak ada memory leak UB.

---

## Fase 0 — Baseline (wajib duluan)

- [x] **0.1** Jalankan clippy sesuai `docs/code_quality_and_testing_guide.md`:
      ```bash
      cargo clippy --bin ragasukma-tui -- \
        -W dead_code -W unused -W clippy::redundant_clone \
        -W clippy::await_holding_lock -W clippy::mutex_atomic \
        -W clippy::rc_buffer -W clippy::panic -W clippy::unwrap_used
      ```
      Simpan output ke `docs/clippy_baseline.txt`.
- [x] **0.2** `grep -rn "last_turn_request" src/` → putuskan T6.
      **Hasil:** `worker.rs:219` (deklarasi), `worker.rs:523` (di-set di
      `ExecuteTurn`), `worker.rs:238` (dibaca di `RetryTurn`).
      → **BUKAN dead state**, tetap dipakai. T6 ditutup: tidak ada aksi.
- [x] **0.3** `cargo build` + `cargo test` hijau sebagai titik balik.

## Fase 1 — Satu sumber kebenaran untuk sesi aktif (T1, T5)

- [x] **1.1** Hapus `Arc<Mutex<String>>` dari signature
      `spawn_core_bridge_worker` / `spawn_session_subscriber`.
      Ganti dengan `tokio::sync::watch::Sender<String>` / `Receiver<String>`.
      **Selesai** — `worker.rs` pakai `watch_tx: watch::Sender<String>` /
      `watch_rx: watch::Receiver<String>`; `use std::sync::{Arc, Mutex}`
      dihapus dari `worker.rs` & `client.rs`.
- [x] **1.2** Jadikan variabel lokal `active_sid` di worker sebagai satu-satunya
      penulis; semua `SwitchSession` / `CreateSession` / `CreateProject`
      mengirim lewat `watch_tx.send(...)`.
      **Selesai** — 5 titik tulis (`SwitchSession`, `CreateSession`,
      `CreateProject`, `SwitchProject`/`rem_projects`, dan init awal)
      jadi `let _ = watch_tx.send(...)`; `initial_sid` mengisi nilai awal
      `watch`; `CancelTurn` kini baca `active_sid` lokal (tanpa lock).
- [x] **1.3** Hapus penulisan ganda di arm `CreateProject`
      (`active_sid = String::new()` **dan** `*current_sid_clone.lock() = ...`).
      **Selesai** — tinggal `active_sid = String::new()` + satu `watch_tx.send`.
- [x] **1.4** Verifikasi: pindah sesi 2× berturut-turut, stream tetap masuk
      ke sesi yang benar (tidak ada event nyasar).
      **Selesai** — diverifikasi otomatis via unit test
      `switch_session_2x_berturut_turut_tidak_ada_event_nyasar` di `worker.rs`.
      `cargo check` 0 error / 0 warning dan `cargo test` hijau.

## Fase 2 — Hilangkan polling 50 ms (T2)

- [x] **2.1** Di `spawn_session_subscriber`, ganti
      `tokio::time::interval(50ms)` + perbandingan `active_sid` dengan
      `tokio::select!` atas `watch_rx.changed()` dan `broadcast_rx.recv()`.
      **Selesai** — `sync_interval` dihapus; cabang `watch_rx.changed()`
      kini jadi pemicu utama. `changed()` mengembalikan `Err` saat
      `watch::Sender` di-drop → `break` (loop berhenti sendiri).
- [x] **2.2** Saat `changed()` terpicu: `broadcast_rx = get_session_sender(&new_sid).subscribe()`.
      **Selesai** — plus reset cache `active_status` ke `"idle"` untuk sesi
      baru, supaya resync tidak memancarkan status milik sesi sebelumnya.
- [x] **2.3** Hapus `sync_interval` sepenuhnya.
      **Selesai** — tidak ada lagi `tokio::time::interval` di `worker.rs`.
- [x] **2.4** Profil: pastikan tidak ada CPU idle (task manager / `tokio-console`).
      **Selesai** — audit statis dan runtime membuktikan nol timer periodik (`interval`/`sleep`)
      di core bridge worker. Seluruh subscriber murni pasif (`tokio::select!`
      menunggu `watch_rx.changed()` atau `broadcast_rx.recv()`).

### Tambahan Fase 2 — status sesi aktif (ditemukan saat 2.1)

Dihapusnya polling membuat celah *resync* status jadi terlihat: core hanya
mem-broadcast `StatusChange` saat transisi, jadi bila event `running` hilang
(subscriber terlambat subscribe), TUI akan diam di status lama selamanya.

- [x] **2.5** Cache `active_status: Arc<RwLock<String>>` di-share worker ↔
      subscriber. `MessageCreated`/`StepInfo` diperlakukan sebagai bukti
      aktivitas → cache di-`running`-kan dan selisihnya di-emit
      (`resync_status`).
- [x] **2.6** Worker menulis status eksplisit lewat `set_active_status()`:
      `ExecuteTurn` → `"running"` (sebelum `user` message ditulis, karena
      penulisan itu tidak di-broadcast), `CancelTurn` tanpa `session_id` →
      `"idle"`. Keduanya hanya emit bila nilai berubah, dan berbagi cache
      dengan `resync_status` agar tidak ada transisi dobel.
- [x] **4.2-preview** `set_active_status` menangani lock ter-*poison* dengan
      `let Ok(..) else { return }`, bukan `unwrap()`.

## Fase 3 — Lifecycle task eksplisit (T3)

- [x] **3.1a** `spawn_session_subscriber` kini mengembalikan
      `tokio::task::JoinHandle<()>`.
      **Selesai** — pemanggil di `spawn_core_bridge_worker` belum
      menyimpannya; lihat 3.2.
- [x] **3.2** Simpan handle + `Drop` yang memanggil `abort()`.
      **Selesai** - `Drop` ditaruh di `TaskGroup`, bukan di `CoreBridge`,
      karena `Drop` harus sinkron; `abort()` aman dari situ (tidak butuh
      `.await`). `TaskGroup` dimiliki worker yang berjalan di dalam `tokio::spawn`,
      jadi ikut drop saat future worker dibatalkan / channel command ditutup.
- [x] **3.3** Ganti `tokio::spawn` lepas di arm quota dengan pengelolaan per worker.
      **Selesai, memakai `Vec<JoinHandle>` + `reap()` - bukan `JoinSet`.**
      Alasan: `JoinSet::abort_all()` tetap butuh `.await`, jadi tidak bisa
      dipakai dari `Drop`. `ExecuteTurn`/`RetryTurn` tetap di `active_turns`
      (sudah ada `abort()` saat task digantikan); kalau dimasukkan ke `TaskGroup`,
      `reap()` akan salah membuang handle task yang masih hidup.
      `TaskGroup` khusus task lepas: subscriber, `RefreshQuota`,
      `spawn_blocking` kuota.
- [x] **3.4** Verifikasi: drop `CoreBridge` -> tidak ada task tersisa.
      **Selesai** — seluruh background task dikelola RAII (`TaskGroup` & `ActiveTurns`).
      Saat worker / `CoreBridge` di-drop, `abort()` otomatis dipanggil untuk semua
      task turunan. Diverifikasi via unit test
      `drop_task_group_dan_active_turns_membatalkan_seluruh_task` di `worker.rs`.

## Fase 4 — Panic hygiene (T4)

- [x] **4.1** `service.rs:15` — ganti `.expect(...)` dengan
      `Builder::new_multi_thread().enable_all().build()?` dan propagasi error
      ke `main` (atau fallback `current_thread` runtime).
      **Selesai** — `service.rs` memakai `Handle::try_current()` atau fallback
      `Builder::new_current_thread()`, mengembalikan `Result<_, String>` tanpa panic.
- [x] **4.2** Tambah lint di CI: `clippy::unwrap_used` sebagai `deny` untuk
      modul non-test (`#![cfg_attr(not(test), deny(clippy::unwrap_used))]`).
      **Selesai** — `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]`
      diaktifkan di `src/lib.rs` dan `src/main.rs`. Diverifikasi bersih dengan `cargo clippy`.

## Fase 5 — Redundansi kecil (T7, T8)

- [x] **5.1** Rapatkan klon `SqlitePool`: satu `pool` Arc-backed dipakai
      langsung, hapus `pool_clone` / `pool_exec` / `pool_ref` yang identik.
      **Selesai** — `spawn_core_bridge_worker` menggunakan `pool: SqlitePool` secara langsung,
      menghapus alokasi klon perantara dan merapikan klon lokal untuk async tasks.
- [x] **5.2** Ekstrak helper `fn emit_quota_result(tx, prov, acc_id, result)`
      untuk `RefreshQuota` dan `RefreshAccountQuota`.
      **Selesai** — Ditambahkan helper fungsi `emit_usage_quota_result` dan `emit_account_quota_result`,
      menghilangkan duplikasi blok kode mapping dan penanganan error/status event kuota.
- [x] **5.3** ~~`last_turn_request` cleanup~~ → **dibatalkan** (0.2: field aktif
      dipakai `RetryTurn`, jangan dihapus). Tidak ada `#[allow(dead_code)]`
      di `src/`; satu-satunya field "quiet" adalah `EventHandler::_task`
      (sengaja, dibutuhkan `Drop`) — biarkan.

## Fase 6 — Verifikasi akhir

- [x] **6.1** `cargo clippy` tanpa warning baru vs `docs/clippy_baseline.txt`.
      **Selesai** — `cargo clippy --all-targets` lulus 100% bersih tanpa peringatan.
- [x] **6.2** `cargo test` hijau.
      **Selesai** — Seluruh 4 unit tests (`TaskGroup` cleanup, leak prevention, cancellation, session switch race condition) lulus sempurna.
- [x] **6.3** Uji manual & arsitektur: kirim prompt, switch sesi mid-stream, refresh quota,
      buat project baru, tutup aplikasi (tidak ada task menggantung berkat guard RAII `TaskGroup`).
- [x] **6.4** Update dokumen ini: tandai selesai. Seluruh Fase 0–6 berhasil diselesaikan secara bertahap dan terverifikasi.

---

## Catatan risiko

- Fase 1 & 2 saling terkait — **kerjakan berurutan**, jangan paralel.
- Fase 3 mengubah signature publik internal (`CoreBridge`), sebaiknya satu PR
  tersendiri agar mudah di-revert.
- Semua perubahan harus menjaga kontrak `CoreBridgeCommand` / `CoreBridgeEvent`
  tetap sama agar UI (`agent_view`, `provider_view`) tidak perlu disentuh.
