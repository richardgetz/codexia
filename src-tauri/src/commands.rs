use crate::protocol::CodexConfig;
use crate::services::{auth, codex, session, slash};
use crate::state::CodexState;
use tauri::{AppHandle, State};

// Re-export types for external use
pub use crate::services::session::Conversation;

#[tauri::command]
pub async fn load_sessions_from_disk() -> Result<Vec<Conversation>, String> {
    session::load_sessions_from_disk().await
}

#[tauri::command]
pub async fn start_codex_session(
    app: AppHandle,
    state: State<'_, CodexState>,
    session_id: String,
    config: CodexConfig,
) -> Result<(), String> {
    codex::start_codex_session(app, state, session_id, config).await
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, CodexState>,
    session_id: String,
    message: String,
) -> Result<(), String> {
    codex::send_message(state, session_id, message).await
}

#[tauri::command]
pub async fn approve_execution(
    state: State<'_, CodexState>,
    session_id: String,
    approval_id: String,
    approved: bool,
) -> Result<(), String> {
    codex::approve_execution(state, session_id, approval_id, approved).await
}

#[tauri::command]
pub async fn stop_session(state: State<'_, CodexState>, session_id: String) -> Result<(), String> {
    codex::stop_session(state, session_id).await
}

#[tauri::command]
pub async fn close_session(state: State<'_, CodexState>, session_id: String) -> Result<(), String> {
    codex::close_session(state, session_id).await
}

#[tauri::command]
pub async fn get_running_sessions(state: State<'_, CodexState>) -> Result<Vec<String>, String> {
    codex::get_running_sessions(state).await
}

#[tauri::command]
pub async fn check_codex_version() -> Result<String, String> {
    codex::check_codex_version().await
}

#[tauri::command]
pub async fn start_chatgpt_login(app: AppHandle, state: State<'_, CodexState>) -> Result<String, String> {
    auth::start_chatgpt_login(app, state).await
}

#[tauri::command]
pub async fn cancel_chatgpt_login(state: State<'_, CodexState>) -> Result<(), String> {
    auth::cancel_chatgpt_login(state).await
}

#[tauri::command]
pub async fn delete_session_file(file_path: String) -> Result<(), String> {
    session::delete_session_file(file_path).await
}

#[tauri::command]
pub async fn get_latest_session_id() -> Result<Option<String>, String> {
    session::get_latest_session_id().await
}

#[tauri::command]
pub async fn get_status_text(state: State<'_, CodexState>, session_id: String) -> Result<String, String> {
    slash::get_status_text(state, session_id).await
}

#[tauri::command]
pub async fn get_repo_diff(working_directory: String) -> Result<String, String> {
    slash::get_repo_diff(working_directory).await
}
