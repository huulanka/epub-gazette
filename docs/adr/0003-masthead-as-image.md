# ADR 0003: Render the masthead and the cover as images

- Status: accepted
- Date: 2026-11-14

## Context

Kindle replaces embedded fonts as soon as the reader picks any font other than
*Publisher Font*. A Fraktur masthead set as text would then turn into Bookerly,
and the retro identity would be lost. The library cover must be a raster image
anyway.

## Decision

The masthead (PNG) and the cover (JPEG) are rendered with resvg from SVG
templates, using only the theme's fonts. The body text stays live text with
embedded fonts, so it still reflows and responds to font size, line spacing
and dark mode.

## Consequences

- The masthead looks the same on every device and setting.
- In dark mode the masthead image stays white. This is acceptable and reads
  like a pasted-in clipping.
- The masthead font does not need to be embedded, which keeps the book smaller.
