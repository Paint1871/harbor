# Releasing Harbor

This is the runbook for cutting a signed release. Until every step here has
run once against a real artifact, the update channel is implemented but
unproven — `docs/status.md` tracks that gate.

## Key custody

- `apps/desktop/src-tauri/minisign.pub` is the release **public** key, baked
  into every build via `harbor_updater::PUBLIC_KEY`. It is safe to commit.
- The matching **secret** key (`minisign.key` plus its password) stays with
  the maintainer. It is never committed, never attached to a release, and
  never placed on CI runners that build untrusted code. Keep one offline
  backup; losing it means every shipped install keeps verifying against a
  key nothing can sign for, and the only fix is a manual reinstall of a
  build carrying a new public key.
- If the key was ever generated ad-hoc for development and not retained,
  generate a deliberate release key (`minisign -G -p minisign.pub -s
  minisign.key`) before the first signed release and commit the new `.pub`.
- CI signs with two repository secrets: `MINISIGN_KEY_B64` (the base64-encoded
  `minisign.key` box) and `MINISIGN_PASSWORD`. Signing runs through
  `cargo run -p harbor-updater --example release_sign`, the same crate code
  the verify-side tests exercise — no third-party signer on the runner.

## Cutting a release

`.github/workflows/release.yml` automates steps 2–4: a matrix builds the
bundles on macOS/Windows/Ubuntu, every artifact gets a `.minisig` from the
release secret, a `publish` job generates `manifest.json`
(`scripts/release_manifest.py`), signs it, and uploads everything to a GitHub
release named for the tag. Manual equivalents are kept below for auditing.

1. Bump `version` in `apps/desktop/src-tauri/tauri.conf.json` and the crate
   versions that track it. Tag `vX.Y.Z`; `tag_is_newer` compares semver, so
   the tag must parse.
2. `pnpm --filter @harbor/desktop tauri build` on each platform.
3. Write `manifest.json` for the release and sign it — the signed manifest
   is the trust anchor that binds tag, platform, file name and sha256, so a
   signed artifact can never be replayed under a different tag:

   ```sh
   sha256sum Harbor_X.Y.Z_aarch64.dmg Harbor_X.Y.Z_x64.msi …
   ```

   ```json
   {
     "version": "X.Y.Z",
     "minimum_version": "0.1.0",
     "artifacts": [
       { "os": "macos",   "arch": "aarch64", "file": "Harbor_X.Y.Z_aarch64.dmg", "sha256": "…" },
       { "os": "macos",   "arch": "x86_64",  "file": "Harbor_X.Y.Z_x64.dmg",     "sha256": "…" },
       { "os": "windows", "arch": "x86_64",  "file": "Harbor_X.Y.Z_x64.msi",     "sha256": "…" },
       { "os": "linux",   "arch": "x86_64",  "file": "Harbor_X.Y.Z_amd64.AppImage", "sha256": "…" }
     ]
   }
   ```

   - `version` must equal the tag minus its `v` prefix — a manifest for a
     different release is refused.
   - `minimum_version` is optional; builds older than it are not offered the
     update (use it when an update path cannot cross a breaking change).
   - `os`/`arch` are Rust's `std::env::consts` spellings (`macos`, `windows`,
     `linux` × `aarch64`, `x86_64`). `file` must keep its platform suffix
     (`.dmg`, `.msi`, `.AppImage`, `.deb`, `.rpm`) — that is a sanity bound,
     not the selection mechanism; the manifest is.

   ```sh
   minisign -Sm manifest.json                    # produces manifest.json.minisig
   ```

4. Publish a GitHub release on the repo `HARBOR_UPDATE_REPO` points at
   (`owner/name`; unset means the updater stays dormant). Attach every
   artifact plus `manifest.json` **and** `manifest.json.minisig` — a release
   without the signed manifest pair is invisible to the updater.
5. macOS code signing and notarization run inside `tauri build` when the
   `APPLE_*` secrets are configured (certificate, identity, and either the
   `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` or `APPLE_API_KEY` credential
   pair). Without them the bundle still builds and is minisign-signed, but
   Gatekeeper will warn — do not treat an unsigned build as shippable for
   end users.
6. Smoke-test the path end to end: install the previous build, run
   `HARBOR_UPDATE_REPO=owner/name` with it, check for updates in Settings →
   General, download, confirm the staged file lands in `updates/` and the
   signature was verified (a tampered artifact must be refused, never
   staged).

## Adapter supply chain

ACP adapters listed in `packages/engine-catalog/src/catalog.json` install
through npm into the app-data `adapters/` directory. Two rules keep that
channel honest:

- `adapterPackage` is always `name@exact-version` — never a range, never
  `latest`. `install_adapter` refuses a spec it cannot parse into an exact
  pin.
- `adapterIntegrity` carries the tarball's published `dist.integrity`
  (`sha512-…` from `npm view <pkg>@<version> dist.integrity`). After npm
  exits, Harbor reads `node_modules/.package-lock.json` and requires the
  recorded version and integrity to match the catalog exactly; a mismatch
  deletes the package before it can run.
- Install runs with `--ignore-scripts`, so no lifecycle hook ever executes
  on this machine. Only the top-level tarball is pinned — transitive
  dependencies still resolve live through npm, which is the residual
  supply-chain gap; vendoring adapters would close it.

Bumping an adapter means updating both fields together; fetch the integrity
for the new version with `npm view`, not by rerunning an install.
