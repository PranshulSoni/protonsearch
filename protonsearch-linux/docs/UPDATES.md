# ProtonSearch Linux updates

The Linux Settings app checks the official GitHub Releases API. It does not
execute update code from a branch, pull request, or arbitrary download host.

## Release requirements

Create a semantic version tag such as `v1.1.0`. The Linux release workflow
builds and publishes the supported package assets:

- Arch: `.pkg.tar.zst`
- Debian/Ubuntu/Mint: `.deb`
- Fedora/RHEL-family: `.rpm`
- `SHA256SUMS.txt`, containing one checksum for every package asset

The updater selects the package family and architecture reported by the local
installation. A package is not installable until its size, HTTPS URL, and
SHA-256 value all match the release metadata.

## Installation behavior

Package-managed installations are updated through the local package manager
with a visible privilege prompt. Manual installations use a same-directory
atomic replacement and keep a rollback copy until the new executable passes a
health check. The service is stopped only during installation and is restarted
if it was active before the update. Settings, index data, clipboard history,
and other XDG user data are never replaced by an update.

Automatic checks run after startup at most once every 24 hours. They never
install silently; the user must select **Download and install**.
