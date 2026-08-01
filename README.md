<p align="center">
  <img src="assets/echo-logo.svg" width="460" alt="Echo logo and wordmark">
</p>

# Echo for Linux

Echo is a minimal desktop dictation app. Hold a shortcut, speak, release it, and
Echo transcribes the recording and pastes the result into the focused text
field.

This repository is the Linux port of the [macOS Echo
app](https://github.com/sabeelash/Echo). It is a
native Rust/GTK application designed for X11 desktop sessions.

## Install

Download the latest tarball from releases, extract it, and place the binary in
your PATH:
```
cd ~/Downloads
wget https://github.com/Sahel13/echo-linux/...
tar xzf echo-linux-x86_64-0.1.0.tar.gz
ln -s ~/Downloads/echo-linux-x86_64-0.1.0/echo ~/.local/bin  # if `~/.local/bin` is in PATH
```

## Status and requirements

The Linux port currently supports:

- X11 sessions only, support for Wayland is planned.
- x86-64 Linux with glibc for release archives.
- GTK4 and libadwaita.
- ALSA/PipeWire audio input through CPAL.
- A Secret Service implementation for securely storing the Groq API key.
- Groq Whisper transcription using `whisper-large-v3-turbo` or
  `whisper-large-v3`.

On Ubuntu 22.04, install the build dependencies with:

```sh
sudo apt install \
  build-essential pkg-config libadwaita-1-dev libasound2-dev libgtk-4-dev
```

## Build and run

Check the local toolchain and build the application:

```sh
./scripts/bootstrap.sh
```

Run the development build from an X11 session:

```sh
./scripts/dev.sh
```

Run formatting, linting, tests, and packaging-script checks:

```sh
./scripts/check.sh
```

To create an x86-64 release archive and checksum:

```sh
./packaging/build-release.sh
```

The archive is written to `dist/`. See [`packaging/README.txt`](packaging/README.txt)
for installation and runtime dependency details.

## Use Echo

1. Start Echo from an X11 desktop session.
2. Open the settings window and enter a Groq API key.
3. Focus an editable text field.
4. Hold the configured shortcut (F10 by default) and speak.
5. Release the shortcut to transcribe and paste the result.

Press Escape while recording to cancel. Settings also include the shortcut,
Whisper model, language, transcription style, custom vocabulary, microphone,
and last-transcript controls.

## Configuration and data

Echo stores its settings at:

```text
${XDG_CONFIG_HOME:-$HOME/.config}/echo/settings.json
```

The Groq API key is stored through the desktop Secret Service rather than in
the settings file.
