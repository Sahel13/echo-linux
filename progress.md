# Echo Linux progress

## Current state

- Last completed feature: AUDIO-001
- Next eligible feature: AUDIO-002
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

### 2026-07-30 — KEY-001 — API-key storage manually verified

- Agent/session: Codex with user-provided manual acceptance evidence
- Commit: this commit (`KEY-001: record manual verification`)
- What changed: Marked the completed Secret Service API-key feature passing; no application code changed in this session.
- Verification commands: `scripts/bootstrap.sh`; `scripts/check.sh`; value-free inspection of the XDG settings schema, Echo user-journal line count, live Echo process count, and temporary application log size.
- Manual acceptance evidence: User confirmed saving a disposable key reports saved without displaying it, status persists after restart, replacement and removal succeed, and stopping Secret Service displays the actionable storage error. The settings file contains only `version`, documented non-secret settings fields, and no `api_key` or `transcript` field. Echo had zero user-journal lines, no live process after the verification session, and a zero-byte temporary launch log; no key value was printed during inspection.
- Known limitations or follow-up: None for KEY-001.
- Next eligible feature: HOTKEY-001

### 2026-07-30 — HOTKEY-001 — X11 F10 backend implemented; manual verification pending

- Agent/session: Codex
- Commit: this commit (`HOTKEY-001: implement X11 F10 backend`)
- What changed: Added a worker-thread X11 shortcut backend using `x11rb`. It resolves the live F10 keysym mapping, passively grabs unmodified F10 for every Caps Lock/Num Lock state, enables XKB detectable auto-repeat when supported, and otherwise suppresses synthetic release/press repeat pairs. Keyboard-map and modifier-map changes re-resolve and re-grab the binding. Grab conflicts and connection failures leave Echo open with an actionable status. The GTK main thread receives only small typed shortcut events.
- Verification commands: `scripts/bootstrap.sh`; baseline `scripts/check.sh`; `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test --all-targets --locked`; `xwininfo -root -display "$DISPLAY"`.
- Manual acceptance evidence: Automated tests passed for keysym resolution, Caps/Num-lock grab variants, detectable auto-repeat, fallback repeat suppression, and conflict messaging. Manual X11 verification was unavailable: although `DISPLAY=:0` is set, `xwininfo` returned `unable to open display ":0"`, so Echo could not connect to an X server.
- Known limitations or follow-up: Keep `HOTKEY-001` at `passes: false`. In an accessible X11 session, focus another application and hold/release F10; relaunch Echo to inspect its shortcut status and confirm one press and one final release. Repeat with Caps Lock and Num Lock in all four combinations, then hold past keyboard-repeat delay and verify no early release. Change keyboard layouts, repeat the hold/release check, and confirm the re-resolved binding still works. Finally have an independent X11 client passively grab unmodified F10 before launching Echo; confirm Echo stays open and says another application is using F10.
- Next eligible feature: HOTKEY-001

### 2026-07-30 — HOTKEY-001 — standard F10 backend manually verified

- Agent/session: Codex with user-provided manual acceptance evidence
- Commit: this commit (`HOTKEY-001: verify F10 backend`)
- What changed: Reverted the unsuccessful `XF86Favorites` default, restoring the documented unmodified X11 F10 default in both settings and the X11 backend. The user will use a personal configuration override for the ThinkPad-specific key later.
- Verification commands: `scripts/bootstrap.sh`; baseline `scripts/check.sh`; final `scripts/check.sh`.
- Manual acceptance evidence: User confirmed the HOTKEY-001 manual matrix against the standard F10 backend: Echo receives one press and one final release while another application has focus; Caps Lock and Num Lock combinations work; holding through keyboard repeat does not release early; a keyboard-layout change re-resolves the binding; and a forced grab conflict displays an error without exiting Echo.
- Known limitations or follow-up: None for HOTKEY-001. ThinkPad-specific `XF86Favorites` binding is deferred to the user-configurable shortcut feature.
- Next eligible feature: HOTKEY-002

### 2026-07-30 — HOTKEY-002 — custom shortcut capture and live X11 re-grab implemented; manual verification pending

- Agent/session: Codex
- Commit: this commit (`HOTKEY-002: add custom shortcut capture`)
- What changed: Added a Change shortcut control that captures a readable non-modifier key plus Ctrl/Alt/Shift/Super modifiers, cancels on Escape, rejects bare modifiers, and persists an accepted binding. The X11 worker now accepts live update commands, resolves the captured keysym, tries the new passive grab before releasing the old one, and retains the old shortcut on a conflict. Captured X11 key names, including ThinkPad `XF86Favorites`, are accepted and reloaded from settings on restart.
- Verification commands: `scripts/bootstrap.sh`; baseline `scripts/check.sh`; `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test --all-targets --locked`; `xwininfo -root -display "$DISPLAY"`; final `scripts/check.sh`.
- Manual acceptance evidence: The 20 non-ignored automated tests cover capture naming/modifiers, Escape and bare-modifier rejection, ThinkPad `XF86Favorites` conversion, custom modifier masks, settings reload, and the prior X11 grab/repeat behavior. Manual X11 verification was unavailable because `xwininfo` returned `unable to open display ":0"`.
- Known limitations or follow-up: Keep `HOTKEY-002` at `passes: false`. In an accessible X11 session: (1) click Change shortcut, capture a non-modifier such as F9, verify its readable name, restart Echo, and confirm it remains shown and active; (2) click Change shortcut, press Escape, then separately press only Ctrl/Shift/Alt/Super and confirm capture cancels or remains pending without changing the saved binding; (3) capture Ctrl+F9 and verify its global press/release while another application has focus; (4) arrange a passive-grab conflict for a new candidate, attempt to capture it, and verify the old shortcut still works; (5) capture the ThinkPad phone-marked key and verify the emitted `XF86Favorites` binding works globally.
- Next eligible feature: HOTKEY-002

### 2026-07-30 — HOTKEY-002 — custom global shortcut manually verified

- Agent/session: Codex with user-provided manual acceptance evidence
- Commit: this commit (`HOTKEY-002: record manual verification`)
- What changed: Marked the completed custom shortcut feature passing; no application code changed in this session.
- Verification commands: `scripts/bootstrap.sh`; `scripts/check.sh`.
- Manual acceptance evidence: User confirmed every HOTKEY-002 acceptance step: capturing and persisting a readable non-modifier shortcut, Escape cancellation and bare-modifier rejection, a global modified shortcut with press/release behavior, conflict rejection while the old shortcut remains active, and capture/binding of the ThinkPad phone-marked key's actual emitted keysym.
- Known limitations or follow-up: None for HOTKEY-002.
- Next eligible feature: AUDIO-001

### 2026-07-30 — AUDIO-001 — live microphone selector implemented; manual verification pending

- Agent/session: Codex
- Commit: this commit (`AUDIO-001: add microphone selector`)
- What changed: Added CPAL input-device discovery on a worker thread, a System Default-first microphone selector, a manual refresh action plus two-second live refresh, selection persistence, and a visible System Default fallback when the chosen device disappears. Device discovery uses CPAL's Linux input names as the available cross-session selection identifiers.
- Verification commands: `scripts/bootstrap.sh`; baseline `scripts/check.sh`; `cargo fmt`; `cargo test --all-targets --locked`; `cargo clippy --all-targets -- -D warnings`; `xwininfo -root -display "$DISPLAY"`; `pactl list short sources`; `target/debug/echo`; final `scripts/check.sh`; `git diff --check`.
- Manual acceptance evidence: The three focused automated tests cover System Default ordering, retention of a connected selected input, and fallback when that input disappears. Manual verification was unavailable: `DISPLAY=:0` cannot be opened, Echo reports `Failed to open display`, and PulseAudio rejects the sandbox connection. No accessible microphone or rendered selector was available to inspect.
- Known limitations or follow-up: Keep `AUDIO-001` at `passes: false`. In an accessible X11 desktop with at least two input sources, launch Echo and verify System Default plus each connected microphone appears in Input. Select a non-default device, quit/relaunch, and verify the selected label persists. Disconnect that device and wait up to two seconds (or click Refresh microphones); verify the selector changes to System Default and says the selected microphone disappeared. Reconnect an input and verify it appears in the list within two seconds without restarting Echo.
- Next eligible feature: AUDIO-001

### 2026-07-30 — AUDIO-001 — microphone selector manually verified

- Agent/session: Codex with user-provided manual acceptance evidence
- Commit: this commit (`AUDIO-001: record manual verification`)
- What changed: Marked the completed microphone input-selection feature passing; no application code changed in this session.
- Verification commands: `scripts/bootstrap.sh`; `scripts/check.sh`.
- Manual acceptance evidence: User confirmed that the Input selector lists System Default and connected microphones; a selected microphone persists after restart; disconnecting it returns the selector to System Default with a clear UI update; and reconnecting a microphone refreshes the list without restarting Echo.
- Known limitations or follow-up: None for AUDIO-001.
- Next eligible feature: AUDIO-002

### 2026-07-30 — AUDIO-002 — asynchronous WAV capture backend implemented; manual verification pending

- Agent/session: Codex
- Commit: this commit (`AUDIO-002: add WAV capture backend`)
- What changed: Added worker-owned CPAL capture for the system default or selected microphone. Callbacks only convert, downmix, resample, and copy samples; finalization writes 16 kHz mono 16-bit PCM WAV data on the capture worker. Device-stream errors are surfaced as a clean capture failure, and every new capture gets independent state. Added the `hound` WAV dependency and focused conversion, resampling, metadata, recovery, and opt-in live-device tests.
- Verification commands: `scripts/bootstrap.sh`; baseline `scripts/check.sh`; `cargo fmt`; `cargo test --locked`; `scripts/check.sh`; `git diff --check`; `pactl list short sources`; `timeout 5s target/debug/echo`; `cargo test --locked audio::tests::live_default_and_selected_microphones_finalize_valid_wavs -- --ignored`.
- Manual acceptance evidence: Automated tests pass for mono integer conversion, stereo float downmixing, 48 kHz resampling, 16 kHz/mono/16-bit PCM WAV headers, and fresh capture state after simulated device loss. Live verification was unavailable: PulseAudio rejected the sandbox connection, `pactl` reported no accessible sources, the ignored live-device test found no connected microphone, and GTK could not open `DISPLAY=:0`.
- Known limitations or follow-up: Keep `AUDIO-002` at `passes: false`. On an accessible X11 desktop with a default input and a second microphone, run `cargo test --locked audio::tests::live_default_and_selected_microphones_finalize_valid_wavs -- --ignored` and inspect the resulting checks for both capture paths. Then use the later FLOW-001 recording control to verify the settings window stays responsive during capture, unplug an active microphone, confirm a clean failure, and immediately start a new recording successfully.
- Next eligible feature: AUDIO-002

### 2026-07-30 — AUDIO-002 — verified PipeWire capture; background microphone refresh made silent

- Agent/session: Codex with user-provided manual acceptance evidence
- Commit: this commit (`AUDIO-002: quiet microphone refresh status`)
- What changed: The periodic microphone refresh still runs every two seconds, but no longer replaces the current status with “Loading microphones…”. The initial load remains labelled and completed refreshes, selection changes, and errors remain visible.
- Verification commands: `scripts/bootstrap.sh`; baseline `scripts/check.sh`; `cargo fmt`; `scripts/check.sh`; `git diff --check`. User run: `cargo test --locked audio::tests::live_default_and_selected_microphones_finalize_valid_wavs -- --ignored`.
- Manual acceptance evidence: User ran the ignored live-device test successfully on their PipeWire system. It captured the system default and a specifically selected microphone and verified each finalized WAV is 16 kHz, mono, and 16-bit PCM. ALSA compatibility-layer diagnostics were emitted but the test passed.
- Known limitations or follow-up: Keep `AUDIO-002` at `passes: false` until the settings window is manually confirmed responsive during an active recording and a live microphone disconnect is confirmed to terminate capture cleanly and permit a fresh recording. Also visually confirm the periodic refresh no longer flashes “Loading microphones…” while the settings window is open.
- Next eligible feature: AUDIO-002

### 2026-07-30 — AUDIO-002 — manually accepted complete

- Agent/session: Codex with user-provided manual acceptance decision
- Commit: this commit (`AUDIO-002: mark capture verified`)
- What changed: Marked the completed microphone capture and WAV-finalization feature passing; no application code changed in this session.
- Verification commands: `scripts/bootstrap.sh`; `scripts/check.sh`; user run: `cargo test --locked audio::tests::live_default_and_selected_microphones_finalize_valid_wavs -- --ignored`.
- Manual acceptance evidence: User verified live PipeWire-backed capture from the system default and a selected microphone; the hardware test passed and validated 16 kHz mono 16-bit PCM WAV output. User explicitly accepted the feature as complete, with any later issue to be handled as a bug report.
- Known limitations or follow-up: The active-recording settings-responsiveness and physical device-loss scenarios will be covered by the later controller workflow and revisited if a bug is reported.
- Next eligible feature: GROQ-001

### 2026-07-30 — GROQ-001 — Groq multipart transcription client verified

- Agent/session: Codex
- Commit: pending (`GROQ-001: add Groq transcription client`)
- What changed: Added the worker-thread-only Groq client using the documented transcription endpoint and a multipart WAV request. It includes the configured model, optional language, trimmed vocabulary plus the selected style exemplar, and JSON response format; successful transcript text is trimmed. A best-effort HEAD prewarm is intentionally independent from the later POST request.
- Verification commands: `scripts/bootstrap.sh`; baseline `scripts/check.sh`; `cargo fmt`; `cargo test --all-targets --locked`; final `scripts/check.sh`; `git diff --check`.
- Manual acceptance evidence: No manual acceptance is required for this feature. Four local mock-server tests verified the endpoint path and method, bearer authorization, WAV multipart part, model, language, prompt, response format, Auto-detect language omission, whitespace trimming, and recovery from a dropped prewarm connection. The mock server ran only on localhost and made no Groq request.
- Known limitations or follow-up: Error mapping and secret-safe error-response handling are deferred to GROQ-002. The client is not yet connected to the dictation controller; FLOW-001 will invoke it from a worker thread.
- Next eligible feature: GROQ-002

### 2026-07-30 — GROQ-002 — Groq failure mapping verified

- Agent/session: Codex
- Commit: pending (`GROQ-002: map transcription failures`)
- What changed: Added concise mappings for Groq HTTP responses and transport failures, including missing keys, authentication, payload size, rate limits, server responses, timeouts, offline connections, and DNS/connect/TLS failures. Groq `error.message` is trimmed and bounded to 80 characters, while malformed success responses remain generic. The client continues to issue one request with no automatic retry and emits no logs.
- Verification commands: `scripts/bootstrap.sh`; baseline `scripts/check.sh`; `cargo fmt`; `cargo fmt --check`; `cargo test --locked groq::tests`; `cargo clippy --all-targets -- -D warnings`; final `scripts/check.sh`; `git diff --check`; source log-call audit with `rg`.
- Manual acceptance evidence: No display server, credential, or hardware is required. Local loopback mock-server tests cover missing-key, 401/403, 413, 429, 5xx, other HTTP, malformed success, bounded `error.message`, timeout, no retry, and response/request privacy. The source audit found no application logging calls, so the Groq client cannot emit API keys, authorization headers, WAV data, or transcript text.
- Known limitations or follow-up: The deterministic network tests cover the offline and DNS/connect/TLS classification messages; a live-network failure is not needed and no real Groq request was made. The client remains controller-unwired until FLOW-001.
- Next eligible feature: PASTE-001

### 2026-07-30 — PASTE-001 — X11 clipboard and XTEST paste backend implemented; manual verification pending

- Agent/session: Codex
- Commit: this commit (`PASTE-001: add X11 paste backend`)
- What changed: Added the narrow X11 `PasteBackend` seam. Its worker snapshots readable text clipboard content, owns and serves the transcript as text, injects `Ctrl+V` through XTEST without presenting or focusing Echo, waits for the target, and restores only the prior readable text if no other owner replaced the clipboard. It keeps serving the resulting text clipboard while Echo remains open; injection or X11 setup failure deliberately leaves the transcript on the clipboard for manual paste.
- Verification commands: `scripts/bootstrap.sh`; initial sandbox `scripts/check.sh` (mock-server loopback bind denied); approved `scripts/check.sh`; `cargo fmt`; `cargo test --locked paste::tests`; `cargo clippy --all-targets -- -D warnings`; final approved `scripts/check.sh`; `git diff --check`; attempted approved `xwininfo -root -display "$DISPLAY"` (no response; terminated).
- Manual acceptance evidence: The three focused tests cover text-only restoration, protection against overwriting a newer clipboard owner, and the actionable injection-failure result. The required real X11 matrix could not run: this environment's X11 root-window query did not respond, so no GTK entry, browser, terminal, or rich-text target could be exercised.
- Known limitations or follow-up: Keep `PASTE-001` at `passes: false`. In an accessible Xorg session, run `cargo test --locked paste::tests::live_x11_pastes_into_the_currently_focused_client -- --ignored --nocapture`, focus each GTK entry, browser field, terminal, and rich-text editor during its three-second delay, and verify the fixed `Echo paste check` text arrives without Echo taking focus. Repeat with a known text clipboard value and confirm it is restored. During the short paste delay, replace the clipboard from another application and confirm its newer value remains. Finally disable or deny XTEST and confirm the transcript remains on the clipboard with “Couldn't paste — transcript is on the clipboard.”
- Next eligible feature: PASTE-001
