Echo for Linux
====================

Echo is an x86-64 glibc desktop dictation application for X11 sessions.

Install
-------

Extract this archive and place `echo` somewhere on your PATH, for example:

    tar -xzf echo-linux-x86_64-VERSION.tar.gz
    install -m 755 echo-linux-x86_64-VERSION/echo ~/.local/bin/echo

Run `echo` from an X11 desktop session.

Verification
------------

Verify the downloaded archive before extraction:

    sha256sum -c echo-linux-x86_64-VERSION.tar.gz.sha256

The checksum file must be kept beside the tarball.
