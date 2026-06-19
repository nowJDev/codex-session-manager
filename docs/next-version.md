# Next Version Work Items

These items track follow-up work after the first Codex release line.

## 1. Automatic Summary Isolation Audit

Goal:

- Confirm `codex exec` summary runs do not pollute the user's normal session list over repeated real-world use.

Current state:

- Summary runs use `~/.codex-sessions/.summary-runs` as `cwd`.
- The backend attempts to clean generated JSONL files whose project folder name contains `summary-runs`.

Next checks:

- Run repeated automatic summaries with a real Codex CLI account.
- Confirm no summary-created rollout remains visible in the normal session list.
- Consider running summaries with a temporary `CODEX_HOME` if any rollout leakage appears.

## 2. Frontend Action Dispatch Tests

Goal:

- Add lightweight frontend coverage for table menu actions.

Target interactions:

- Archive calls `archiveSession` for active sessions.
- Unarchive calls `unarchiveSession` for archived sessions.
- Delete passes `sessionId` and `filePath`.
- Cloud button chooses upload for local/synced sessions and checkout for cloud-only sessions.

## 3. Tauri GUI E2E Harness

Goal:

- Add a stable way to open the desktop app and verify core flows visually or through automation.

Candidate checks:

- App launches with an isolated `CODEX_HOME`.
- Session row appears from a synthetic rollout file.
- Archive/cloud/delete actions refresh the list as expected.

## 4. Release Migration Notes

Goal:

- Keep public release guidance clear for early testers.

Notes:

- `v0.5.0` is preserved as the first Codex port tag.
- Public installers should use `v0.5.1` or newer.
- Add migration notes if any user installed local `v0.5.0` artifacts before the public release workflow was fixed.
