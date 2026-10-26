# In-app updates

Apex Overlays checks GitHub Releases for a newer version (on start-up, and from
**Settings → General → Updates**). When one is found you click **Install … and restart**:
the installer is downloaded, its signature is verified, it installs silently and the app restarts.

## How it works

- Each release carries a `latest.json` manifest and a signed installer. The app reads
  `https://github.com/ShamzOut2/Apex_Overlays/releases/latest/download/latest.json`.
- Installers are signed with a private key that only lives in the repository's GitHub secrets.
  The app contains the matching public key (`apps/desktop/tauri.conf.json → plugins.updater.pubkey`)
  and refuses any update that is not signed with it.

## One-time setup (repository owner)

1. Open the repository on GitHub → **Settings → Secrets and variables → Actions → New repository secret**.
2. Name: `TAURI_SIGNING_PRIVATE_KEY`. Value: the full contents of the private key file you were given
   (`apex-overlays-updater.key`). The key has no password, so no password secret is needed.
3. Keep a backup of the key somewhere safe. If it is lost, installed copies can no longer verify new
   versions, and everyone would have to reinstall once with a new key.

Without the secret, releases still build (installer, portable exe, server) but contain no `latest.json`,
so installed apps will not see them as updates.

## Publishing an update

1. Bump the version in `Cargo.toml` (`[workspace.package] version`), `apps/desktop/tauri.conf.json` and `ui/package.json`.
2. Merge to `main`, then **Actions → Release → Run workflow** with the version (e.g. `v0.5.0`), or push a tag `v0.5.0`.
3. Installed apps offer the update the next time they start or when you click *Check for updates*.

Copies older than 0.4.0 have no updater: install 0.4.0 (or later) once by hand; every update after that
happens inside the app.
