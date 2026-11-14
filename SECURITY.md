# Security policy

## Supported versions

Only the latest release receives fixes.

## Reporting a vulnerability

Please do not open a public issue. Use GitHub's
[private vulnerability reporting](https://github.com/huulanka/epub-gazette/security/advisories/new)
instead. You will get an answer within a week.

## Deployment guidance

- epub-gazette parses untrusted images and text. Run it on an internal network
  only; do not expose it to the internet.
- Set `RENDER_TOKEN` if other services share the network.
- The container runs as a non-root user from a distroless image and works with
  a read-only root filesystem and all capabilities dropped (see
  `compose.example.yml`).
- Request bodies are limited to 20 MiB, decoded images to 12 MiB.
