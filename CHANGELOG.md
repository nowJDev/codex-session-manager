# Changelog

All notable changes to this project are documented here.

## [0.4.1] — 2026-05-20

### Fixed
- **자동 요약 "program not found" 실패** — Tauri GUI 앱이 사용자 PATH를 못 받는 환경에서 `claude` CLI 찾기 실패하던 문제. `environment::locate_claude()`에 환경변수(`CLAUDE_CLI`) → PATH → 알려진 설치 위치(Anthropic 공식 `~/.local/bin/claude`, npm/pnpm/yarn global, Homebrew) 순으로 검색하는 fallback 추가.
- 자동 요약 호출 시 Windows에서 콘솔 창이 잠깐 떴다 사라지는 깜빡임 (CREATE_NO_WINDOW).

### Added
- **디버그 로그** — `~/.claude-sessions/debug.log`에 세션 resume / 자동 요약 호출 전체 명령과 결과 누적 기록.
- **설정 → "문제 신고 / 디버그 로그" 섹션** — 로그 불러오기, 폴더 열기, 마지막 부분 클립보드 복사, GitHub Issue 링크 안내.
- `get_debug_log_cmd` / `open_debug_log_folder_cmd` Tauri 커맨드.

## [0.4.0] — 2026-05-15

### Added
- **즐겨찾기 토글** — 별 아이콘 클릭으로 즐겨찾기 지정. 목록 최상단 고정 정렬.
- **이름/ID 분리 표시** — 이름 컬럼과 ID 컬럼이 별도. 이름 비어있으면 "이름 없음" 표시.
- **자동 요약 백그라운드 워커** — `claude -p --model claude-haiku-4-5` subprocess로 헤드리스 호출하여 빈 description 세션을 1배치(5개)씩 자동 요약. 시작 시 자동 실행, 한 건 완료할 때마다 `auto-summary-progress` 이벤트로 UI 자동 갱신.
- **배치 요약** — 세션 5개를 한 호출에 묶어 처리. wake-up 비용 줄여 세션당 평균 ~12초 → ~5초.
- **요약 격리 폴더** — `~/.claude-sessions/.summary-runs/`에서 claude 호출하여 새 세션 jsonl이 만들어져도 무한 요약 루프에 빠지지 않도록 scanner가 skip 처리.
- **재생성 모드** — 메뉴 "요약 재생성" 클릭 시 이전 요약을 프롬프트 hint로 넣어 다른 관점으로 다시 요약.
- **resume 플래그** — 설정창에서 `--dangerously-skip-permissions`, `--debug`, `--verbose` 체크박스 + 자유 입력란. 실시간 미리보기 표시.
- **Custom 터미널** — `preferredTerminal: "custom"` + `customTerminalProgram` + `customTerminalArgs` (`{cwd}`, `{id}`, `{flags}`, `{claude_invoke}` 토큰 치환).
- **클라우드 동기화 자동 폴더 감지** — Google Drive 데스크탑 클라이언트의 마운트 위치 자동 탐지 (Backup&Sync, Drive for Desktop 가상 드라이브, macOS CloudStorage). 설정창에 "자동 연결" 버튼 한 번으로 끝.
- **single source of truth 동기화** — 업로드 후 로컬 jsonl 자동 삭제. 클라우드에만 본체 유지. 다른 PC에서 checkout → 작업 → checkin 흐름.
- **락 파일 메커니즘** — 클라우드 폴더에 `<id>.lock` 두어 동시 편집 방지. hostname + acquired_at 기록. 다른 PC가 락 잡고 있으면 checkout 시 에러로 알림.
- **컬럼 폭 드래그 조절** — 헤더 셀 우측 핸들 드래그로 폭 조절. `localStorage`에 저장되어 재실행 시 유지.
- **호버 툴팁** — 잘린 셀(이름·설명·프로젝트·ID·시각·크기)에 마우스 호버 시 전체 내용 표시.

### Changed
- **요약 모델을 Anthropic API 직접 호출에서 `claude -p` subprocess로 교체**. API 키 입력 불필요 — claude CLI가 이미 인증돼 있으면 그대로 활용.
- **`description`과 `autoSummary` 의미 분리** — 자동 생성된 내용은 `autoSummary`에만 저장. `description`은 사용자 수동 편집 시에만 채움. 상세 패널은 두 값이 같으면 한 번만 표시.
- **이름 자동 생성** — 자동 요약과 함께 12자 이내 짧은 제목도 생성.
- **세션 정렬 키** — 즐겨찾기 우선 → 그 다음 last_timestamp 내림차순.
- 설정창에서 Anthropic API 키 입력란 제거 (이제 claude CLI 자체 인증 사용).

### Fixed
- 자동 요약 호출이 만든 jsonl로 인한 무한 재요약 루프 (격리 폴더 + scanner skip 패턴으로 차단).

### New CLI commands (`session-cli`)
- `set-favorite <id> <0|1>` — 즐겨찾기 토글
- `auto-summarize <id>` — 단일 세션 자동 요약
- `auto-summarize-batch [N]` — 빈 description 세션 N개 일괄 처리
- `detect-gdrive` — Google Drive 폴더 감지 결과 JSON
- `connect-gdrive` — Google Drive 자동 연결
- `upload <id>` / `checkout <id>` / `checkin <id>` — 클라우드 동기화 흐름

## [0.3.0] — 2026-05-04

### Added
- **Multi-terminal support on Windows.** Auto-detects Git Bash, Windows Terminal (`wt.exe`), PowerShell (`pwsh`/`powershell`), and Command Prompt. Each terminal gets a tailored launch command (e.g. `wt new-tab -d <cwd> powershell -NoExit -Command claude --resume <id>`).
- **Settings → Preferred terminal** dropdown lets users pin a specific terminal or stick with auto-detect. Persisted in `config.json` as `preferredTerminal`.
- **Settings → Environment diagnostics** — one-click "Run diagnostics" shows whether `claude` is on `PATH` (with version + path) and lists every detected terminal with its resolved location.
- **Warning banner** at the top of the app when `claude` CLI is not found on PATH, with a link to the install guide.
- `check_environment_cmd` Tauri command + `EnvironmentReport` IPC type for the diagnostics feature.
- `terminal.rs` module — `TerminalKind` enum, `detect_all_terminals()`, `pick_terminal()`, `build_resume_command()` (pure, fully unit-testable per terminal kind).
- `environment.rs` module — `check_environment()` resolves `claude` from `PATH` (handles `.exe`/`.cmd`/`.bat`) and runs `claude --version` with a 5-second timeout.
- `WINDOWS_TERMINAL` environment variable — explicit override for `wt.exe` location.
- `session-cli check-env` and `session-cli set-terminal <kind>` subcommands.
- `session-cli` headless binary (`src-tauri/src/bin/cli.rs`) — exposes every backend operation (list, get-config, set-name, set-desc, delete-meta, resume-plan, messages, paths, check-env, set-terminal) for scripting, debugging, and CI.
- 21 Rust integration tests (`src-tauri/tests/integration.rs`) — scanner JSONL parsing, config persistence, settings partial updates, terminal command building per kind, terminal alias parsing, environment diagnostics consistency.
- `CLAUDE_SESSION_HOME` environment variable — overrides the home directory used to resolve `~/.claude/projects/` and `~/.claude-sessions/`. Used by the test suite to run against isolated temp dirs.
- `GIT_BASH` environment variable — explicit override for `git-bash.exe` location on Windows.

### Changed
- **Windows: smarter `git-bash.exe` discovery.** Previously hardcoded to `C:\Program Files\Git\git-bash.exe`. Now searches `GIT_BASH` env var, then `%ProgramFiles%`, `%ProgramFiles(x86)%`, `%ProgramW6432%`, `%LOCALAPPDATA%`, with a fallback to `cmd.exe` if Git is not installed.
- `resume.rs` refactored: terminal selection moved to `terminal.rs`, command construction split into `build_resume_plan()` (pure, testable) and `resume_in_new_terminal()` (spawns the process). Enables headless testing of resume logic without launching real terminals.
- Backend modules in `src-tauri/src/lib.rs` exposed as `pub mod` so external integration tests and the CLI harness can call them directly.

### Fixed
- Adding the `session-cli` binary required `default-run = "claude-session-manager"` in `Cargo.toml`; otherwise `pnpm tauri dev` failed with `cargo run could not determine which binary to run`.

## [0.2.0] — Tauri rewrite

- Full rewrite from terminal UI to a Tauri 2 + React desktop app.
- Adds cloud sync (any locally-mounted cloud folder), auto-summary via Claude Haiku, and i18n (en/ko).

## [0.1.0]

- Initial implementation as a terminal UI.
