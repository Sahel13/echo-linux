# Echo for Linux

Echo is an X11 desktop dictation application. This repository currently
contains the minimal GTK4/libadwaita application scaffold; dictation features
are implemented incrementally according to `feature-list.json`.

## Requirements

Echo's first release supports X11 only. Start it from an X11 desktop session
with a working `DISPLAY` (for example, an Xorg GNOME session). Wayland is not
supported yet.

The Ubuntu 22.04 build dependencies are:

```sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev
```

Install a current stable Rust toolchain through [rustup](https://rustup.rs/).

At runtime, the application dynamically needs GTK4 and libadwaita, supplied by
Ubuntu packages such as `libgtk-4-1` and `libadwaita-1-0`. It is not a static
or universally portable Linux binary.

## Build, test, and run

From the repository root:

```sh
scripts/bootstrap.sh
scripts/check.sh
scripts/dev.sh
```

`bootstrap.sh` is repeatable: it verifies the GTK4/libadwaita development
libraries, fetches the locked Rust dependencies, and builds the project.
`check.sh` runs formatting, Clippy with warnings denied, and tests. `dev.sh`
builds and launches Echo while enabling Rust backtraces and GTK diagnostic
messages.

## Development workflow

Read `AGENTS.md` and `docs/PORTING_GUIDE.md` before changing a feature. The
ordered feature list and recorded verification evidence live in
`feature-list.json` and `progress.md`.
