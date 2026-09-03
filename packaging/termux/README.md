# Termux package recipe

This directory mirrors the required placement in the `termux/termux-packages`
repository: copy `war-dodger/` to `packages/war-dodger/`.

Before submitting a pull request:

1. Create and push a release tag matching `TERMUX_PKG_VERSION`.
2. Download its source archive and replace the placeholder checksum in
   `build.sh` with its SHA-256 value. Termux review requires a real checksum.
3. Build it from a checkout of `termux-packages` with
   `./build-package.sh packages/war-dodger` and test the generated `.deb` on a
   physical Android device with Termux:API installed.
4. Run the repository's package-format and CI checks, then submit only this
   recipe (not a compiled binary).

The runtime dependency is `termux-api`. The separate
Termux:API Android application and its location permission remain user-facing
requirements; a package cannot grant them.
