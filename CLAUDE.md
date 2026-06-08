# CLAUDE.md

## Shell gotchas (Windows + PowerShell default)

- **`cargo` needs `--manifest-path src-tauri/Cargo.toml`** when run from repo root.
- **Use the PowerShell tool for `git` commands**, not Bash. `$env:VAR` and backtick escapes
  (`n, `t) are PowerShell syntax that Bash silently mangles. If you must use Bash for git,
  always pass absolute Windows paths (e.g. `C:/Users/.../file.txt`).
- **Commit messages:** write to a temp file with `Write`, then `git commit -F <abspath>`.
  Never inline here-strings or heredocs — both shells have traps (PowerShell `@'...'@`
  leaks literal `@` lines; Bash heredocs interact poorly with Windows path separators).

## Agent skills

### Issue tracker

Issues live as local markdown files under `.scratch/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default label strings (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context repo — one `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.
