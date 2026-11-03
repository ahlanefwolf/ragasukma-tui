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
                    Some(Some(format!("Gagal: {err}")))
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
        _ => None,
    }
}
