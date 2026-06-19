# 자동 업데이트 구현 계획

For agentic workers: REQUIRED SUB-SKILL: executing-plans. 단계는 체크박스 `- [ ]` 문법을 따른다.

## Goal

설치형 Codex Session Manager에서 설정의 업데이트 확인 버튼으로 Tauri 공식 updater를 실행해 새 버전 다운로드, 설치, 재시작까지 진행한다. Portable 사용자는 기존처럼 GitHub 릴리즈 링크를 열어 직접 교체한다.

## Architecture

- Tauri updater 플러그인은 설치형 자동 업데이트만 담당한다.
- `latest.json`과 서명된 updater artifact는 GitHub Release asset으로 배포한다.
- 공개키는 `tauri.conf.json`에 커밋하고, 개인키는 GitHub Actions secret에만 저장한다.
- 설정 UI는 설치형 자동 업데이트 버튼과 portable 링크 버튼을 분리한다.

## Tech Stack

- Tauri 2 updater plugin.
- `@tauri-apps/plugin-updater`.
- `@tauri-apps/plugin-process`.
- GitHub Actions release workflow.

### Task 1

Files:
- Modify `package.json`.
- Modify `src-tauri/Cargo.toml`.
- Modify `src-tauri/src/lib.rs`.
- Modify `src-tauri/capabilities/default.json`.

- [ ] updater/process 플러그인 의존성을 추가한다.
- [ ] Rust 앱 빌더에 updater/process 플러그인을 등록한다.
- [ ] Tauri capability에 updater/process 권한을 추가한다.
- [ ] `cargo check`와 `tsc`로 플러그인 연결을 검증한다.

### Task 2

Files:
- Modify `src-tauri/tauri.conf.json`.
- Modify `.github/workflows/release.yml`.

- [ ] Tauri updater 서명 공개키와 endpoint를 설정한다.
- [ ] `bundle.createUpdaterArtifacts`를 활성화한다.
- [ ] Release workflow에 `TAURI_SIGNING_PRIVATE_KEY`와 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 환경변수를 연결한다.
- [ ] Windows portable zip 업로드는 유지한다.

### Task 3

Files:
- Modify `src/components/SettingsDialog.tsx`.

- [ ] 업데이트 버튼이 Tauri updater `check()`를 먼저 호출하도록 바꾼다.
- [ ] 업데이트가 있으면 `downloadAndInstall()`을 실행하고, 설치 완료 후 재시작을 시도한다.
- [ ] updater가 설정되지 않은 portable/개발 실행에서는 기존 GitHub 릴리즈 링크 안내를 유지한다.
- [ ] 진행 상태 문구를 표시한다.

### Task 4

Files:
- Modify `README.md`.
- Modify `CHANGELOG.md`.

- [ ] 설치형 자동 업데이트와 portable 수동 업데이트 차이를 문서화한다.
- [ ] 검증 후 커밋한다.
