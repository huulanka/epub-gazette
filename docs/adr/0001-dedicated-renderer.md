# ADR 0001: A dedicated rendering service instead of building EPUBs in n8n

- Status: accepted
- Date: 2026-11-14

## Context

A daily newspaper is assembled in n8n and has to reach a Kindle as an EPUB.
n8n can zip files (Compression node) and run JavaScript (Code node), so building
the EPUB inside the workflow looked possible without extra infrastructure.

A good EPUB needs more than a zip, though:

- an uncompressed `mimetype` as the first entry (the Compression node has no control over this);
- embedded fonts, a masthead and a cover rendered as images with those fonts;
- grayscale processing of illustrations;
- templates and CSS that can be versioned, reviewed and tested;
- validation against epubcheck, because Amazon rejects broken books.

We looked for an existing project offering "JSON in, typeset EPUB out" as a
service. KindleEar, RSSPub, the-daily-epub and kindle-news-filter all bundle
fetching, storage and delivery, and none exposes rendering on its own.

## Decision

Build a small, stateless HTTP service (`epub-gazette`) that renders a
documented JSON format into an EPUB. n8n stays the orchestrator and calls it
over the internal Docker network.

## Consequences

- One more container to run. It holds no state, needs no volume and uses little memory.
- The look lives in a Git repository with tests and CI rather than in workflow code fields.
- The service is useful to others and is published as open source.
