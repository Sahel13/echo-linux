# Echo Linux progress

## Current state

- Last completed feature: INIT-001
- Next eligible feature: APP-001
- Build status: passing
- Known blockers: none

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
