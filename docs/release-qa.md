# Release QA

This document records release-level checks that are not fully covered by unit or integration tests.

## Current Release

- Current installer release: `v0.5.1`.
- Release page: <https://github.com/nowJDev/codex-session-manager/releases/tag/v0.5.1>.
- `v0.5.0` remains as the first Codex port tag, but the installer release was superseded by `v0.5.1` because GitHub Actions needed Node 24 for `pnpm 11.8.0`.

## Installer Smoke Test

Windows release assets to test:

- `Codex.Session.Manager_0.5.1_x64-setup.exe`.
- `Codex.Session.Manager_0.5.1_x64_en-US.msi`.

Smoke-test checklist:

1. Install the Windows release asset.
2. Launch Codex Session Manager.
3. Confirm the app opens without a crash.
4. Confirm Settings diagnostics find `codex` when Codex CLI is installed.
5. Uninstall or keep the app intentionally after testing.

Result on 2026-06-19:

- Downloaded `Codex.Session.Manager_0.5.1_x64-setup.exe` from the GitHub release.
- Verified SHA256 `f4ca828d4e583a332906c19cbf32e737d91d6fe7e9289688780f0eb28b3cdb4c`, matching the release asset digest.
- Installed silently into a temporary directory.
- Launched `codex-session-manager.exe` and confirmed it stayed alive for 8 seconds.
- Ran the bundled `uninstall.exe /S`; install directory and Start Menu link were removed.

## Session Action Checks

Archive and deletion actions should continue to use Codex CLI before direct file manipulation:

- `codex archive <session-id>`.
- `codex unarchive <session-id>`.
- `codex delete <session-id>`.

Backend coverage:

- `scanner_archive_actions_use_codex_cli`.
- `scanner_delete_session_uses_codex_cli_before_file_fallback`.
- `scanner_marks_archived_sessions`.

Smoke-test result on 2026-06-19:

- Ran `session-cli archive` and `session-cli unarchive` against an isolated `CODEX_HOME` with a fake `CODEX_CLI`.
- Confirmed fake Codex CLI received `archive <session-id>` and `unarchive <session-id>`.
- Ran `session-cli delete` and confirmed fake Codex CLI received `delete <session-id>`.
- Confirmed the target rollout file was removed by the CLI action.

## Cloud Sync Checks

Cloud sync uses the `Codex Sessions` folder under the selected cloud root and restores rollout files into the Codex date folder layout.

Backend coverage:

- `cloud_checkout_restores_codex_rollout_date_path`.
- `cloud_only_sessions_are_reported_with_cloud_only_storage_type`.

Manual smoke-test checklist:

1. Configure a temporary cloud root.
2. Upload a local session.
3. Delete the local rollout copy.
4. Check out the cloud session.
5. Confirm it restores to `~/.codex/sessions/YYYY/MM/DD/`.
6. Edit the restored local rollout file.
7. Check in the cloud session.
8. Confirm the cloud JSONL is updated and the lock file is released.

Smoke-test result on 2026-06-19:

- Ran `session-cli upload`, `session-cli checkout`, and `session-cli checkin` against an isolated `CODEX_HOME` and temporary `Codex Sessions` cloud folder.
- Confirmed checkout restored the rollout file into the Codex date-folder layout.
- Edited the checked-out rollout and confirmed checkin copied the edit back to the cloud JSONL.
- Confirmed the cloud lock file was released after checkin.

## Next Version Candidates

- Add lightweight frontend tests for archive/delete/cloud action dispatch.
- Add an end-to-end GUI test harness if Tauri window automation becomes stable in this environment.
- Review whether automatic summaries should run in a fully isolated `CODEX_HOME`.
- Consider a migration note for users who installed `v0.5.0` from local artifacts before the public `v0.5.1` release.
