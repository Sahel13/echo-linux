Echo for Linux
====================

Echo is an x86-64 glibc desktop dictation application for X11 sessions only.
It does not support Wayland. It is dynamically linked and is not a portable,
static Linux binary.

Install
-------

Extract this archive and place `echo` somewhere on your PATH, for example:

    tar -xzf echo-linux-x86_64-VERSION.tar.gz
    install -m 755 echo-linux-x86_64-VERSION/echo ~/.local/bin/echo

Run `echo` from an X11 desktop session. On Wayland Echo opens its settings
window and explains that global shortcuts and pasting are disabled.

Ubuntu 22.04 runtime dependencies
----------------------------------

Build and test the release on Ubuntu 22.04 LTS. Install these runtime packages
(their dependencies provide the remaining shared libraries reported by `ldd`):

    sudo apt install \
      libadwaita-1-0 libasound2 libdbus-1-3 libgtk-4-1 libx11-6 \
      libxi6 libxtst6

Echo also needs a running X11 server, a Secret Service implementation for
secure API-key storage, and an audio input available through ALSA/PipeWire.
The package list is intentionally for Ubuntu 22.04; other distributions use
their corresponding GTK4, libadwaita, D-Bus, ALSA, and X11 runtime packages.

Verification
------------

Verify the downloaded archive before extraction:

    sha256sum -c echo-linux-x86_64-VERSION.tar.gz.sha256

The checksum file must be kept beside the tarball.
