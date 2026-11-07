use crate::agent_view::states::AgentWorkspaceState;
use crate::app::types::Action;
use crate::core_bridge::CoreBridgeCommand;
use tokio::sync::mpsc::UnboundedSender;

/// Input editing: typing, paste, deletion, newline, cursor movement, clear.
pub(crate) fn handle_action(
    agent: &mut AgentWorkspaceState,
    action: Action,
    _bridge_cmd_tx: Option<&UnboundedSender<CoreBridgeCommand>>,
) -> Option<Option<String>> {
    let status = match action {
        Action::AgentInsertChar(c) => {
            agent.insert_char(c);
            agent.check_and_update_tag_query();
            None
        }
        Action::AgentPaste(text) => {
            agent.paste(&text);
            None
        }
        Action::AgentBackspace => {
            agent.backspace();
            None
        }
        Action::AgentInputDelete => {
            agent.delete_forward();
            None
        }
        Action::AgentInputNewline => {
            agent.insert_newline();
            None
        }
        Action::AgentInputCursorLeft => {
            agent.cursor_left();
            None
        }
        Action::AgentInputCursorRight => {
            agent.cursor_right();
            None
        }
        Action::AgentInputCursorUp => {
            agent.cursor_up();
            None
        }
        Action::AgentInputCursorDown => {
            agent.cursor_down();
            None
        }
        Action::AgentInputCursorHome => {
            agent.cursor_home();
            None
        }
        Action::AgentInputCursorEnd => {
            agent.cursor_end();
            None
        }
        Action::AgentInputDeleteWordBackward => {
            agent.delete_word_backward();
            None
        }
        Action::AgentInputClearToStart => {
            agent.clear_to_start();
            None
        }
        Action::AgentInputClearToEnd => {
            agent.clear_to_end();
            None
        }
        Action::AgentClearInput => {
            agent.clear_input();
            agent.close_tag_query();
            None
        }
        _ => return None,
    };

    Some(status)
}
