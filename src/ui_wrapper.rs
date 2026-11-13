use crate::process_manager::ProcessManager;
use crate::types::ProcessError;
use crate::ui::{AppState, run_terminal_ui};
use std::sync::Arc;

/// Run the terminal UI on every process the manager loaded.
pub async fn run_interactive_ui(manager: ProcessManager) -> Result<(), ProcessError> {
    let manager = Arc::new(manager);
    let state = Arc::new(AppState::new());

    let processes_to_start = manager.process_names().await;
    let procfile_path = manager.procfile_path().await;
    run_terminal_ui(manager, state, processes_to_start, &procfile_path).await
}
