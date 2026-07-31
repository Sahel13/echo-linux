# Echo for Linux

<p align="center">
  <img src="assets/echo-logo.svg" width="460" alt="Echo logo and wordmark">
</p>

Echo is an X11 desktop dictation application. Echo is distributed as a
versioned `x86_64` glibc tarball. It is dynamically linked, so it is intended
for the documented Ubuntu 22.04 LTS baseline rather than every Linux
distribution and release.

## Requirements

Echo's first release supports X11 only. Start it from an X11 desktop session
with a working `DISPLAY` (for example, an Xorg GNOME session). Wayland is not
supported yet.

The Ubuntu 22.04 build dependencies are:

```sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev
```

Install a current stable Rust toolchain through [rustup](https://rustup.rs/).

At runtime on Ubuntu 22.04, install:

```sh
sudo apt install libadwaita-1-0 libasound2 libdbus-1-3 libgtk-4-1 libx11-6 libxi6 libxtst6
```

Those packages pull in the remaining GTK, GLib, Cairo, Pango, font, and X11
libraries needed by the dynamically linked binary. Echo also requires an X11
server, a Secret Service implementation, and an ALSA/PipeWire microphone.

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

## Release archive

Build the x86-64 archive on Ubuntu 22.04 LTS:

```sh
packaging/build-release.sh
packaging/verify-release.sh dist/echo-linux-x86_64-0.1.0.tar.gz
```

This produces a tarball and a neighboring SHA-256 checksum. The archive
contains `echo`, `README.txt`, `LICENSE`, and the Echo PNG icon under the XDG
icon layout. Verify the checksum before extracting it; no Rust toolchain is
needed to run the extracted binary.

To build, smoke-test, and export that archive from a clean Ubuntu 22.04
container, use Docker BuildKit:

```sh
docker build --target smoke-test -f packaging/Dockerfile.ubuntu-22.04 .
docker build --target archive --output type=local,dest=dist -f packaging/Dockerfile.ubuntu-22.04 .
packaging/verify-release.sh dist/echo-linux-x86_64-0.1.0.tar.gz
```

The smoke-test stage has no Rust toolchain. It verifies the checksum and
dynamic libraries, then runs the extracted binary under an isolated X11 server
for three seconds; a healthy GUI process is expected to be stopped by the
timeout.

## Development workflow

Read `AGENTS.md` and `docs/PORTING_GUIDE.md` before changing a feature. The
ordered feature list and recorded verification evidence live in
`feature-list.json` and `progress.md`.
