# ADR-0011: File-type filtering

- Status: Accepted
- Date: 2026-06-07

## Context

A dropped folder contains arbitrary files. We must decide which are video candidates worth
probing, without probing everything (slow) or trusting extensions blindly (wrong).

## Decision

Filter candidates by extension, with one probe-based fallback:

- **Known video extension** → include directly, no probe needed.
- **No extension** → `ffprobe` it; include only if a video stream is found.
- **Known non-media extension** → skip immediately.
- **Unknown extension** → skip.

## Consequences

- Cheap in the common case (extension match), correct in the tricky one (extensionless files).
- Files that pass the filter are still probed in pre-flight for duration and stream
  confirmation ([ADR-0010](0010-preflight-encode-separation.md)); audio-only files are rejected
  there.
- The known-extension allowlist must stay broad to match the "any video file" goal
  ([ADR-0005](0005-prebuilt-ffmpeg-binaries.md)).
