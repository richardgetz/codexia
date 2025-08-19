# Codexia Agents & Architecture

This document explains how the Codexia codebase works end‑to‑end and how the app orchestrates agents, sessions, and the Codex CLI. It also notes how to use the optional `codex/` folder (if present) as a reference to the Codex core.

## What Codexia Is

Codexia is a desktop GUI for the Codex CLI with multi‑session support. Each UI session runs its own Codex process, streams events in real time, and supports sandbox + approval policies for executing commands and applying patches.

## High‑Level Flow

- UI (React/TypeScript) renders sessions, messages, approvals, and config.
- Tauri (Rust) spawns one Codex process per session in protocol mode and relays JSON events to the UI.
- The UI sends user input and approval decisions back to the Rust backend, which writes protocol messages to the Codex process stdin.

## Repository Layout (App)

- `src/`: React UI (components, hooks, stores, services, types).
- `src-tauri/`: Rust backend (Tauri commands, Codex process manager, filesystem helpers).
- `public/`, `index.html`, `vite.config.ts`: Frontend tooling and assets.

If a top‑level `codex/` folder exists, it vendors the Codex core repo (Rust crates + JS CLI wrapper) only as reference material. Codexia does not build or run that code directly; it shells out to your installed `codex` binary.

## Runtime Architecture

- UI State: Zustand stores hold sessions, messages, approvals, and config.
- IPC: UI calls Tauri commands via `@tauri-apps/api/core` and listens to events via `@tauri-apps/api/event`.
- Backend State: `src-tauri/src/state.rs` keeps a `HashMap<String, CodexClient>` for live sessions.
- Codex Client: `src-tauri/src/codex_client.rs` starts `codex proto`, writes submissions to stdin, parses JSON lines from stdout, and emits per‑session events like `codex-event-{sessionId}`.
- Discovery: `src-tauri/src/utils/codex_discovery.rs` resolves a native `codex` binary (prefers platform binaries; skips Node wrapper scripts).
- Logging: `src-tauri/src/utils/logger.rs` writes debug logs to `/tmp/codexia.log`.

## Session Lifecycle

Tauri commands (see `src-tauri/src/commands.rs`):
- `start_codex_session`: Spawns/records a Codex process for a session ID.
- `send_message`: Sends a `UserInput` submission.
- `approve_execution`: Sends an approval decision for `exec` requests.
- `close_session` / `stop_session`: Gracefully shuts down and cleans up.
- `get_running_sessions`: Returns active session IDs.

Frontend coordination:
- `src/services/sessionManager.ts` ensures a session is running, restarts with new config, and tracks active sessions.
- `src/hooks/useCodexEvents.ts` listens on `codex-event-{sessionId}` and routes events to the conversation store and approval UI.

## Protocol: Submissions and Events

Rust types in `src-tauri/src/protocol.rs` define the wire protocol to the Codex process:
- Submissions (`Submission { id, op }`): `UserInput`, `ExecApproval`, `PatchApproval`, `Interrupt`, `Shutdown`, etc.
- Events (`Event { id, msg }`): `session_configured`, `agent_message`, `agent_message_delta`, `exec_approval_request`, `patch_approval_request`, `task_started/complete`, `turn_complete`, `exec_command_*`, `error`, `shutdown_complete`, etc.

The UI mirrors these types in `src/types/codex.ts` and handles them in `useCodexEvents`.

## Configuration & Policies

- UI type: `src/types/codex.ts` (`CodexConfig`) controls model, provider, approval policy, sandbox mode, working directory, and optional custom args.
- Backend type: `src-tauri/src/protocol.rs` (`CodexConfig`) maps to CLI flags for `codex proto`.
- Policies: Approval policies (`untrusted`, `on-request`, `on-failure`, `never`) and sandbox modes (`read-only`, `workspace-write`, `danger-full-access`) are passed to the Codex process. These mirror Codex CLI behavior.
- Discovery: The backend will prefer a native `codex` binary if present; otherwise returns a helpful error.

## Persistence & Imports

- Session Import: `src-tauri/src/services/session.rs` can parse historical `.jsonl` files under `~/.codex/sessions` and import them as read‑only conversations. It auto‑deletes metadata‑only files (single line, no messages).
- Favorites: The UI allows favoriting imported sessions; data is stored in the frontend store.

## Approvals & Patches

- Execution approvals: When Codex emits an `exec_approval_request`, the UI shows an approval dialog and replies with `approve_execution`.
- Patch approvals: The protocol supports `patch_approval_request` and `PatchApproval` submissions. The UI wiring is present; surfacing a full patch review UI is straightforward to extend.

## Working With the Optional `codex/` Folder

If you see a `codex/` folder at the repository root, it is a local copy of the Codex core for reference and examples:
- `codex/codex-rs/`: Rust workspace with crates like `core`, `exec`, `mcp-server`, `protocol`, `apply-patch`, etc.
- Useful references:
  - Agent orchestration: `codex/codex-rs/core/src/codex.rs`
  - Protocol and planning: `codex/codex-rs/protocol/src/*.rs`
  - Patch tool: `codex/codex-rs/apply-patch/*`
  - Sandbox/policies: `codex/codex-rs/execpolicy/*` and `linux-sandbox/*`
  - MCP types and client: `codex/codex-rs/mcp-types/*`, `mcp-client/*`

Notes:
- Codexia does not compile/run this code; it shells out to your installed `codex` binary.
- Use it to understand the agent’s internal behaviors, event sequencing, and policy enforcement.

## Extending Codexia

- New model/provider: Add fields to `CodexConfig` (TS + Rust), map to CLI args in `codex_client.rs`, and expose toggles in the Config dialog.
- Enhanced approvals: Extend the approval dialog components, handle `patch_approval_request`, and add corresponding Tauri command(s) if needed.
- Filesystem features: `src-tauri/src/filesystem/*` includes helpers (diff, analysis, IO) that you can expose via new commands and UI panels.
- Session UX: `src/stores/*` and `src/components/*` can be extended for richer history, grouping, and debugging.

## Development Tips

- Dev: `bun tauri dev` (frontend + Tauri) and `bun run build` (frontend only).
- Rust checks: `cargo check --manifest-path src-tauri/Cargo.toml`; format with `cargo fmt`.
- Logs: Inspect `/tmp/codexia.log` for backend details (process discovery, stdout parsing, lifecycle).
- Verifying Codex: Use `check_codex_version` (Tauri command) or run `codex -V` to confirm the binary on PATH.

## Mental Model Summary

- One session = one Codex process in protocol mode.
- UI <-> Tauri via commands/events; Tauri <-> Codex via stdin/stdout JSON.
- Policies flow from UI config into the Codex process and govern execution and patch application.
- `codex/` (when present) is a reference copy of Codex core; Codexia treats it as documentation, not as a runtime dependency.

