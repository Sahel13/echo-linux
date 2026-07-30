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
