# Echo Linux progress

## Current state

- Last completed feature: SET-001
- Next eligible feature: KEY-001
- Build status: passing
- Known blockers: KEY-001 needs manual GTK verification on an accessible desktop session.

## Session log

Append entries; do not rewrite earlier entries.

### YYYY-MM-DD — FEATURE-ID — short result

- Agent/session:
- Commit:
- What changed:
- Verification commands:
- Manual acceptance evidence:
- Known limitations or follow-up:
- Next eligible feature:

### 2026-07-29 — INIT-001 — Rust GTK4/libadwaita scaffold verified

- Agent/session: Codex initializer agent
- Commit: this commit (`INIT-001: scaffold Rust GTK application`)
- What changed: Initialized the Git repository; added the locked Rust GTK4/libadwaita binary, development harness scripts, README, MIT license, and required empty `tests/`, `assets/`, and `packaging/` directories.
- Verification commands: `scripts/bootstrap.sh`; `scripts/check.sh`; bounded X11 launch via `scripts/dev.sh` with `xwininfo` and an isolated window capture.
- Manual acceptance evidence: In the active X11 display (`DISPLAY=:0`), one live window titled `Echo` with class `echo` opened. An isolated capture visually showed the Echo heading and “Echo for Linux is ready.” The tiling window manager expanded the 360×180 default window to its tile; no later application behavior was implemented.
- Known limitations or follow-up: This is intentionally only the empty application scaffold. Single-instance, settings, X11-only messaging, and dictation behavior remain for later features.
- Next eligible feature: APP-001

### 2026-07-30 — APP-001 — single-instance hidden settings window lifecycle verified

- Agent/session: Codex
- Commit: this commit (`APP-001: implement single-instance lifecycle`)
- What changed: Retained the application window across activations, presented it when Echo is relaunched, intercepted normal window close to hide it, and added a Quit action wired to the visible Quit button.
- Verification commands: `scripts/bootstrap.sh`; initial and final `scripts/check.sh`; `cargo fmt --check`; `cargo build --locked`; live X11 process/window checks using `pgrep`, `xwininfo`, a standard `WM_DELETE_WINDOW` request, and `gapplication action io.github.sahel.Echo quit`.
- Manual acceptance evidence: On the active X11 display (`DISPLAY=:0`), a second launch left exactly one `echo` process and one viewable Echo window. A normal X11 close request left that process running and changed the same window to `IsUnMapped`; another launch made it `IsViewable` again. The Quit action, bound to the visible Quit button, terminated the process and destroyed its window.
- Known limitations or follow-up: Verification used the available X11 environment and standard X11/application action interfaces; no dictation backend is introduced by this feature.
- Next eligible feature: APP-002

### 2026-07-30 — APP-002 — X11 session gate implemented; manual verification pending

- Agent/session: Codex
- Commit: pending (`APP-002: gate unsupported sessions`)
- What changed: Added GTK display-backend detection at activation. X11 retains the normal ready message; every non-X11 or unavailable display backend shows that Echo requires X11 and that global shortcuts and pasting are disabled. No shortcut or paste backend exists or is initialized by this feature.
- Verification commands: `scripts/bootstrap.sh`; initial `scripts/check.sh`; `cargo fmt --check`; `cargo test --locked`; final `scripts/check.sh`; attempted X11 launch with `target/debug/echo` and inspection with `xwininfo`; attempted local Broadway non-X11 launch with `broadwayd` and `GDK_BACKEND=broadway`.
- Manual acceptance evidence: Automated tests cover X11 and every GTK backend classification (Wayland, Broadway, macOS, Win32, and unavailable display). Manual display verification was unavailable: the sandbox could not open the advertised `DISPLAY=:0`, and Broadway could create its Unix socket but could not bind an inspection port, so its rendered window could not be inspected.
- Known limitations or follow-up: On an accessible Wayland session, launch Echo and verify the window says “Echo for Linux requires an X11 session. Global shortcuts and pasting are disabled.” Confirm that F10 and paste injection have no effect. On an accessible Xorg session, launch Echo and verify the unsupported-session text is absent and the ready text appears. Keep `APP-002` at `passes: false` until both visual checks are recorded.
- Next eligible feature: APP-002

### 2026-07-30 — APP-002 — X11-only session behavior manually verified

- Agent/session: Codex with user-provided manual acceptance evidence
- Commit: pending (`APP-002: record manual verification`)
- What changed: Marked the completed X11 session gate feature passing; no application code changed in this session.
- Verification commands: `scripts/bootstrap.sh`; `scripts/check.sh`.
- Manual acceptance evidence: User confirmed the APP-002 acceptance checks: a non-X11 session shows the X11-required explanation with no shortcut-grab or paste-injection activity, and an X11 session does not show the unsupported-session message.
- Known limitations or follow-up: None for APP-002.
- Next eligible feature: SET-001

### 2026-07-30 — SET-001 — versioned atomic XDG settings verified

- Agent/session: Codex
- Commit: this commit (`SET-001: persist typed XDG settings`)
- What changed: Added typed non-secret settings with every documented default, JSON version `1`, and an XDG configuration path at `echo/settings.json`. Echo loads and atomically rewrites a valid document at startup; failed loads or saves produce a visible error without logging configuration contents. Atomic writes flush a temporary file before replacement and sync the containing directory on Linux.
- Verification commands: `scripts/bootstrap.sh`; baseline `scripts/check.sh`; `cargo fmt`; `cargo clippy --all-targets -- -D warnings`; `cargo test --all-targets --locked`; final `scripts/check.sh`.
- Manual acceptance evidence: No display server, hardware, credential, or interactive control is required for this persistence feature. The seven passing tests exercise all acceptance cases directly: every default, XDG path selection, changed settings reload via a fresh store, an interruption during the real write path before replacement, and inspection that the saved document has neither API-key nor transcript fields.
- Known limitations or follow-up: Settings controls are intentionally deferred to their ordered features. The persisted schema includes the required setting types but no secret value or last transcript field.
- Next eligible feature: KEY-001

### 2026-07-30 — KEY-001 — Secret Service API-key lifecycle implemented; manual verification pending

- Agent/session: Codex
- Commit: this commit (`KEY-001: add Secret Service key storage`)
- What changed: Added a `keyring` Secret Service backend that stores the Groq key at a dedicated Echo service/account pair. The GTK window now has a password entry, saved/not-saved status, and save/remove actions. All Secret Service access runs in a worker thread; the UI never reads a saved key into the entry and shows only generic secure-storage errors.
- Verification commands: `scripts/bootstrap.sh`; baseline `scripts/check.sh`; `cargo test --locked`; `target/debug/deps/echo-e38521ea62a85e8e --ignored secret::tests::real_secret_service_saves_survives_a_fresh_store_replaces_and_removes`; final `scripts/check.sh`.
- Manual acceptance evidence: The dedicated real-service test passed against the active desktop Secret Service, proving save, a fresh-store status lookup, replacement, and removal using a separate `key-001-test-groq-api-key` account. The GTK window was launched on X11, but this harness cannot inject keyboard input or inspect rendered label text, so the UI acceptance steps were not claimed as manual evidence.
- Known limitations or follow-up: Keep `KEY-001` at `passes: false`. On an accessible X11 desktop with the normal keyring running, enter a disposable key in Echo’s password entry and save it; verify the status says a key is saved while the entry remains empty. Quit and relaunch Echo, verify saved status persists, replace it, then remove it. Inspect Echo’s XDG settings file, application logs, process arguments, and environment output to verify the disposable key is absent. Finally stop the desktop Secret Service, launch or refresh Echo, and verify the actionable secure-storage error appears. Report those observations before marking the feature passing.
- Next eligible feature: KEY-001
