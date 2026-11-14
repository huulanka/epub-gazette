# n8n integration

epub-gazette does the typesetting only. Collecting, choosing and writing the
news stays in your workflow. A typical daily workflow:

```
Schedule (05:30)
 → RSS feeds → normalize and deduplicate
 → LLM: pick today's stories (structured JSON)
 → LLM: write the articles (structured JSON)
 → weather, almanac, public-domain engraving
 → Code node: assemble issue.v1 JSON
 → HTTP Request: POST http://epub-gazette:8080/v1/render   (response: file)
 → Send Email: attachment to …@kindle.com
```

## Running next to n8n

Attach the container to the network n8n uses and do not publish a port (see
[`compose.example.yml`](../compose.example.yml)). n8n then reaches it as
`http://epub-gazette:8080`.

## HTTP Request node

| Setting | Value |
|---------|-------|
| Method | `POST` |
| URL | `http://epub-gazette:8080/v1/render` |
| Send Body | JSON, *Using JSON*: `{{ $json.issue }}` |
| Response Format | **File**, put output in field `data` |
| Header (optional) | `Authorization: Bearer {{ $env.RENDER_TOKEN }}` |

Set *Options → Response → Never Error* off, so validation errors (HTTP 422)
fail the execution. The error body lists every problem, which helps when
tuning LLM prompts.

The resulting binary property can go straight into the attachment of a
*Send Email* or *Gmail* node. Use the file name from the response's
`Content-Disposition`, or set your own, e.g.
`ragazzi-observer-{{ $now.toFormat('yyyy-MM-dd') }}.epub`.

## Front image from Wikimedia Commons

Public-domain engravings make good front images. Search via the Commons API
and keep only free licenses:

```
https://commons.wikimedia.org/w/api.php?action=query&format=json
  &generator=search&gsrnamespace=6&gsrlimit=10
  &gsrsearch=<keyword> engraving filetype:bitmap
  &prop=imageinfo&iiprop=url|size|extmetadata&iiurlwidth=1400
```

- Accept `extmetadata.LicenseShortName` of `Public domain`, `CC0` or
  `No restrictions`. Reject CC BY / BY-SA, unless you add attribution.
- Download `thumburl` (already scaled to 1400 px), base64-encode it into
  `front_image.data` and put the source into `front_image.credit`.
- Send a descriptive `User-Agent`, as Wikimedia's API policy asks.

## Tips for LLM-written issues

- Ask for JSON that matches `GET /v1/schema` (OpenAI-compatible APIs and
  OpenRouter accept it as `response_format.json_schema`).
- Paragraphs are plain text. Markdown or HTML is printed literally.
- `teaser` keeps the front-page index and the cover short when headlines are long.
- `kicker` is good for labels like *Fortsetzung* (follow-up) or *Hintergrund*.
