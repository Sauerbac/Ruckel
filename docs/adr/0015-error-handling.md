# ADR-0015: Error handling — skip-and-continue + summary

- Status: Accepted
- Date: 2026-06-07

## Context

In a batch, one bad file should not abort the rest. Errors come in two flavors: those
detectable before encoding and those that surface during it.

## Decision

- **Pre-flight errors** (audio-only, unreadable file): collected before encoding starts and
  surfaced in the plan.
- **Encode errors:** skip the failed file, continue the batch.
- **End-of-batch summary** reports every failure with its reason.

## Consequences

- A single failure never derails a batch.
- All failures are reported together at the end, not as a stream of interruptions.
- UI: errored rows auto-expand with the message; the status bar shows an "N done · N errors"
  summary. No completion modal. See [ADR-0020](0020-frontend-interaction-model.md).
