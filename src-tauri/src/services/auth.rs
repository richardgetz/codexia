use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::state::CodexState;
use crate::utils::codex_discovery::discover_codex_command;
use crate::utils::logger::log_to_file;

fn default_codex_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".codex"))
}

pub async fn start_chatgpt_login(app: AppHandle, state: State<'_, CodexState>) -> Result<String, String> {
    let codex_home = default_codex_home().ok_or_else(|| "Could not determine HOME directory".to_string())?;

    let codex_path = discover_codex_command()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not find codex executable".to_string())?;

    // Spawn `codex login` and capture stderr (auth URL is printed to stderr)
    let mut cmd = Command::new(codex_path);
    cmd.arg("login");
    cmd.env("CODEX_HOME", &codex_home);
    cmd.kill_on_drop(true);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start codex login: {}", e))?;
    // Take I/O handles before storing child for cancellation
    // child.stderr/stdout are already piped; take() to move them out
    let mut child_for_pipes = child;
    let stderr = child_for_pipes.stderr.take();
    let stdout = child_for_pipes.stdout.take();

    // Store child so we can cancel later
    let child_arc = state.login_child.clone();
    {
        let mut guard = child_arc.lock().await;
        *guard = Some(child_for_pipes);
    }

    let app_clone = app.clone();

    // Parse the auth URL from stderr output and emit it once discovered
    if let Some(stderr) = stderr {
        tauri::async_runtime::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut last_url: Option<String> = None;
            while let Ok(Some(line)) = lines.next_line().await {
                // Look for a URL on the line
                if let Some(pos) = line.find("http") {
                    let candidate = line[pos..].trim().to_string();
                    if candidate.starts_with("http://") || candidate.starts_with("https://") {
                        last_url = Some(candidate.clone());
                        let _ = app_clone.emit("auth-login-url", &serde_json::json!({"url": candidate}));
                    }
                }
            }
            if let Some(url) = last_url {
                log_to_file(&format!("Login URL: {}", url));
            }
        });
    }

    // Also listen to stdout just to drain it (avoid blocking)
    if let Some(stdout) = stdout {
        tauri::async_runtime::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(_)) = lines.next_line().await {}
        });
    }

    // Wait for process to finish in background; emit completion status
    let app_clone2 = app.clone();
    let child_arc_for_wait = child_arc.clone();
    tauri::async_runtime::spawn(async move {
        // fetch and take the child from shared state so we can await it
        let mut child_opt: Option<tokio::process::Child> = {
            let mut g = child_arc_for_wait.lock().await;
            g.take()
        };
        if let Some(mut child) = child_opt.take() {
            match child.wait().await {
                Ok(status) if status.success() => {
                    let _ = app_clone2.emit("auth-login-complete", &serde_json::json!({"status":"ok"}));
                }
                Ok(status) => {
                    let _ = app_clone2.emit("auth-login-error", &serde_json::json!({"error": format!("exit status {}", status)}));
                }
                Err(e) => {
                    let _ = app_clone2.emit("auth-login-error", &serde_json::json!({"error": e.to_string()}));
                }
            }
        }
    });

    // We don't have the URL synchronously; return a helpful message.
    Ok("Starting login; watch for auth-login-url event".to_string())
}

pub async fn cancel_chatgpt_login(state: State<'_, CodexState>) -> Result<(), String> {
    let mut guard = state.login_child.lock().await;
    if let Some(child) = guard.as_mut() {
        if let Err(e) = child.kill().await {
            return Err(format!("Failed to cancel login: {}", e));
        }
    }
    *guard = None;
    Ok(())
}
