# I1 — Multi-file

Status: needs-triage

## What to build

Thicken the spine from one file to **N**. The scanner (B3) emits multiple Conversion Jobs; the
encoder processes them **sequentially** (ADR-0009); the frontend left panel renders **N file rows**
and advances each via the `file_index` / `total_files` already carried in the progress event. No
contract change — the tracer's payloads already anticipate N.

## Acceptance criteria

- [ ] Dropping a folder (or multiple files) produces one row per candidate video and one `_ppt.mp4` per source
- [ ] Jobs encode sequentially; the active row shows progress while others show queued/done state
- [ ] `file_index`/`total_files` drive per-row and overall progress correctly
- [ ] Overall status-bar progress reflects the whole batch

## Blocked by

- 05 (file-row component states)
- 08 (scanner emitting N jobs)
