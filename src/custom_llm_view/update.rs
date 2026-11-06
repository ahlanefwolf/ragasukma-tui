use super::types::CustomLlmTabState;
use crate::app::types::Action;

/// Handles actions related to the Custom LLM tab.
/// Returns Some(Option<status_message>) if action was recognized & handled,
/// or None if action does not belong to Custom LLM.
pub fn handle_custom_llm_action(
    state: &mut CustomLlmTabState,
    action: &Action,
) -> Option<Option<String>> {
    match action {
        Action::FormChar(c) if state.is_form_open => {
            state.form_input(*c);
            Some(None)
        }
        Action::FormPaste(text) if state.is_form_open => {
            state.form_paste(text);
            Some(None)
        }
        Action::FormBackspace if state.is_form_open => {
            state.form_backspace();
            Some(None)
        }
        Action::FormNextField if state.is_form_open => {
            state.form_next_field();
            Some(None)
        }
        Action::FormPrevField if state.is_form_open => {
            state.form_prev_field();
            Some(None)
        }
        Action::FormSubmit if state.is_form_open => match state.form_submit() {
            Ok(msg) => {
                tracing::info!("Custom LLM endpoint saved: {msg}");
                Some(Some(msg))
            }
            Err(err) => {
                tracing::warn!("Custom LLM form submit error: {err}");
                Some(Some(format!("Failed: {err}")))
            }
        },
        Action::CancelForm if state.is_form_open => {
            state.close_form();
            Some(None)
        }
        Action::CustomLlmAdd => {
            state.open_add_form();
            Some(Some("Add Custom LLM Endpoint".to_string()))
        }
        Action::CustomLlmEdit => {
            if state.selected_account().is_some() {
                state.open_edit_form();
                Some(Some("Edit Custom LLM Endpoint".to_string()))
            } else {
                Some(Some("No provider selected".to_string()))
            }
        }
        Action::CustomLlmToggle => {
            let msg = state.toggle_enabled();
            Some(msg)
        }
        Action::CustomLlmDelete => {
            if state.open_delete_confirm() {
                Some(Some("Confirm delete endpoint?".to_string()))
            } else {
                Some(Some("No provider selected to delete".to_string()))
            }
        }
        Action::CustomLlmConfirmDelete => {
            let msg = state.confirm_delete();
            Some(msg)
        }
        Action::CustomLlmCancelDelete => {
            state.cancel_delete();
            Some(Some("Deletion canceled".to_string()))
        }
        Action::CustomLlmRefresh => {
            state.refresh_models();
            let count = state.models.len();
            Some(Some(format!("Model list refreshed ({count} models)")))
        }
        Action::CustomLlmToggleFocus => {
            state.toggle_focus();
            let msg = match state.focus {
                super::types::CustomLlmFocus::Providers => {
                    "Focus: Provider List (Press Tab to switch)"
                }
                super::types::CustomLlmFocus::Models => "Focus: Models List (Press Tab to switch)",
            };
            Some(Some(msg.to_string()))
        }
        _ => None,
    }
}
