# Kindle notes

## Delivering issues

Amazon's **Send to Kindle** accepts EPUB by e-mail:

1. Find your device address under *Amazon → Manage Your Content and Devices →
   Preferences → Personal Document Settings* (`…@kindle.com`).
2. Add the sending address to *Approved Personal Document E-mail List*.
3. Mail the `.epub` as an attachment (max. 50 MB; an issue is about 1–2 MB).
   Subject and body are ignored.

Amazon converts the EPUB on its servers. It rejects books with structural
errors, which is why CI runs epubcheck.

## In the library

- The library shows `images/cover.jpg`. It carries both the EPUB 3
  `cover-image` property and the EPUB 2 `<meta name="cover">` that Kindle
  conversion looks for.
- The title is `<Name> – <weekday> <date>`, so issues sort sensibly and remain
  distinguishable.

## Reading settings

- Choose **Aa → Font → Publisher Font** to use the embedded Libre Caslon and
  Old Standard. Other fonts work, but the look is less period-accurate.
- The masthead and the cover are images, so the Fraktur never changes.
- Dark mode inverts text and rules. The masthead image stays white, like a
  clipping pasted onto the page.

## What Kindle CSS can and cannot do

| Works | Avoid |
|-------|-------|
| `em` sizes, margins, padding, borders (incl. `double`) | flexbox, grid, multi-column |
| `float` (drop caps), `text-transform`, `letter-spacing` | CSS custom properties |
| `font-variant: small-caps`, `@font-face` with TTF | `::before` / `::after` content |
| `page-break-*`, `hyphens: auto` with `xml:lang` | fixed text colors (breaks dark mode) |
| PNG/JPEG images | variable fonts, animated GIFs (only the first frame shows) |

Each section is its own XHTML file, so every section starts on a new page and
appears as a chapter in the Kindle's *Go To* menu, with its articles nested
below.

## Previewing

Amazon's free **Kindle Previewer 3** (macOS/Windows) converts an EPUB the same
way Send to Kindle does and shows it on simulated devices. It is the quickest
check after changing a theme.
