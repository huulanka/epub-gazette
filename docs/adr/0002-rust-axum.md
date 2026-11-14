# ADR 0002: Rust with axum, themes compiled into the binary

- Status: accepted
- Date: 2026-11-14

## Context

The service runs on a small NAS next to other containers. It needs SVG
rasterization with custom fonts, image processing, templating and an HTTP API.
The maintainer prefers established, well-supported tools.

## Decision

- Rust with axum/tokio for the HTTP layer.
- `epub-builder` for packaging, `resvg` for SVG, `image` for raster work,
  `minijinja` for templates, `schemars` for the published JSON Schema.
- Themes (CSS, templates, fonts) are embedded with `include_bytes!`, so the
  binary is self-contained and the image is distroless.

## Consequences

- A single static-ish binary with a small, non-root image and low memory use.
- Adding a theme needs a rebuild. This is fine for now; loading themes from a
  directory can be added later without changing the input format.
