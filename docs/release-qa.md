# Release QA

This document records release-level checks that are not fully covered by unit or integration tests.

## Current Release v0.5.12

2026-11-02에 [v0.5.12](https://github.com/nowJDev/codex-session-manager/releases/tag/v0.5.12)를 최신 정식 릴리스로 발행했다. 태그의 소스 커밋은 `8cfa8099683c8bbed07e5175c4dda63d8e0db21f`이다. [Release 실행 41374695666](https://github.com/nowJDev/codex-session-manager/actions/runs/41374695666)의 Windows 빌드와 공개 전환이 성공했다. 실행 기록은 `superpowers/plans/2026-11-02-v0.5.12-release.md`를 따른다.

- 버전 네 곳(package, Tauri, Cargo, Cargo.lock)이 0.5.12로 일치하고 의존성 lock 항목은 바뀌지 않았다.
- 릴리스 준비 상태에서 Node 18개·삭제 검사 4개와 production build가 통과했다. 기능 코드의 직전 Windows Rust 122개 통과와 사용자 작동 확인은 `reviews/2026-11-02-paginated-sessions.md`에 기록했다.
- 태그 푸시 후 자동 실행이 생성되지 않아 기존 workflow_dispatch로 Release를 실행했다. main CI 자동 실행은 생성되지 않았으며 별도의 CI 통과로 보고하지 않는다.
- 다운로드한 자산 네 개 모두 크기·SHA256이 GitHub digest와 일치했다. 포터블 exe의 PE 버전은 0.5.12이며 SHA256은 `ea96862e2d26609601ca6f3e24a419d1eb0546e009cb052df2a41290f66fa92d`이다.
- updater의 `windows-x86_64`와 `windows-x86_64-nsis`가 v0.5.12 설치 파일을 가리키며 signature가 공개 `.sig`와 같다. 저장소의 공개키로 Ed25519 서명·trusted comment를 검증했고, 변조된 설치 파일은 거부됐다. 검증 구현은 minisign-verify 0.2.5의 공개 테스트 벡터와 변조 거부도 통과했다.
- 공개 `/releases/latest/download/latest.json`이 HTTP 200으로 0.5.12를 반환하며 다운로드한 자산과 해시가 일치했다. 릴리스는 draft·prerelease가 아니며 한국어 변경 내역을 제공한다.
- 바탕화면 수정 버전 바로가기의 실행 파일을 검증한 공개 포터블 exe로 갱신했다. 사용자 앱을 강제 종료하거나 설치 프로그램을 재설치하지 않았다. 이전 로컬 exe와 테스트용 CLI·심볼, 임시 worktree·브랜치·다운로드·프런트 빌드는 정리했다.
- 공개 설치 프로그램의 설치·업데이트 실행 및 공개 exe의 새 GUI 실행은 수행하지 않았다. 사용자 세션·설정은 보존했다. 기존 Actions의 Node 20 런타임 사용 중단 경고가 있었으며 Node 24 강제 실행으로 배포는 성공했다.

| 자산 | SHA256 |
| --- | --- |
| `Codex.Session.Manager_0.5.12_x64-setup.exe` | `084cf34591c81a422d73146695e2f0189c7e009bdb283b74919a2d2955b31fb9` |
| `Codex.Session.Manager_0.5.12_x64-setup.exe.sig` | `2335e037f7303b557fe3b308886bf86f875c225df320c7898801ddccaed73364` |
| `Codex.Session.Manager_v0.5.12_x64-portable.zip` | `c88bdab9bf0405792f7ce31b2d3f4340eb26c4a3af55e28f2c85d3e09dd8d3e3` |
| `latest.json` | `bb72b88c65c4168a24be95cf3a0708f17432d97aa30316dd30b563c9e96bf01c` |

## Previous Release v0.5.11

- Installer release at that time: `v0.5.11`.
- Release page: <https://github.com/nowJDev/codex-session-manager/releases/tag/v0.5.11>.
- `v0.5.0` remains as the first Codex port tag, but the installer release was superseded by `v0.5.1` because GitHub Actions needed Node 24 for `pnpm 11.8.0`.
- `v0.5.11` adds parent/sub-session trees, preserves relationships in cloud metadata, improves deletion recovery and refresh responsiveness, and fixes the browser tab title.

Verification on 2026-10-08:

- Rust 94 tests, frontend tree/refresh 8 tests and deletion regression 4 checks passed.
- TypeScript check and production frontend build passed.
- Chrome UI checks used the production frontend and a real local session snapshot through a temporary read-only IPC bridge. Expand/collapse, nested trees, keyboard input, ancestor-preserving search, visible-row selection and family-preserving sort passed.
- Windows release workflow completed successfully and published v0.5.11 as the latest non-draft, non-prerelease release. Installer, portable ZIP, installer signature and latest.json were uploaded.
- Downloaded asset SHA256 values match GitHub digests. The portable executable reports 0.5.11; updater platforms reference the v0.5.11 installer and carry the same signature as its .sig asset.
- Native Tauri IPC and a v0.5.11 installer smoke test have not been manually verified.

## Installer Smoke Test

Windows release assets to test:

- `Codex.Session.Manager_<version>_x64-setup.exe`.
- `Codex.Session.Manager_v<version>_x64-portable.zip`.
- `latest.json` and the installer `.sig` are updater support assets.

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

현재 등록 세션 목록·삭제는 app-server의 `thread/list`와 `thread/delete`를 사용한다. 삭제 전 부모·하위 세션의 영향 범위를 확인하고 삭제 후 전체 등록 목록에서 부재를 검증한다. SQLite·인덱스를 직접 수정하지 않는다. 기존 CLI archive·unarchive 동작은 유지한다.

현재 삭제 회귀는 등록 부재, 남아 있는 등록의 실패 판정, 전송 불확실성과 검증 실패를 포함한다. 상세 검증은 `reviews/2026-11-02-paginated-sessions.md`를 따른다.

다음은 2026-06-19 당시 CLI 기반 경로의 과거 검증 기록이다.

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
