# Echo for Linux: Porting Guide

## 1. Product definition

Echo for Linux is an X11 desktop dictation app:

> Hold F10, speak, release F10, and the transcript appears in the focused text
> field as quickly as possible.

The Linux app is a new Rust repository. It does not share source code or build
configuration with the macOS app. The macOS repository is a behavioral
reference only.

### First-release scope

- Rust, GTK4, and libadwaita.
- X11 sessions only. Detect Wayland and show a clear unsupported-session
  message instead of partially working.
- Groq transcription only.
- Hold F10 by default. The shortcut is user-configurable.
- A compact settings window; no tray icon.
- A downloadable `x86_64-unknown-linux-gnu` tarball.
- The host provides GTK4, libadwaita, X11, and a Secret Service implementation.

### Required feature parity

- `whisper-large-v3-turbo` and `whisper-large-v3`.
- Auto-detect plus the language list in the macOS app.
- Normal and Lower Case styles.
- Custom vocabulary.
- System-default or selected microphone.
- Hold-to-record and release-to-transcribe.
- Ignore holds shorter than 300 ms.
- Escape cancels an active recording.
- Recording, transcribing, and error overlay.
- Paste into the previously focused application.
- Copy last transcript.
- Lifetime dictated-word count.
- Secure Groq API-key storage.
- Short, useful errors for microphone, network, API, and paste failures.

### Explicit non-goals

- Local transcription.
- Wayland support in the first release.
- A system tray or status icon.
- Flatpak, AppImage, Debian, or RPM packages.
- Streaming partial transcripts.
- LLM cleanup or additional writing styles.
- Context awareness, transcript history, analytics, or account management.

## 2. Decisions and assumptions

| Area | Decision |
| --- | --- |
| Display server | X11 first; Wayland is a later backend |
| Shortcut | Physical F10 with no modifiers by default; configurable |
| Wayland permissions | One-time XDG portal authorization when Wayland is implemented |
| UI | Compact GTK4/libadwaita window |
| Distribution | Downloadable tarball containing the binary, README, license, and icons |
| Transcription | Groq only, retaining all Groq-side macOS features |
| Repository | New repository |
| Initial CPU target | x86-64 Linux using glibc |
| Compatibility baseline | Ubuntu 22.04 LTS: GTK 4.6 and libadwaita 1.1 APIs |

“F10” means the X11 `F10` keysym. Some ThinkPad firmware/Fn-lock settings may
expose the physical key as an `XF86` phone keysym instead. The shortcut-capture
UI must accept whichever keysym X11 actually emits, so the user can bind the
physical key without knowing its name.

The tarball is not a fully static application. GTK and libadwaita are dynamic
system libraries. Build the release on Ubuntu 22.04 LTS and do not use GTK or
libadwaita APIs newer than its GTK 4.6/libadwaita 1.1 packages. Document the
minimum runtime libraries and do not claim universal Linux compatibility.

## 3. Behavior contract

### Startup

1. Start as a normal single-instance GTK application.
2. If the session is not X11, open the settings window with an explanation that
   this release supports X11 only. Do not install a shortcut or attempt input
   injection.
3. Load settings and the API-key status.
4. Attempt to grab the configured shortcut.
5. If the key is unavailable, keep the window usable and show an actionable
   error beside the shortcut control.
6. If no API key exists, present the API-key setup page.
7. Closing the window hides it while background dictation remains active.
   “Quit” actually exits.

With no tray icon, users reopen the hidden window by launching Echo again. The
single-instance application activates the existing process and presents its
window.

### Dictation transaction

The controller owns exactly one state:

```text
Idle -> Recording -> Transcribing -> Idle
                    \-> Error -> Idle
```

1. Shortcut press while Idle starts recording, prewarms Groq, and shows the red
   recording overlay.
2. Repeated key-down events while Recording do nothing.
3. Escape while Recording deletes the recording and returns to Idle.
4. Shortcut release before 300 ms deletes the recording and returns to Idle.
5. A valid release finalizes the WAV and changes the overlay to transcribing.
6. A successful, non-empty transcript is styled, counted, remembered for the
   current process, and inserted at the caret.
7. An empty transcript shows “No speech detected.”
8. Failures show a short message for 2.5 seconds, then return to Idle.
9. Temporary audio is deleted after success, failure, or cancellation.
10. A new dictation cannot begin until the prior transaction is fully reset.

The selected model, language, style, vocabulary, and microphone are snapshotted
when recording begins. Changing settings mid-transaction must not split one
dictation across configurations.

### Shortcut editing

- The control says “Press a key…” while capturing.
- Escape cancels capture.
- Accept a non-modifier key with optional modifiers.
- Reject bare modifiers and Escape.
- Try the X11 grab before saving.
- If the grab fails, retain the old working shortcut and explain the conflict.
- The default is unmodified `F10`.

While recording, temporarily grab Escape so it cancels Echo rather than acting
in the focused application. Release that grab on every exit path.

### Paste semantics

The first X11 implementation uses the clipboard plus XTEST-generated `Ctrl+V`:

1. Snapshot the current text clipboard when readable.
2. Set the transcript as clipboard text.
3. Generate `Ctrl+V` in the application that retained focus.
4. Wait briefly for the target to consume it.
5. Restore the prior text clipboard only if no other process changed the
   clipboard in the meantime.
6. If injection fails, leave the transcript on the clipboard and show:
   “Couldn't paste — transcript is on the clipboard.”

Do not activate or focus Echo during dictation. The overlay must not accept
focus or pointer input. Full preservation of arbitrary non-text MIME clipboard
payloads is not a first-release requirement.

### Settings

Persist non-secret settings under the XDG configuration directory. Use one
small, versioned settings file with atomic replacement. Defaults:

| Setting | Default |
| --- | --- |
| Shortcut | F10, no modifiers |
| Model | `whisper-large-v3-turbo` |
| Language | `en` |
| Style | Normal |
| Microphone | System default |
| Vocabulary | Empty |
| Total words | 0 |

The last transcript is memory-only. Store the Groq key through the desktop
Secret Service, never in the settings file, logs, command line, or environment.
The UI supports save, replace, and remove without displaying a saved key.

### Groq request

- Endpoint:
  `https://api.groq.com/openai/v1/audio/transcriptions`
- Multipart fields: `file`, `model`, optional `language`, optional `prompt`,
  and `response_format=json`.
- Trim surrounding whitespace from the returned `text`.
- Start a best-effort connection prewarm when recording begins.
- Never retry automatically: retries can duplicate cost and add surprising
  latency.
- Map authentication, rate-limit, payload-size, server, timeout, DNS, and
  offline failures to concise messages.

The prompt is the trimmed vocabulary followed by the selected style exemplar,
matching the macOS behavior. Lower Case also lowercases the returned transcript
after transcription.

The exact style definitions are:

| UI name | Prompt exemplar | Post-processing |
| --- | --- | --- |
| Normal | `The following is a professional transcript with proper capitalization, punctuation, and complete sentences. The meeting starts at 3pm, the budget is $12,500, and we are in room 204.` | None |
| Lower Case | `here's a casual transcript with no capitalization and relaxed punctuation just lowercase text. i'll grab 2 coffees and meet you at 5` | Unicode lowercase |

The language choices are Auto-detect (empty code), English (`en`), Spanish
(`es`), French (`fr`), German (`de`), Italian (`it`), Portuguese (`pt`), Dutch
(`nl`), Hindi (`hi`), Arabic (`ar`), Chinese (`zh`), Japanese (`ja`), Korean
(`ko`), and Russian (`ru`).

Use these user-facing failure mappings:

| Cause | Message |
| --- | --- |
| Missing key | `No API key — add one in Settings` |
| HTTP 401/403 | `Invalid API key — check Settings` |
| HTTP 413 | `Recording too large for Groq` |
| HTTP 429 | `Rate limited by Groq — try again shortly` |
| HTTP 5xx | `Groq server error (HTTP N)` |
| Other HTTP | `Groq request failed (HTTP N)` |
| Unreadable success response | `Unreadable response from Groq` |
| Offline/connection lost | `No internet connection` |
| Timeout | `Request timed out` |
| DNS/connect/TLS | `Can't reach Groq` |
| Other network failure | `Network error — try again` |

If the response has a non-empty `error.message`, prefer its first 80 characters
over the generic HTTP mapping. Never include an authorization value or response
body in logs.

Groq currently accepts WAV and downsamples to 16 kHz mono. Record a 16 kHz,
mono, 16-bit PCM WAV. If the device cannot provide that format, capture a
supported native format, downmix, and resample in-process before upload. Do not
add FFmpeg or GStreamer solely for encoding.

## 4. Architecture

Keep one binary crate. Do not create a workspace or plugin system.

```text
src/
  main.rs                 GTK application entry point
  app.rs                  startup, activation, and shutdown
  controller.rs           dictation state machine
  settings.rs             typed settings and XDG persistence
  secret.rs               Groq key storage
  groq.rs                 request construction and error mapping
  audio.rs                devices, capture, conversion, and WAV finalization
  shortcut.rs             backend-neutral shortcut interface
  shortcut/x11.rs         X11 grab and press/release events
  paste.rs                backend-neutral insertion interface
  paste/x11.rs            X11 clipboard and XTEST paste
  ui/settings_window.rs
  ui/overlay.rs
  ui/help.rs
```

This is a responsibility map, not a requirement to create empty files early.
Split a file only when implementing the responsibility that belongs there.

### Main-loop and worker boundary

- GTK and UI state stay on the GLib main thread.
- The X11 event source forwards small typed events to the controller.
- Audio callbacks only copy/convert samples into owned recording state. They
  never update GTK or perform network work.
- Network and file finalization run asynchronously off the GTK thread.
- Results return to the controller through a small channel.
- The controller alone decides state transitions and overlay visibility.

Avoid a second state model in the UI. Widgets render the controller and settings
state.

### Suggested dependencies

Choose current compatible releases when initializing the repository; commit the
lockfile. Prefer these narrow responsibilities:

- `gtk4`, `libadwaita`, `glib`: application and UI.
- `x11rb`: X11 connection, passive grabs, and XTEST.
- `cpal`: microphone discovery and capture.
- `hound`: PCM WAV writing.
- A small pure-Rust resampler only if real devices prove it necessary.
- `reqwest`, `serde`: HTTPS multipart request and JSON response.
- `tokio` or GLib futures: one async runtime, not both as competing app models.
- `keyring` with a Secret Service backend: API-key storage.
- `directories`: XDG paths.
- `serde_json` or `toml`: choose one settings format.
- `thiserror`: errors that cross module boundaries.
- `tracing`: structured logs with secret and transcript values excluded.

Before adopting a dependency, confirm that it is maintained, supports the
minimum Rust version, and does not pull in a second GUI or media framework.
Do not wrap every dependency behind an interface. Only `ShortcutBackend` and
`PasteBackend` need platform seams for the later Wayland port.

### X11 shortcut details

- Resolve the configured keysym against the active XKB keyboard map.
- Use a passive key grab on the root window.
- Subscribe to press, release, mapping-change, and connection-error events.
- Normalize Caps Lock and Num Lock modifier variants so lock state does not
  break activation.
- Enable detectable auto-repeat when available; otherwise suppress synthetic
  release/press repeat pairs. Holding the key must produce one press and one
  final release.
- Re-resolve and re-grab after a keyboard-map change.
- Report `BadAccess` as a shortcut conflict rather than terminating.

### X11 overlay details

GTK4 does not provide portable absolute window placement. For this X11-only
release, use the X11 surface handle to apply the required window-manager hints
and place the overlay near the bottom center of the active monitor.

The overlay is:

- undecorated and transparent;
- always above ordinary windows;
- absent from task switchers and pagers;
- non-focusable and pointer-transparent;
- visible across desktops when the window manager supports it;
- a red pulse while recording, a neutral faster pulse while transcribing, and
  one-line text while showing an error.

Treat placement and “all desktops” as best effort across X11 window managers.
Focus preservation is mandatory.

### Future Wayland seam

Do not implement Wayland in the first release, but keep two narrow traits:

```rust
trait ShortcutBackend {
    // start, stop, update binding, and emit pressed/released
}

trait PasteBackend {
    // insert text or report that it remains on the clipboard
}
```

The Wayland shortcut backend will use XDG Desktop Portal Global Shortcuts,
whose Activated and Deactivated signals preserve hold behavior. Text injection
will use a persistent, user-authorized Remote Desktop portal keyboard session
to send `Ctrl+V`. Store and rotate portal restore tokens as specified by the
portal. Do not add compositor-specific protocols to the first Wayland attempt.

## 5. UI specification

Use one compact adaptive window with these groups:

1. API key: saved/not saved status and Change/Remove action.
2. Shortcut: current binding and Change action.
3. Transcription: model, language, style, and custom vocabulary.
4. Input: microphone.
5. General: copy last transcript, word count, Help, and Quit.

Keep labels and explanations short. Hide engine selection because Groq is the
only engine. The title bar uses the Echo icon and normal libadwaita controls.
Do not recreate macOS menu styling.

The API-key editor uses a password entry. Loading the page reports only whether
a key exists; it does not put the stored key into the widget.

## 6. Repository and harness

The initializer agent creates this minimum repository state:

```text
AGENTS.md
Cargo.toml
Cargo.lock
README.md
LICENSE
feature-list.json
progress.md
scripts/bootstrap.sh
scripts/check.sh
scripts/dev.sh
src/
tests/
assets/
packaging/
```

- `bootstrap.sh` checks required system libraries and fetches/builds Rust
  dependencies. It is safe to run repeatedly.
- `check.sh` runs formatting check, Clippy with warnings denied, unit tests, and
  any non-interactive integration tests.
- `dev.sh` builds and launches Echo with useful logs.
- `README.md` gives exact build, run, runtime-dependency, and X11 instructions.
- `AGENTS.md` contains the session procedure in `AGENT_INSTRUCTIONS.md`.
- Copy `feature-list.json` and initialize `progress.md` from the templates in
  this guide directory.

The initializer makes one clean commit after the scaffold builds and the empty
GTK application launches.

## 7. Sequential implementation

`feature-list.json` is the source of truth. Features are ordered by dependency.
An implementation agent:

1. Reads `AGENTS.md`, `progress.md`, `feature-list.json`, and recent git log.
2. Runs `scripts/bootstrap.sh` and `scripts/check.sh`.
3. Fixes a broken baseline before starting new work.
4. Selects the first failing feature whose dependencies pass.
5. Implements only that feature.
6. Runs its acceptance steps and the full check script.
7. Changes only that feature's `passes` field to `true`.
8. Appends evidence and remaining risks to `progress.md`.
9. Commits the clean state with the feature ID in the message.

Agents must not rewrite feature descriptions, weaken acceptance steps, or mark
manual behavior passing based only on unit tests. If a feature cannot be
verified in the current environment, leave it failing and document what is
needed.

## 8. Testing strategy

### Automated

- Settings defaults, migrations, and atomic persistence.
- Style exemplars and lowercase post-processing.
- Vocabulary/language request fields.
- Groq response decoding and error mapping using a local mock HTTP server.
- Controller transitions including short hold, cancellation, empty transcript,
  overlapping input, and every failure exit.
- Sample conversion/downmix/resampling and valid WAV headers.
- Word counting.
- Shortcut normalization and auto-repeat filtering with recorded X11 events.

Never call the real Groq API in the default test suite.

### Manual X11 acceptance

Test on at least one GNOME Xorg session and one second X11 environment before
release. For each:

- Plain GTK text field.
- Firefox or Chromium address bar and web text area.
- Terminal.
- LibreOffice Writer or another rich editor.
- Empty and non-empty clipboard.
- Caps Lock and Num Lock on and off.
- Keyboard-layout change.
- Microphone disconnect during recording.
- Network offline, invalid key, and rate-limit response.
- Window closed/hidden while dictating.
- Multi-monitor overlay placement.
- Re-launch activates the existing process.

Record the environment and result in `progress.md`. A feature is not complete
when its essential end-to-end path has not been exercised.

## 9. Release

The first release archive is:

```text
echo-linux-x86_64-VERSION.tar.gz
  echo
  README.txt
  LICENSE
  share/icons/hicolor/.../echo.png
```

Build in a clean Ubuntu 22.04 LTS container. Run the test suite there, inspect
linked libraries with `ldd`, and include the exact runtime package list in
`README.txt`. Generate a SHA-256 checksum beside the archive.

The archive does not install files or configure session startup automatically.
Users extract it and place `echo` somewhere on `PATH`. Users who want Echo to
start with their session configure that through their desktop environment,
window manager, or distribution-specific startup mechanism.

## 10. Completion criteria

The port is ready only when:

- every entry in `feature-list.json` passes;
- `scripts/check.sh` succeeds from a clean checkout;
- all manual X11 acceptance cases have recorded evidence;
- no secret or transcript text appears in logs;
- temporary recordings are removed on every tested exit path;
- the release tarball runs on the documented baseline without a Rust toolchain;
- README limitations say X11-only and list dynamic runtime dependencies;
- a fresh agent can build, run, test, and choose the next task using repository
  files alone.

## References

- [Existing Echo macOS source](../../README.md)
- [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)
- [Groq speech-to-text documentation](https://console.groq.com/docs/speech-to-text)
- [XDG Global Shortcuts portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html)
- [XDG Remote Desktop portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html)
- [PipeWire Rust bindings](https://pipewire.pages.freedesktop.org/pipewire-rs/pipewire/index.html)
