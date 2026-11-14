# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.1] - 2026-11-14

### Changed

- The library cover lists article headlines instead of teasers. Teasers are longer and were cut off mid-sentence after two lines.

## [0.1.0] - 2026-11-14

### Added

- `issue.v1` JSON format with validation, sanitizing and a published JSON Schema.
- EPUB 3 rendering with NCX, nested table of contents, embedded fonts and Kindle cover metadata.
- Theme `klassisch-1900`: Fraktur masthead, Old Standard headlines, Libre Caslon body, drop caps, pull quotes, analysis boxes.
- Rendered masthead (PNG) and library cover (JPEG) with the theme's own fonts.
- E-ink preparation of the front illustration (grayscale, auto levels, resizing).
- CLI (`render`, `validate`, `schema`, `themes`, `serve`, `healthcheck`) and HTTP API with optional bearer token.
- German and English labels and date formats.
- Distroless container image, CI with epubcheck, multi-arch releases to GHCR.
