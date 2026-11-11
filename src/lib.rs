// Fase 4.2 refactor_plan: jalur panic di kode non-test dilarang lewat lint,
// bukan lewat konvensi. `unwrap_used` menangkap `Option`/`Result::unwrap`;
// `expect_used` menangkap varian yang membawa pesan. Keduanya `deny` agar
// build gagal, bukan sekadar warning yang bisa terlewat.
// `cfg_attr(not(test), ...)` membuat helper tes tetap boleh memakai `unwrap`.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used)
)]

pub mod agent_view;
pub mod app;
pub mod assets;
pub mod core_bridge;
pub mod custom_llm_view;
pub mod event;
pub mod logging;
pub mod provider_view;
pub use provider_view as provider_views;
pub use provider_view::oauth_listener;
pub mod terminal;
pub mod theme;
pub mod ui;
