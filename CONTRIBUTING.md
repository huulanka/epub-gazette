# Contributing

Thanks for your interest! Issues and pull requests are welcome.

## Ground rules

- Keep the renderer **stateless**: no network access, no disk writes, no
  clock-dependent output. The same input must produce the same EPUB.
- Everything the reader sees comes from a theme. Rust code computes data and
  layout; templates and CSS decide the look.
- Kindle compatibility beats browser elegance. Theme CSS must not use flexbox,
  grid, custom properties or generated content, and must not set text colors.

## Workflow

1. Fork and create a branch from `main`.
2. `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
3. If you changed the input model, run `UPDATE_SCHEMA=1 cargo test` and commit
   `schema/issue.v1.json`.
4. If markup changed on purpose, review snapshots with `cargo insta review`.
5. Add an entry under *Unreleased* in `CHANGELOG.md`.
6. Open a pull request. CI also validates the sample issue with W3C epubcheck.

For larger changes (new input fields, new endpoints), please open an issue
first. Architectural decisions are recorded in `docs/adr/`.

## Adding a theme

1. Copy `themes/klassisch-1900` to `themes/<your-id>`.
2. Adjust `style.css`, the XHTML templates (`base`, `front`, `section`, `end`)
   and the SVG templates (`masthead`, `cover`). The template context is built
   in `src/render.rs` (`PaperView`, `SectionView`, `ArticleView`).
3. Put the fonts in `fonts/` together with their licenses. Only fonts that
   allow embedding (e.g. OFL) are accepted. Use static instances, not
   variable fonts.
4. Register the theme in `src/theme.rs`.
5. Render `examples/sample-issue.json` with `"theme": "<your-id>"`, check it in
   Kindle Previewer, and add a screenshot to the pull request.

## Releases

Maintainers tag `vX.Y.Z` on `main` after moving the *Unreleased* changelog
entries under the new version. The release workflow publishes the container
image to GHCR and creates the GitHub release.
