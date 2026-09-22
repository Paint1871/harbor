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

## Cutting a release

1. Bump `version` in `apps/desktop/src-tauri/tauri.conf.json` and the crate
   versions that track it. Tag `vX.Y.Z`; `tag_is_newer` compares semver, so
   the tag must parse.
2. `pnpm --filter @harbor/desktop tauri build` on each platform.
3. Sign every artifact that the updater may download:

   ```sh
   minisign -Sm "Harbor_X.Y.Z_aarch64.dmg"      # produces .dmg.minisig
   ```

   Sign the artifact file itself, not an archive of it — `verify_release`
   checks the downloaded bytes directly.
4. Publish a GitHub release on the repo `HARBOR_UPDATE_REPO` points at
   (`owner/name`; unset means the updater stays dormant). Attach each
   artifact **and** its `.minisig` sidecar. Asset names must keep their
   platform suffix (`.dmg`, `.msi`, `.AppImage`, `.deb`, `.rpm`) — the
   updater pairs `name.minisig` by filename.
5. Smoke-test the path end to end: install the previous build, run
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
