use crate::app::types::Action;
use super::types::CustomLlmTabState;

/// Menangani action yang berkaitan dengan Custom LLM tab.
/// Mengembalikan Some(Option<status_message>) jika action dikenali & ditangani,
/// atau None jika action bukan milik Custom LLM.
pub fn handle_custom_llm_action(state: &mut CustomLlmTabState, action: &Action) -> Option<Option<String>> {
    match action {
        Action::FormChar(c) => {
            state.form_input(*c);
            Some(None)
        }
        Action::FormBackspace => {
            state.form_backspace();
            Some(None)
        }
        Action::FormNextField => {
            state.form_next_field();
            Some(None)
        }
        Action::FormPrevField => {
            state.form_prev_field();
            Some(None)
        }
        Action::FormSubmit => {
            match state.form_submit() {
                Ok(msg) => {
                    tracing::info!("Custom LLM endpoint saved: {msg}");
                    Some(Some(msg))
                }
                Err(err) => {
                    tracing::warn!("Custom LLM form submit error: {err}");
                    Some(Some(format!("Failed: {err}")))
                }
            }
        }
        Action::CancelForm => {
            state.close_form();
            Some(None)
        }
        Action::CustomLlmAdd => {
            state.open_add_form();
            Some(Some("Tambah Custom LLM Endpoint".to_string()))
        }
        Action::CustomLlmEdit => {
            state.open_edit_form();
            Some(Some("Edit Custom LLM Endpoint".to_string()))
        }
        Action::CustomLlmToggle => {
            if let Some(msg) = state.toggle_enabled() {
                tracing::info!("{msg}");
                Some(Some(msg))
            } else {
                Some(None)
            }
        }
        Action::CustomLlmDelete => {
            if state.open_delete_confirm() {
                Some(Some("Konfirmasi hapus endpoint?".to_string()))
            } else {
                Some(None)
            }
        }
        Action::CustomLlmConfirmDelete => {
            let msg = state.confirm_delete();
            Some(msg)
        }
        Action::CustomLlmCancelDelete => {
            state.cancel_delete();
            Some(Some("Penghapusan dibatalkan".to_string()))
        }
        Action::CustomLlmRefresh => {
            state.refresh_models();
            let count = state.models.len();
            Some(Some(format!("Daftar model diperbarui ({count} model)")))
        }
        Action::CustomLlmToggleFocus => {
            state.toggle_focus();
            let msg = match state.focus {
                super::types::CustomLlmFocus::Providers => "Fokus: List Provider (Tekan Tab untuk ganti)",
                super::types::CustomLlmFocus::Models => "Fokus: List Models (Tekan Tab untuk ganti)",
            };
            Some(Some(msg.to_string()))
        }
        _ => None,
    }
}
