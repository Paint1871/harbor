# PR-02 verification

Verified locally on 2026-09-05 in Chromium 152 using the isolated Vite preview.
This verifies the shared primitives, not the native desktop window or release.

- All 31 Black and 6 Light token declarations match Design §5.2. Shared
  dimensions and accents remain available in Light.
- OS Light with a fresh page still starts Black. Appearance switching updates
  surfaces and native control color scheme. The synchronous forced-dark helper
  sets both the theme attribute and color scheme.
- Native radio arrow selection works, and Tab leaves the group after one stop.
- Enter submits once with the original multiline text. Shift+Enter inserts a
  newline. Blank/whitespace-only drafts cannot submit.
- IME composition Enter, the legacy key-code-229 case, and modified shortcuts
  are not intercepted. Enter works after composition ends.
- Disabled buttons, the segmented group, and the composer reject input.
- Both app and live OS Reduce Motion produce a stillframe, suppress CSS
  animation, and update the React context. Turning the OS preference off
  restores motion when the app preference is also off.
- Black and Light screenshots were visually inspected at 1280 × 768; the preview
  also has no horizontal overflow at 800 × 600. Captures remain outside Git.
- Browser logs show no application errors or Vite error overlay.
- `pnpm install --frozen-lockfile` and `pnpm check` pass: clean-room guard,
  Rust format/Clippy, strict UI TypeScript checking, and Vite build.

Native WebKit rendering, the title bar, and desktop first-paint checks follow
with PR-03/04. This preview does not claim desktop chrome visual sign-off.

# Visual language overhaul (2026-09-06)

Elevated `@harbor/ui` from generic dark chrome to an editorial, near-black
host. §5.2 token names and values are unchanged. Complementary tokens cover
hairline, shadows, field/well/overlay, focus glow, title/label tracking, and
pill/field radii. Light uses paper (white field, warm well) rather than gray
fill. `applyTheme` now writes the canvas color onto `html` so Light can
replace the first-paint inline black; HTML still starts `data-theme="black"`.

Primitives: inverted primary, quiet ghost, inset segmented pill, raised
two-line rail rows without chunky borders, 16px composer with think-violet
focus ring and a quiet footer. Class names and composer behavior (Enter /
Shift+Enter / IME / disabled) are unchanged. Reduce Motion still zeros
`--harbor-motion-duration` and kills animation. Forced-colors rules remain.

Verified: `pnpm --filter @harbor/ui check` — strict `tsc --noEmit` and Vite
preview build, both clean. Desktop chrome should consume the new complementary
tokens; this note does not sign off native window paint.
