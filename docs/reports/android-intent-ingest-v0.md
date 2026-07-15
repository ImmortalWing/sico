# Android Intent and package ingestion v0 review

> - status: accepted
> - date: 2026-07-16
> - phase: M6

## Result

GO for STEP-0056 contract/cross-check. View, Send and picker results accept exactly one `content:` URI, exact Sico MIME and `.sapp` display name. Provider bytes are read once with a 64 MiB + 1 sentinel and then handed to the shared signed install path.

The `sico://open` deep link carries no package URI; it may only open the system picker. File/http schemes, MIME/name mismatch, multiple ClipData items and oversized streams fail closed. Manifest and Kotlin remain contract-reviewed without SDK compilation.

