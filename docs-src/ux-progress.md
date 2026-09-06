# Workspace experience pass — 2026-09-05

The full design and functionality improvement remains in progress.

Implemented: working Dashboard mode links; guided Agent empty state; removal of
unavailable rail destinations; modal agent creation with valid chat-engine
selection, loading/saving states, retries, and draft preservation on errors;
visible agent-list failures; New Agent navigation from destinations. Fixed existing
listener typing errors and late cleanup, plus a Clippy warning in the PTY test.

Verified: frozen install; workspace TypeScript/build checks; Rust formatting,
strict Clippy and 44 Rust tests; two existing renderer structure tests; debug macOS
app bundle. Native UI checks confirmed existing agent loading, modal opening,
no-ready-engine state, Escape dismissal, Dashboard at 1280 × 768, and its Code
navigation. Browser checks reported no errors; screenshots remain outside the repo.

Not verified: successful agent creation and actual engine conversation (native
engine detection returned no ready chat engine). Structure tests are not evidence
of those flows. No signed/notarized or release-readiness claim is made.

Next audit: improve Chat folder selection and Code's pre-project state; complete
placeholder settings; fix Plugins destination layout; handle existing agents with
incompatible engines; label face choices accessibly; verify appearance persistence,
light theme, narrow windows, zoom and keyboard behavior. Complete the design's
side-by-side reference visual ritual and record ten deltas; only public descriptions
and Harbor screenshots were inspected in this pass. Maintainer sign-off remains
outstanding. Keep the full goal active.

## Contribution certification

I did not copy or look at the reference product's source.

This contribution is original code and copy written for Harbor under Apache-2.0.
No reference-product assets or credentials were added. Public descriptions informed
behavioral inspiration. Existing asset provenance was not altered or invented.
The pre-existing Cargo.toml change was preserved.
