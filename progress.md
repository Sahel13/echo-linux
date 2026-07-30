# Echo Linux progress

## Current state

- Last completed feature: E2E-001
- Next eligible feature: RELEASE-001
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

### 2026-07-30 — PASTE-001 — X11 paste backend manually verified

- Agent/session: Codex with user-provided manual acceptance evidence
- Commit: this commit (`PASTE-001: record manual verification`)
- What changed: Marked the completed X11 paste feature passing; no application code changed in this session.
- Verification commands: `scripts/bootstrap.sh`; `scripts/check.sh`; user run: `cargo test --locked paste::tests::live_x11_pastes_into_the_currently_focused_client -- --ignored --nocapture`.
- Manual acceptance evidence: User confirmed the PASTE-001 matrix works, including insertion into the requested text targets without Echo taking focus, text clipboard restoration, preserving a newer clipboard value, and the actionable clipboard fallback on forced injection failure.
- Known limitations or follow-up: None for PASTE-001.
- Next eligible feature: FLOW-001

### 2026-07-30 — FLOW-001 — hold-to-record controller implemented; manual verification pending

- Agent/session: Codex
- Commit: this commit (`FLOW-001: add dictation transaction controller`)
- What changed: Added the single GTK-main-thread dictation controller with Idle, Recording, Transcribing, and timed Error states. It snapshots settings at press, starts CPAL capture and Groq prewarm on workers, ignores overlapping holds, applies the 300 ms threshold, finalizes/transcribes/pastes through the existing worker seams, and deletes temporary WAVs on completion, cancellation, and failure. The X11 shortcut worker now temporarily grabs Escape during recording and cancels the recording if that grab cannot be acquired, preventing Escape from reaching the focused app.
- Verification commands: `scripts/bootstrap.sh`; initial sandbox `scripts/check.sh` (existing localhost mock-server binds denied); approved baseline `scripts/check.sh`; `cargo fmt`; `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test --locked controller::tests`; final approved `scripts/check.sh`; `git diff --check`; `xwininfo -root -display "$DISPLAY"`; `pactl list short sources`; `timeout 5 target/debug/echo`.
- Manual acceptance evidence: The five controller tests cover successful state transitions, short-hold and Escape cancellation, every failure and empty-speech transition, ignored invalid/overlapping events, and temporary-audio removal for success, empty, cancellation, and failure exits. Full check passed with 45 tests; its three pre-existing live tests remain ignored. Manual X11/microphone acceptance was unavailable: `xwininfo` could not open `:0`, PulseAudio/PipeWire rejected the connection, and Echo reported `Failed to open display`.
- Known limitations or follow-up: Keep `FLOW-001` at `passes: false`. In an accessible X11 session with a working Groq key and microphone, focus a text entry and: (1) tap the shortcut for under 300 ms and confirm no recording, request, or paste; (2) hold it, speak, and release, confirming exactly one recording, request, and paste; (3) press Escape during recording and confirm no request or paste and no Escape effect in the focused app; (4) hold the shortcut while transcribing and confirm it is ignored; (5) test success, empty speech, cancellation, microphone loss, invalid key/network, and forced paste failure, confirming no `/tmp/echo-recording-*.wav` file remains after each. After a microphone-loss error, immediately make another recording to confirm capture is not poisoned.
- Next eligible feature: FLOW-001

### 2026-07-30 — FLOW-001 — zero-frame timing guard and safe diagnostics; manual verification still pending

- Agent/session: Codex
- Commit: not committed — manual verification is required before committing this FLOW-001 change.
- What changed: Added per-transaction, privacy-safe stderr diagnostics limited to transaction ID, controller state, capture-ready count, finalized frame count, transcription-request count, and paste-attempt count. A finalized zero-frame WAV now follows the existing “No speech detected.” error transition before Groq or paste. Paste dispatch now occurs only after the controller has applied the successful-transcript transition, preventing worker dispatch from getting ahead of controller state. No amplitude or other audio-content heuristic was added.
- Verification commands: `pwd`; `scripts/bootstrap.sh`; initial sandbox `scripts/check.sh` (existing localhost mock-server binds denied); approved baseline `scripts/check.sh`; inspected `3a58ced`, `5e37bea`, and `c32222b`; `cargo test --locked audio::tests::live_default_and_selected_microphones_finalize_valid_wavs -- --ignored --nocapture`; `cargo fmt`; `cargo test --locked controller::tests`; `cargo clippy --all-targets -- -D warnings`; final approved `scripts/check.sh`; `git diff --check`.
- Manual acceptance evidence: The active X11 root window and PipeWire microphone sources were accessible, and the rebuilt Echo process was launched on `DISPLAY=:0`. Its diagnostics received no F10 events during the available test window, so neither the silent-hold nor spoken-hold scenario was observed. The live audio test did confirm that the default and selected microphone paths can start and finalize valid WAVs on this host. The seven controller tests cover the new zero-frame no-speech/no-transcription/no-paste branch and paste-after-transition ordering; the full check passed with 47 tests (3 live tests ignored).
- Known limitations or follow-up: Keep `FLOW-001` at `passes: false`. On the active X11 session, launch the rebuilt Echo, focus a normal text field, and perform a 300+ ms silent F10 hold followed by a 300+ ms spoken F10 hold. For the silent transaction, record only the safe diagnostic counters: it must end with `finalized_frames=0`, `transcription_requests=0`, and `paste_attempts=0`, while the UI says “No speech detected.” For the spoken transaction, confirm exactly one request and one paste and that the text field receives the result. If the silent transaction reports nonzero frames, do not treat this guard as a fix; retain `passes: false` and use the counters to investigate the remaining cause without logging audio or transcript data. Separately, injected Ctrl+V maps to image paste in Codex CLI and yields its “no image on clipboard” error; this is a later PASTE-001 compatibility bug and is intentionally not implemented during FLOW-001.
- Next eligible feature: FLOW-001

### 2026-07-30 — FLOW-001 — silent microphone noise is rejected before Groq

- Agent/session: Codex with user-provided X11 manual acceptance evidence
- Commit: pending (`FLOW-001: reject sparse VAD noise`)
- What changed: Replaced the reverted amplitude threshold with the `webrtc-vad` WebRTC/libfvad detector over 20 ms 16 kHz PCM frames. The controller now requires one contiguous 200 ms VAD speech run before it may transcribe; sparse VAD positives from microphone noise enter the existing “No speech detected.” path and remove the temporary WAV without Groq or paste. Added state/counter diagnostics limited to transaction ID, controller state, capture count, frame counts, VAD counts, request count, and paste count. Paste dispatch remains ordered after the controller transition. No API key, authorization header, transcript, path, or audio data is logged.
- Verification commands: `pwd`; `scripts/bootstrap.sh`; sandbox `scripts/check.sh` (the existing localhost mock-server bind is denied); approved baseline and final `scripts/check.sh`; `cargo info webrtc-vad@0.4.0`; `cargo fmt`; `cargo test controller::tests`; `cargo test audio::tests`; `cargo clippy --all-targets -- -D warnings`; `cargo test --locked audio::tests::live_default_and_selected_microphones_finalize_valid_wavs -- --ignored --nocapture`; `git diff --check`.
- Manual acceptance evidence: On the user's active X11 session, a spoken hold completed one request and one paste. A silent 300+ ms hold showed “No speech detected.” and the visible privacy-safe diagnostic reported `capture=1`, `frames=25931`, `speech=5`, `longest=5`, `requests=0`, and `pastes=0`; it inserted no text. This establishes that sparse VAD positives, rather than controller timing or a stale paste, caused the former “Thank you” result. The full suite passed with 49 tests and three explicitly ignored environment-dependent live tests; the separate live microphone finalization test passed.
- Known limitations or follow-up: Keep `FLOW-001` at `passes: false` until its remaining manual acceptance matrix is recorded: under-300 ms tap, Escape cancellation, overlapping hold while transcribing, and temporary-file checks across the documented error paths. The diagnostic line is temporary FLOW-001 verification instrumentation and contains counters only. Separately, Codex CLI maps injected Ctrl+V to image paste and reports “no image on clipboard”; that remains a later PASTE-001 compatibility bug and was not changed here.
- Next eligible feature: FLOW-001

### 2026-07-30 — FLOW-001 — complete hold-to-record transaction manually verified

- Agent/session: Codex with user-provided X11 manual acceptance evidence
- Commit: pending (`FLOW-001: reject sparse VAD noise`)
- What changed: Marked FLOW-001 passing after completing the manual matrix. The final controller uses a 300 ms hold minimum, Escape cancellation, one-transaction state ownership, ordered paste dispatch, and a WebRTC VAD contiguous-run gate that rejects sparse microphone-noise positives before any Groq request or paste.
- Verification commands: `scripts/bootstrap.sh`; approved `scripts/check.sh`; `cargo test controller::tests`; `cargo test audio::tests`; `cargo clippy --all-targets -- -D warnings`; approved `cargo test --locked audio::tests::live_default_and_selected_microphones_finalize_valid_wavs -- --ignored --nocapture`; final approved `scripts/check.sh`; `git diff --check`.
- Manual acceptance evidence: User confirmed all remaining FLOW-001 manual acceptance checks: under-300 ms tap makes no request or paste; spoken hold makes exactly one request and paste; Escape cancels without request, paste, or reaching the focused app; a hold during transcription is ignored; and temporary audio is removed along the documented terminal paths. The final silent hold was independently evidenced by the X11 diagnostic `capture=1 frames=25931 speech=5 longest=5 requests=0 pastes=0` and showed “No speech detected.” without inserting text.
- Known limitations or follow-up: FLOW-001 diagnostics intentionally report counters only and never transcript text, audio, API keys, authorization headers, or audio paths. Codex CLI's Ctrl+V image-paste behavior remains a later PASTE-001 compatibility bug and is not part of this feature.
- Next eligible feature: STYLE-001

### 2026-07-30 — STYLE-001 — transcription controls implemented; active-transaction snapshot verification pending

- Agent/session: Codex
- Commit: this commit (`STYLE-001: add transcription controls`)
- What changed: Added visible, persisted Model, Language, Style, and Custom vocabulary controls. The model list contains both required Groq IDs; the language list contains Auto-detect and all thirteen required languages; and the style list contains Normal and Lower Case. Lower Case now applies Unicode lowercasing to the trimmed Groq response, while Normal leaves it unchanged. The existing controller continues to clone settings at shortcut press, so the selected request/style configuration is retained for that transaction.
- Verification commands: `pwd`; `scripts/bootstrap.sh`; initial sandbox `scripts/check.sh` (the existing localhost mock-server bind is denied); approved baseline and final `scripts/check.sh`; `cargo fmt --check`; `cargo test --locked transcription_control_options_cover_every_required_choice`; `cargo test --locked normal_style_preserves_text_and_lower_case_uses_unicode_lowercase`; `git diff --check`; approved X11 launches with isolated XDG settings, `xwininfo`, `xdotool`, and window-only `ffmpeg` captures.
- Manual acceptance evidence: On the active X11 session, a window-only capture showed the four controls with the documented defaults. X11 interaction selected both models, both styles, custom vocabulary, and each of the fourteen language entries, checking the isolated settings file after every selection. A restart capture visibly reloaded non-default `whisper-large-v3`, Russian, Lower Case, and `Echo Style Test` vocabulary. The full check passed with 51 tests and three explicitly ignored environment-dependent live tests.
- Known limitations or follow-up: Keep `STYLE-001` at `passes: false` until the active-transaction snapshot step is manually observed. In an X11 session with a working microphone and a test Groq key, set Lower Case, start a 300+ ms hold, change Style to Normal while recording, speak, and release; the inserted result must still be lowercased. Repeat starting at Normal and switching to Lower Case, expecting the initial Normal behavior. Change model, language, and vocabulary during separate active recordings and use Groq request telemetry from a test account or an approved TLS-capable capture to confirm every request retains the values selected at shortcut press. Do not record API keys, authorization headers, transcript text, or audio in that evidence.
- Next eligible feature: STYLE-001

### 2026-07-30 — STYLE-001 — active-transaction snapshot removed and feature accepted

- Agent/session: Codex
- Commit: this commit (`STYLE-001: remove transaction settings snapshot`)
- What changed: Removed the controller's transaction-wide settings snapshot at the user's direction. Capture now reads the selected microphone when recording starts, and transcription copies the current settings only when dispatching its worker. The user confirmed that changing a style during recording is unsupported, so no active-transaction snapshot behavior is retained or tested.
- Verification commands: `pwd`; `scripts/bootstrap.sh`; approved baseline `scripts/check.sh`; `rg -n snapshot src/controller.rs`; final `scripts/check.sh`; `git diff --check`.
- Manual acceptance evidence: The prior STYLE-001 X11 verification exercised and restarted every visible model, language, style, and vocabulary control. Per the user's direction, no mid-recording setting-change test is required.
- Known limitations or follow-up: None for STYLE-001. Mid-recording setting changes are unsupported and are not guaranteed to apply consistently to the in-flight dictation.
- Next eligible feature: OVERLAY-001

### 2026-07-30 — OVERLAY-001 — non-interactive X11 dictation overlay implemented

- Agent/session: Codex
- Commit: this commit (`OVERLAY-001: add X11 dictation overlay`)
- What changed: Added a GTK-main-thread overlay driven directly by the dictation controller. Recording uses a red pulse, transcribing uses a faster neutral pulse, and errors use a single ellipsized line before the controller's existing 2.5-second timeout hides the window. The X11 surface is transparent, override-redirect, pointer-transparent, non-focusable, always above, sticky, and excluded from task switchers and pagers. Placement selects the monitor containing the focused X11 window, then falls back to the pointer or first monitor, and positions the overlay at bottom center.
- Verification commands: `pwd`; complete reads of `AGENTS.md`, `docs/PORTING_GUIDE.md`, `progress.md`, and `feature-list.json`; `git log -10 --oneline`; `git status --short --branch`; `scripts/bootstrap.sh`; initial sandbox `scripts/check.sh` (existing localhost mock-server binds denied); approved baseline and final `scripts/check.sh`; `cargo fmt`; `cargo clippy --all-targets -- -D warnings`; `cargo test --locked overlay::tests`; `cargo test --locked controller::tests`; `git diff --check`; approved X11 checks with `xwininfo`, `xprop`, `xrandr`, `xdotool`, a controlled `xmessage` focus target, and overlay-only `ffmpeg` captures.
- Manual acceptance evidence: Non-interactive inspection on X11 at 1920×1200 captured the visible 260×64 bottom-centered recording overlay at `(830,1088)`, showing a red pulse and `Recording…`. A silent release produced the one-line `No speech detected.` error, and the overlay changed from `IsViewable` to `IsUnMapped` after the 2.5-second error interval. While a controlled Xmessage window had focus, its X11 focus ID `39845924` remained unchanged before, during, and after a brief recording cancelled before transcription. Moving the pointer over the visible overlay still reported the underlying Xmessage window, proving pointer pass-through. `xprop` reported input focus false, notification window type, above/skip-taskbar/skip-pager/sticky states, and all-desktops `4294967295`.
- Known limitations or follow-up: The available X11 environment exposed only one `eDP` monitor, so physical multi-monitor placement could not be exercised. The silent recording moved through transcribing too quickly to retain a rendered transcribing capture; focused automated tests verify that the neutral transcribing pulse changes opacity twice as fast as recording and that focused-monitor bounds select offset monitor geometries. Per the user's instruction for this unattended session, these unavailable interactive cases are documented and the feature is marked passing.
- Next eligible feature: HISTORY-001

### 2026-07-30 — HISTORY-001 — persistent word total and memory-only last transcript

- Agent/session: Codex unattended implementation agent
- Commit: this commit (`HISTORY-001: add dictation history`)
- What changed: Added a process-local history state that records each non-empty successful transcription exactly once, counts whitespace-delimited words into the existing persisted `total_words` setting, and retains the exact styled transcript only in memory. Added a History section showing the lifetime total and a Copy last transcript action; persistence runs off the GTK thread and failures remain visible without exposing transcript text.
- Verification commands: `pwd`; complete reads of `AGENTS.md`, `docs/PORTING_GUIDE.md`, `progress.md`, and `feature-list.json`; `git log -10 --oneline`; `git status --short --branch`; `scripts/bootstrap.sh`; sandbox baseline `scripts/check.sh` (existing localhost mock-server binds denied); approved baseline and final `scripts/check.sh`; `cargo fmt`; `cargo test --locked history::tests`; `cargo clippy --all-targets -- -D warnings`; `git diff --check`.
- Manual acceptance evidence: This was an unattended session, so no live spoken transcription or human clipboard inspection was attempted. The three focused tests verify that one successful exact transcript adds its whitespace-delimited words once, that saving and reloading settings retains the total while a fresh process starts with an empty last transcript, and that an empty result changes neither value. Existing controller tests verify short, cancelled, empty, and failed paths never reach the non-empty transcript-success branch. The copy handler passes the retained string unchanged to GTK's text clipboard.
- Known limitations or follow-up: Live end-to-end clipboard inspection and a spoken restart cycle were unavailable without human input. Per the user's instruction for this unattended session, these unavailable interactive checks are documented and HISTORY-001 is marked passing.
- Next eligible feature: UI-001

### 2026-07-30 — UI-001 — compact accessible settings window completed

- Agent/session: Codex unattended implementation agent
- Commit: this commit (`UI-001: complete settings window`)
- What changed: Reorganized the existing controls into the five required API key, Shortcut, Transcription, Input, and General groups inside a vertically scrollable 640-pixel clamp. Added a standard libadwaita header with Echo name, microphone branding, and a short dictation subtitle; a persistent Launch at login switch; a keyboard-accessible Help dialog; and grouped Quit action. Added explicit accessible labels and mnemonics for controls without self-describing button text. The UI contains Groq settings directly and adds no engine picker, tray requirement, local transcription, or other out-of-scope feature. The launch preference only persists here; creating and validating the XDG autostart desktop file remains AUTOSTART-001.
- Verification commands: `pwd`; complete reads of `AGENTS.md`, `docs/PORTING_GUIDE.md`, `progress.md`, and `feature-list.json`; `git log -10 --oneline`; `git status --short --branch`; `scripts/bootstrap.sh`; initial sandbox `scripts/check.sh` (existing localhost mock-server binds denied); approved baseline and final `scripts/check.sh`; `cargo fmt`; `cargo build --locked`; `cargo test --locked tests::settings_window_contract_has_the_required_groups_and_scope`; `cargo clippy --all-targets -- -D warnings`; `git diff --check`; approved X11 inspection with `xwininfo`, `xprop`, `xdotool`, and window-only `ffmpeg` captures.
- Manual acceptance evidence: Unattended inspection on the active 1920×1170 X11 display captured the standard libadwaita Echo/microphone header and both the top and bottom of the vertically scrollable form. The content remained centered and bounded to 640 pixels at the wide tiled size; labels, status text, dropdowns, entries, actions, and the General switch remained readable. Synthetic Tab traversal advanced through the form and automatically scrolled from the API-key editor to the General actions. Alt+H opened a visible 560×342 `About Echo` dialog containing hold/release, Escape cancellation, and X11 guidance. Alt+L focused the Launch at login switch, Space changed the isolated XDG settings document from `false` to `true`, and Alt+Q exited Echo. The focused contract test covers all five required groups, all required control names, help content, and absence of an engine control; the full suite passed 58 tests with three documented environment-dependent tests ignored.
- Known limitations or follow-up: The active tiling window manager forced Echo to 1920×1170 and ignored an X11 request to resize it to 480×680, so a second compact-size capture was unavailable. The scroll/clamp behavior was directly visible at the forced wide size, and the initial missing vertical-expansion defect found by that capture was fixed before final verification. The session's AT-SPI bus rejected connections, so a screen-reader tree could not be inspected; keyboard traversal, live mnemonics, explicit GTK accessible labels, and the focused contract test provide unattended evidence instead. Per the user's instruction for this unattended session, these unavailable interactive cases are documented and UI-001 is marked passing.
- Next eligible feature: AUTOSTART-001

### 2026-07-30 — AUTOSTART-001 — XDG launch-at-login lifecycle completed

- Agent/session: Codex unattended implementation agent
- Commit: this commit (`AUTOSTART-001: manage XDG autostart entry`)
- What changed: Added an XDG autostart backend that atomically writes `autostart/io.github.sahel.Echo.desktop` for the absolute current executable and removes it when disabled. The Launch at login switch now updates the desktop entry and persisted setting together on a worker thread, rolls the entry back if settings persistence fails, and prevents overlapping toggle operations. On startup, an enabled preference validates the entry without rewriting it; a missing, invalid, or different executable path produces an actionable retoggle warning.
- Verification commands: `pwd`; complete reads of `AGENTS.md`, `docs/PORTING_GUIDE.md`, `progress.md`, and `feature-list.json`; `git log -10 --oneline`; `git status --short --branch`; `scripts/bootstrap.sh`; sandbox baseline `scripts/check.sh` (existing localhost mock-server binds denied); approved baseline and final `scripts/check.sh`; `cargo fmt`; `cargo test --locked autostart::tests`; `cargo clippy --all-targets -- -D warnings`; `git diff --check`; `desktop-file-validate` against the generated isolated entry; approved X11 checks using an isolated `XDG_CONFIG_HOME`, `xdotool`, `xwininfo`, and window-only `ffmpeg` captures.
- Manual acceptance evidence: On the active 1920×1200 X11 display, toggling Launch at login on created a desktop-file-validate-clean entry whose quoted `Exec` exactly matched `/home/sahel/code/echo-linux/target/debug/echo`, and persisted `launch_at_login: true`. After exiting Echo, executing that generated command as a simulated login launch produced exactly one Echo process and one visible Echo window. Launching a copied binary from `/tmp` with the original entry retained visibly showed “Echo moved — toggle launch at login off and on to update it,” and inspection confirmed the entry was not silently rewritten. Toggling the setting off removed the desktop file and persisted `false`. Three focused tests independently cover the XDG location, valid entry creation/removal, and moved-binary detection without rewrite; the full suite passed 61 tests with three documented environment-dependent tests ignored.
- Known limitations or follow-up: An actual desktop logout/login cycle was unavailable in this unattended session, so the generated entry was exercised by running its exact `Exec` command in the active X11 session rather than restarting the whole session. Per the user's instruction, this unavailable interactive boundary is documented and AUTOSTART-001 is marked passing.
- Next eligible feature: E2E-001

### 2026-07-30 — E2E-001 — clean-checkout suite and unattended X11 exits verified

- Agent/session: Codex unattended verification agent
- Commit: this commit (`E2E-001: verify end-to-end quality gates`)
- What changed: No application code changed. Marked the end-to-end quality gate passing after consolidating the dated feature evidence, running the harness from a fresh local clone, and exercising privacy-safe X11 cancellation and no-speech transactions. The default microphone was muted only for the controlled no-speech check and restored to its original unmuted state immediately afterward.
- Verification commands: `pwd`; complete reads of `AGENTS.md`, `docs/PORTING_GUIDE.md`, `progress.md`, and `feature-list.json`; `git log -10 --oneline`; `git status --short --branch`; repository `scripts/bootstrap.sh`; sandbox baseline `scripts/check.sh` (existing localhost mock-server binds denied); approved baseline and final `scripts/check.sh`; clean local clone with `git clone --local --no-hardlinks`, followed by `scripts/bootstrap.sh` and approved `scripts/check.sh` inside the clone; approved X11/audio inspection with `xwininfo`, `xrandr`, `pactl`, `xmessage`, `xdotool`, `gapplication`, `pgrep`, and value-free `/tmp/echo-recording-*.wav` checks; source log-call audit with `rg`; `git diff --check`.
- Manual acceptance evidence: The clean clone built from scratch and passed all 61 non-environmental tests; the three ignored tests remain explicitly hardware/service-dependent. On the active 1920×1200 X11 display with an isolated XDG configuration, a disposable Xmessage target retained the same focus ID before and after every transaction. A 100 ms F10 tap reported one capture followed by `shortcut-released-short` and `recording-cancelled`, with zero request/paste counters and no temporary WAV. A 500 ms hold cancelled by Escape likewise preserved focus and left no temporary WAV. After quitting that Echo process, a fresh process successfully reacquired F10; with the default source temporarily muted, a 700 ms hold finalized 10,912 frames with `speech=0`, `longest=0`, `requests=0`, and `pastes=0`, entered the timed error path, returned to Idle, and left no temporary WAV. Quitting left no Echo process, and the microphone was confirmed unmuted again. Earlier dated entries provide the available porting-guide matrix evidence: PASTE-001 records GTK/browser/terminal/rich-editor targets plus empty/non-empty clipboard behavior; HOTKEY-001 records Caps Lock, Num Lock, repeat, and layout-change behavior; AUDIO-002 and FLOW-001 record live PipeWire capture, a spoken request/paste, cancellation, recovery, and terminal-path cleanup; APP-001 and AUTOSTART-001 record reactivation and single-instance relaunch; OVERLAY-001 records focus preservation and the available single-monitor overlay behavior. The source audit found only the fixed ignored-test instruction and the controller's numeric/state diagnostic output; request privacy tests passed, and the isolated settings tree contained no `api_key`, `authorization`, or `transcript` field.
- Known limitations or follow-up: Human speech, live clipboard inspection, physical microphone disconnection, live offline/invalid-key/rate-limit UI paths, dictation while the settings window is hidden, a physical multi-monitor setup, and repeated spoken dictation in a second X11 environment could not be exercised without human input or unavailable hardware/session infrastructure. Only one `eDP` monitor and one active X11 environment were present. Network/error behavior and all controller exit paths are covered by the passing automated suite, but those checks are not claimed as manual evidence. Per the user's explicit unattended-session instruction, these unavailable manual cases are documented and E2E-001 is marked passing.
- Next eligible feature: RELEASE-001
