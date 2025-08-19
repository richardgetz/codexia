use std::fs;
use std::path::PathBuf;

use crate::state::CodexState;
use serde_json::Value;
use base64::Engine;
use tauri::State;

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

fn codex_home() -> Option<PathBuf> {
    if let Ok(v) = std::env::var("CODEX_HOME") {
        if !v.is_empty() {
            return Some(PathBuf::from(v));
        }
    }
    home_dir().map(|h| h.join(".codex"))
}

fn read_auth_json() -> Option<Value> {
    let path = codex_home()?.join("auth.json");
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&text).ok()
}

fn decode_jwt_email_and_plan(id_token: &str) -> (Option<String>, Option<String>) {
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() < 2 { return (None, None) }
    let payload_b64 = parts[1];
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload_b64);
    if let Ok(bytes) = decoded {
        if let Ok(s) = String::from_utf8(bytes) {
            if let Ok(p) = serde_json::from_str::<serde_json::Value>(&s) {
                let email = p.get("email").and_then(|v| v.as_str()).map(|s| s.to_string());
                let plan = p.get("https://api.openai.com/auth")
                    .and_then(|v| v.get("chatgpt_plan_type"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                return (email, plan);
            }
        }
    }
    (None, None)
}

fn home_relative(path: &str) -> String {
    if let Some(home) = home_dir() {
        let home_str = home.to_string_lossy().to_string();
        if path.starts_with(&home_str) {
            return format!("~{}", &path[home_str.len()..]);
        }
    }
    path.to_string()
}

pub async fn get_status_text(state: State<'_, CodexState>, session_id: String) -> Result<String, String> {
    let sessions = state.sessions.lock().await;
    let client = sessions.get(&session_id).ok_or_else(|| "Session not found".to_string())?;
    let cfg = client.config();

    let mut out = String::new();
    out.push_str("/status\n\n");
    // 📂 Workspace
    out.push_str("📂 Workspace\n");
    let cwd_disp = if cfg.working_directory.is_empty() { String::from("~") } else { home_relative(&cfg.working_directory) };
    out.push_str(&format!("  • Path: {}\n", cwd_disp));
    out.push_str(&format!("  • Approval Mode: {}\n", cfg.approval_policy));
    out.push_str(&format!("  • Sandbox: {}\n\n", cfg.sandbox_mode));

    // 👤 Account info from ~/.codex/auth.json
    if let Some(auth) = read_auth_json() {
        if auth.get("tokens").is_some() {
            out.push_str("👤 Account\n");
            out.push_str("  • Signed in with ChatGPT\n");
            if let Some(id_token) = auth.get("tokens").and_then(|t| t.get("id_token")).and_then(|v| v.as_str()) {
                let (email, plan) = decode_jwt_email_and_plan(id_token);
                if let Some(e) = email { out.push_str(&format!("  • Login: {}\n", e)); }
                match auth.get("OPENAI_API_KEY").and_then(|v| v.as_str()) {
                    Some(k) if !k.is_empty() => out.push_str("  • Using API key. Run codex login to use ChatGPT plan\n"),
                    _ => {
                        let p = plan.unwrap_or_else(|| "Unknown".to_string());
                        let mut ch = p.chars();
                        let title = match ch.next() { Some(f) => f.to_uppercase().collect::<String>() + ch.as_str(), None => p };
                        out.push_str(&format!("  • Plan: {}\n", title));
                    }
                }
                out.push_str("\n");
            }
        }
    }

    // 🧠 Model
    out.push_str("🧠 Model\n");
    out.push_str(&format!("  • Name: {}\n", cfg.model));
    out.push_str(&format!("  • Provider: {}\n\n", cfg.provider));

    // 📊 Token Usage (not tracked; show zeros)
    out.push_str("📊 Token Usage\n");
    out.push_str(&format!("  • Session ID: {}\n", session_id));
    out.push_str("  • Input: 0\n");
    out.push_str("  • Output: 0\n");
    out.push_str("  • Total: 0\n\n");

    Ok(out)
}

pub async fn get_repo_diff(working_directory: String) -> Result<String, String> {
    use tokio::process::Command;
    use std::process::Stdio;
    let cwd = if working_directory.is_empty() { std::env::current_dir().unwrap_or_default() } else { PathBuf::from(&working_directory) };

    // Check in repo
    let status = Command::new("git").args(["rev-parse","--is-inside-work-tree"]).current_dir(&cwd).stdout(Stdio::null()).stderr(Stdio::null()).status().await;
    if !matches!(status, Ok(s) if s.success()) { return Ok("`/diff` — _not inside a git repository_".to_string()); }

    // tracked diff
    let tracked_out = Command::new("git").args(["diff","--color"]).current_dir(&cwd).stdout(Stdio::piped()).stderr(Stdio::null()).output().await;
    let tracked = match tracked_out {
        Ok(o) if o.status.success() || o.status.code() == Some(1) => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => String::new(),
    };
    // untracked list
    let untracked_out = Command::new("git").args(["ls-files","--others","--exclude-standard"]).current_dir(&cwd).stdout(Stdio::piped()).stderr(Stdio::null()).output().await;
    let untracked_list = match untracked_out { Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(), _ => String::new() };

    let null_path = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let mut untracked_diff = String::new();
    for file in untracked_list.lines().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let d_out = Command::new("git").args(["diff","--color","--no-index","--", null_path, file]).current_dir(&cwd).stdout(Stdio::piped()).stderr(Stdio::null()).output().await;
        let d = match d_out { Ok(o) if o.status.success() || o.status.code() == Some(1) => String::from_utf8_lossy(&o.stdout).into_owned(), _ => String::new() };
        untracked_diff.push_str(&d);
    }

    let combined = format!("{tracked}{untracked_diff}");
    if combined.trim().is_empty() {
        Ok("/diff\n\nNo changes detected.".to_string())
    } else {
        Ok(format!("/diff\n\n{}", combined))
    }
}
