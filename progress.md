# Echo Linux progress

## Current state

- Last completed feature: INIT-001
- Next eligible feature: APP-001
- Build status: passing
- Known blockers: APP-002 requires manual verification in accessible X11 and non-X11 graphical sessions.

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
