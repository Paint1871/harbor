# Window icon provenance

`icon.png` is the generated Harbor mark used by the desktop bundle. The same
RGBA asset is shared by the UI logo component at
`packages/ui/src/harbor-logo.png`, so the title bar, welcome views, and app icon
cannot drift apart.

The artwork was generated with the built-in ImageGen logo-brand workflow for
Harbor: a minimal open harbor arch enclosing a blue beacon with a restrained
amber signal on a transparent background. It contains no third-party brand
assets, wordmark, or watermark.

`render.py` is retained as a small sync helper for copying the shared UI asset
into this Tauri icon location; it does not recreate the artwork.
