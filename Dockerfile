# syntax=docker/dockerfile:1

# ---- build ------------------------------------------------------------------
FROM rust:1.99-slim-trixie AS build
WORKDIR /src

# Cache dependencies separately from the application code.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs \
    && cargo build --release --locked \
    && rm -rf src target/release/epub-gazette* target/release/deps/*epub_gazette* \
       target/release/.fingerprint/epub-gazette-*

COPY src ./src
COPY themes ./themes
# touch: copied files may be older than the placeholder build above
RUN touch src/main.rs src/lib.rs && cargo build --release --locked \
    && cp target/release/epub-gazette /epub-gazette

# ---- runtime ----------------------------------------------------------------
# distroless: glibc + CA certs, no shell, runs as uid 65532.
FROM gcr.io/distroless/cc-debian13:nonroot
COPY --from=build /epub-gazette /usr/local/bin/epub-gazette

ENV BIND=0.0.0.0:8080 \
    RUST_LOG=info
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/epub-gazette", "healthcheck"]

ENTRYPOINT ["/usr/local/bin/epub-gazette"]
CMD ["serve"]

LABEL org.opencontainers.image.title="epub-gazette" \
      org.opencontainers.image.description="Turns a JSON newspaper issue into a typeset, Kindle-ready EPUB" \
      org.opencontainers.image.source="https://github.com/huulanka/epub-gazette" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0"
