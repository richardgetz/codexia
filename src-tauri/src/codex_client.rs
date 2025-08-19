use anyhow::Result;
use serde_json;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::protocol::{
    CodexConfig, Event, InputItem, Op, Submission
};
use crate::utils::logger::log_to_file;
use crate::utils::codex_discovery::discover_codex_command;


pub struct CodexClient {
    #[allow(dead_code)]
    app: AppHandle,
    session_id: String,
    process: Option<Child>,
    stdin_tx: Option<mpsc::UnboundedSender<String>>,
    #[allow(dead_code)]
    pub(crate) config: CodexConfig,
}

impl CodexClient {
    pub async fn new(app: &AppHandle, session_id: String, config: CodexConfig) -> Result<Self> {
        log_to_file(&format!("Creating CodexClient for session: {}", session_id));
        
        // Build codex command based on configuration
        let (command, args): (String, Vec<String>) = if let Some(configured_path) = &config.codex_path {
            (configured_path.clone(), vec![])
        } else if let Some(path) = discover_codex_command() {
            (path.to_string_lossy().to_string(), vec![])
        } else {
            return Err(anyhow::anyhow!("Could not find codex executable"));
        };

        let mut cmd = Command::new(&command);
        if !args.is_empty() {
            cmd.args(&args);
        }
        cmd.arg("proto");
        
        // Use -c configuration parameter format (codex proto only supports -c configuration)
        if config.use_oss {
            cmd.arg("-c").arg("model_provider=oss");
        }
        
        if !config.model.is_empty() {
            cmd.arg("-c").arg(format!("model={}", config.model));
        }
        
        if !config.approval_policy.is_empty() {
            cmd.arg("-c").arg(format!("approval_policy={}", config.approval_policy));
        }
        
        if !config.sandbox_mode.is_empty() {
            let sandbox_config = match config.sandbox_mode.as_str() {
                "read-only" => "sandbox_mode=read-only".to_string(),
                "workspace-write" => "sandbox_mode=workspace-write".to_string(), 
                "danger-full-access" => "sandbox_mode=danger-full-access".to_string(),
                _ => "sandbox_mode=workspace-write".to_string(),
            };
            cmd.arg("-c").arg(sandbox_config);
        }
        
        // Set working directory for the process
        if !config.working_directory.is_empty() {
            cmd.current_dir(&config.working_directory);
        }

        // Point Codex to ~/.codex for auth/config (matches codex CLI default)
        // and, if present, surface OPENAI_API_KEY from auth.json to the child.
        // This allows users who logged in via `codex login` or edited auth.json
        // to have the same credentials picked up by the spawned process.
        if let Ok(home) = std::env::var("HOME") {
            let mut codex_home = PathBuf::from(home);
            codex_home.push(".codex");
            cmd.env("CODEX_HOME", &codex_home);

            // Try to read ~/.codex/auth.json and propagate OPENAI_API_KEY if set.
            let auth_path = codex_home.join("auth.json");
            if auth_path.exists() {
                match fs::read_to_string(&auth_path) {
                    Ok(contents) => {
                        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&contents) {
                            if let Some(key) = json_val
                                .get("OPENAI_API_KEY")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                            {
                                cmd.env("OPENAI_API_KEY", key);
                                log_to_file("Propagated OPENAI_API_KEY from ~/.codex/auth.json");
                            }
                        }
                    }
                    Err(e) => {
                        log_to_file(&format!(
                            "Could not read ~/.codex/auth.json: {}",
                            e
                        ));
                    }
                }
            }
        }

        // Add custom arguments
        if let Some(custom_args) = &config.custom_args {
            for arg in custom_args {
                cmd.arg(arg);
            }
        }
        
        // Print the command to be executed for debugging
        log_to_file(&format!("Starting codex with command: {:?}", cmd));
        
        let mut process = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = process.stdin.take().expect("Failed to open stdin");
        let stdout = process.stdout.take().expect("Failed to open stdout");

        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<String>();

        // Handle stdin writing
        let mut stdin_writer = stdin;
        tokio::spawn(async move {
            while let Some(line) = stdin_rx.recv().await {
                if let Err(e) = stdin_writer.write_all(line.as_bytes()).await {
                    log_to_file(&format!("Failed to write to codex stdin: {}", e));
                    break;
                }
                if let Err(e) = stdin_writer.write_all(b"\n").await {
                    log_to_file(&format!("Failed to write newline to codex stdin: {}", e));
                    break;
                }
                if let Err(e) = stdin_writer.flush().await {
                    log_to_file(&format!("Failed to flush codex stdin: {}", e));
                    break;
                }
            }
            log_to_file("Stdin writer task terminated");
        });

        // Handle stdout reading
        let app_clone = app.clone();
        let session_id_clone = session_id.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            log_to_file(&format!("Starting stdout reader for session: {}", session_id_clone));
            
            while let Ok(Some(line)) = lines.next_line().await {
                log_to_file(&format!("Received line from codex: {}", line));
                if let Ok(event) = serde_json::from_str::<Event>(&line) {
                    log_to_file(&format!("Parsed event: {:?}", event));
                    // Send event to frontend
                    if let Err(e) = app_clone.emit(&format!("codex-event-{}", session_id_clone), &event) {
                        log_to_file(&format!("Failed to emit event: {}", e));
                    }
                } else {
                    log_to_file(&format!("Failed to parse codex event: {}", line));
                }
            }
            log_to_file(&format!("Stdout reader terminated for session: {}", session_id_clone));
        });

        let client = Self {
            app: app.clone(),
            session_id,
            process: Some(process),
            stdin_tx: Some(stdin_tx),
            config: config.clone(),
        };

        Ok(client)
    }

    pub fn config(&self) -> CodexConfig {
        self.config.clone()
    }


    async fn send_submission(&self, submission: Submission) -> Result<()> {
        if let Some(stdin_tx) = &self.stdin_tx {
            let json = serde_json::to_string(&submission)?;
            stdin_tx.send(json)?;
        }
        Ok(())
    }

    pub async fn send_user_input(&self, message: String) -> Result<()> {
        let submission = Submission {
            id: Uuid::new_v4().to_string(),
            op: Op::UserInput {
                items: vec![InputItem::Text { text: message }],
            },
        };

        self.send_submission(submission).await
    }

    pub async fn send_exec_approval(&self, approval_id: String, approved: bool) -> Result<()> {
        let decision = if approved { "allow" } else { "deny" }.to_string();
        
        let submission = Submission {
            id: Uuid::new_v4().to_string(),
            op: Op::ExecApproval {
                id: approval_id,
                decision,
            },
        };

        self.send_submission(submission).await
    }

    #[allow(dead_code)]
    pub async fn send_patch_approval(&self, approval_id: String, approved: bool) -> Result<()> {
        let decision = if approved { "allow" } else { "deny" }.to_string();
        
        let submission = Submission {
            id: Uuid::new_v4().to_string(),
            op: Op::PatchApproval {
                id: approval_id,
                decision,
            },
        };

        self.send_submission(submission).await
    }

    #[allow(dead_code)]
    pub async fn interrupt(&self) -> Result<()> {
        let submission = Submission {
            id: Uuid::new_v4().to_string(),
            op: Op::Interrupt,
        };

        self.send_submission(submission).await
    }

    pub async fn close_session(&mut self) -> Result<()> {
        log_to_file(&format!("Closing session: {}", self.session_id));
        
        // Send shutdown command
        let submission = Submission {
            id: Uuid::new_v4().to_string(),
            op: Op::Shutdown,
        };
        
        if let Err(e) = self.send_submission(submission).await {
            log_to_file(&format!("Failed to send shutdown command: {}", e));
        }

        // Close stdin channel
        if let Some(stdin_tx) = self.stdin_tx.take() {
            drop(stdin_tx);
        }

        // Terminate process
        if let Some(mut process) = self.process.take() {
            if let Err(e) = process.kill().await {
                log_to_file(&format!("Failed to kill codex process: {}", e));
            }
        }

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.close_session().await
    }

    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.process.is_some() && self.stdin_tx.is_some()
    }
}
