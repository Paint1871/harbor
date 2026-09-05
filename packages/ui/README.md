# Harbor UI

PR-02 provides the v0 design tokens and six primitives: Button, Segmented,
RailRow, Composer, Card, and Pill. Imports are explicit subpaths, for example
`@harbor/ui/Button`. Consumers compile the TypeScript source with Vite.

Import `@harbor/ui/tokens.css` followed by `@harbor/ui/primitives.css` once at the
application entry point. Tokens are shared across Black and Light; surfaces,
text, borders, and glow intensity change with appearance. Font stacks use local
fallbacks and make no font-service requests.

Use one `ThemeProvider` at the application root. Its `theme` and `reduceMotion`
props are controlled by the host; persistence belongs to SQLite settings in a
later PR. With no props it uses Black and the OS motion preference. App Reduce
Motion can enable stillframes even when the OS preference is off; the OS
accessibility preference is never overridden. `useTheme().reducedMotion` also
lets future canvas renderers avoid starting animation loops.

Before CSS or React loads, host HTML must declare `data-theme="black"` and a
matching dark background. `forceDark()` provides synchronous initialization for
renderer entry points; it does not replace the host HTML or native window
background. The desktop window integration is PR-03.

Buttons default to `type="button"`; icon-only buttons need an accessible label.
Segmented is a native radio group with a required group label. RailRow is a
button with presentational leading/trailing slots; do not nest interactive
controls inside it. Pill is presentational unless the caller adds a live-region
role. Status always includes text as well as color.

Composer is controlled and emits the original nonblank text on Enter or Send.
Shift+Enter inserts a newline. IME composition and modified shortcut keys are
not submitted. The host owns draft clearing, pending turns, errors, attachments,
and ACP configuration controls. No engine options are invented here.

## Development preview

```sh
pnpm --filter @harbor/ui dev
pnpm --filter @harbor/ui check
```

The isolated preview is a component fixture, not the desktop app. It contains
only local sample state and never connects to an engine. `check` runs strict
TypeScript validation and builds the preview with Vite. Browser QA should cover
Black/Light, OS and app Reduce Motion, keyboard radio navigation, disabled
controls, multiline entry, IME, and empty-message suppression. Screenshots stay
outside the repository. Desktop chrome calibration and maintainer visual
sign-off belong to the subsequent chrome PRs.
