mod codex_client;
mod commands;
mod config;
mod filesystem;
mod protocol;
mod services;
mod state;
mod utils;

use commands::{
    approve_execution, cancel_chatgpt_login, check_codex_version, close_session,
    get_latest_session_id, get_repo_diff, get_running_sessions, get_status_text,
    load_sessions_from_disk, send_message, start_chatgpt_login, start_codex_session,
    stop_session, delete_session_file,
};
use config::{get_project_name, read_codex_config, read_mcp_servers, add_mcp_server, delete_mcp_server};
use filesystem::{
    directory_ops::{get_default_directories, read_directory},
    file_analysis::calculate_file_tokens,
    file_io::{read_file, write_file},
    file_parsers::{
        csv::read_csv_content, pdf::read_pdf_content, xlsx::read_xlsx_content,
    },
    git_diff::get_git_file_diff,
};
use state::CodexState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging with better configuration for both dev and prod
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(CodexState::new())
        .invoke_handler(tauri::generate_handler![
            start_codex_session,
            send_message,
            approve_execution,
            stop_session,
            close_session,
            get_running_sessions,
            load_sessions_from_disk,
            delete_session_file,
            get_latest_session_id,
            get_status_text,
            get_repo_diff,
            check_codex_version,
            start_chatgpt_login,
            cancel_chatgpt_login,
            read_directory,
            get_default_directories,
            calculate_file_tokens,
            read_file,
            write_file,
            read_pdf_content,
            read_csv_content,
            read_xlsx_content,
            get_git_file_diff,
            read_codex_config,
            get_project_name,
            read_mcp_servers,
            add_mcp_server,
            delete_mcp_server,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
