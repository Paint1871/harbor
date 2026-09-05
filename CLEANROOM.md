# Clean-room contribution policy

Harbor implements a supplied behavioral and visual specification with original
code and assets. It is not a fork of BridgeMind. Public descriptions can explain
behavior; they are not a source of implementation code or redistributable assets.

Do not inspect or copy the reference product's source, installers, binaries,
logos, screenshots, faces, CDN assets, marketing paragraphs, or backend flows.
Create Harbor's logo, faces, and orb independently. Record generated asset
provenance, including prompts and available generation metadata, when assets
are introduced. Never claim a seed or provenance record that was not supplied.

## Contributor certification

For every contribution, certify:

> I did not copy or look at the reference product's source.

Also certify that you created the contribution or have the right to submit it
under Apache-2.0, that its provenance is accurately described, and that it
contains no reference-product assets or credentials. Include this certification
in the PR description. A commit sign-off may record contributor identity;
automated tools must not invent another person's attestation or signature.

## Automated guard

`scripts/deny-brand.sh` scans tracked and non-ignored untracked file names and
contents, including hidden files and text embedded in binary files. Tracked
files are scanned even when an ignore rule matches them. Diagnostics report
file names, not matching content, to avoid exposing accidental credentials.

The guard rejects these case-insensitively:

- BridgeMind, BridgeSpace, BridgeVoice, BridgeAgent, BridgeMCP, BridgeShot,
  BridgeSwarm, and BridgeBench (including download and plugin-gateway hosts
  containing those names).
- Cognito, Stripe price identifiers (`price_` followed by letters or digits),
  and the phrase “Agent Super App”.

Only these exact repository-relative documentation paths are exempt:
`CLEANROOM.md`, `docs-src/references.md`, and `DESIGN.md`. The supplied design
normally stays outside the repository. There is no general docs, lockfile,
script, fixture, or source exemption. Unknown reference infrastructure must be
added to the guard if discovered; do not contact it to implement Harbor.

The guard cannot prove asset ownership or detect copied images. Reviewers must
check provenance and the contribution certification as well.

## Bootstrap provenance

PR-01 was written from the supplied Harbor design without inspecting the
reference product's source or fetching its assets. The license is the canonical
Apache-2.0 text from the Apache Software Foundation. The code of conduct is
original Harbor community copy. No product assets are included in this PR.
