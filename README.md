# Codex Session Manager

OpenAI Codex CLI 세션을 데스크톱에서 빠르게 찾고, 이름을 붙이고, 이어서 실행할 수 있게 해주는 Tauri 앱입니다.

[Download latest release](https://github.com/nowJDev/codex-session-manager/releases/tag/v0.5.6) · [Report an issue](https://github.com/nowJDev/codex-session-manager/issues/new)

![Codex Session Manager dark UI](docs/screenshot.png)

## What It Does

- `~/.codex/sessions/YYYY/MM/DD/*.jsonl`과 `~/.codex/archived_sessions`에서 Codex 세션을 스캔합니다.
- 세션에 이름, 설명, 즐겨찾기, 자동 요약을 붙여 다시 찾기 쉽게 만듭니다.
- 이름, 설명, 프로젝트, 세션 ID, 첫 사용자 메시지로 검색하고 컬럼별로 정렬합니다.
- 세션을 더블클릭하거나 메뉴에서 `codex resume <session-id>`로 새 터미널에서 이어갑니다.
- 체크박스로 여러 세션을 선택해 한 번에 삭제할 수 있습니다.
- Google Drive 같은 로컬 동기화 폴더에 세션 JSONL과 메타데이터를 동기화합니다.
- 설치형 앱은 GitHub Release의 `latest.json`을 사용해 자동 업데이트를 확인합니다.

## Latest Release

Current release: [Codex Session Manager v0.5.6](https://github.com/nowJDev/codex-session-manager/releases/tag/v0.5.6).

| Platform | Download |
|---|---|
| Windows | `Codex.Session.Manager_0.5.6_x64-setup.exe`, `Codex.Session.Manager_0.5.6_x64_en-US.msi`, or `Codex.Session.Manager_v0.5.6_x64-portable.zip` |
| macOS Apple Silicon | `Codex.Session.Manager_0.5.6_aarch64.dmg` |
| Linux | `.deb`, `.rpm`, or `.AppImage` from the release assets |

설치형 앱의 자동 업데이트는 `v0.5.4` 이후 빌드부터 동작합니다. `v0.5.3` 이하에서 자동 업데이트 기능을 받으려면 릴리즈 페이지에서 설치 파일을 한 번 직접 받아 설치해야 합니다.

## Requirements

- [OpenAI Codex CLI](https://github.com/openai/codex)가 `PATH`에 있어야 합니다.
- 소스에서 빌드하려면 Node.js 24+, pnpm, Rust toolchain, Tauri 2 platform prerequisites가 필요합니다.

## Core Features

### Session Management

- Codex JSONL의 `session_meta`, `turn_context`, `event_msg`, `response_item` 레코드를 관대하게 파싱합니다.
- 앱 전용 메타데이터는 `~/.codex-sessions/config.json`에 저장합니다.
- Archive / Unarchive 메뉴는 `codex archive <session-id>`와 `codex unarchive <session-id>`를 사용합니다.
- 삭제 액션은 파일 직접 삭제보다 `codex delete <session-id>`를 우선 사용합니다.

### Resume And Terminal

- Windows Terminal, PowerShell, cmd, Git Bash 등을 감지합니다.
- 설정에서 선호 터미널과 custom terminal command를 지정할 수 있습니다.
- `--dangerously-bypass-approvals-and-sandbox`, `--debug`, `--verbose`와 자유 입력 resume flags를 지원합니다.

### Sync And Summary

- Google Drive 등 로컬 동기화 폴더 아래 `Codex Sessions` 폴더를 사용합니다.
- 로컬 세션 업로드, 클라우드 세션 체크아웃, 재동기화를 지원합니다.
- 로컬 `codex exec`를 사용해 이름과 설명을 자동 생성합니다. 이 기능은 Codex CLI 인증 상태에 의존합니다.

## Configuration

앱 설정은 `~/.codex-sessions/config.json`에 저장됩니다.

```json
{
  "sessions": {
    "abc123-uuid": {
      "name": "my-feature",
      "description": "Working on the new auth flow",
      "autoSummary": "Refactoring auth middleware",
      "storageType": "local",
      "updatedAt": "2026-04-15T..."
    }
  },
  "settings": {
    "locale": "ko",
    "cloudPath": "G:/My Drive/Codex Sessions",
    "extraProjectDirs": ["D:/other/.codex/sessions"],
    "excludedScanPaths": ["temporary-bot-session"]
  }
}
```

| Variable | Purpose |
|---|---|
| `CODEX_HOME` | Codex 홈 디렉터리를 직접 지정합니다. 기본값은 `~/.codex`입니다. |
| `CODEX_SESSION_HOME` | 테스트와 CLI 격리 실행용 홈입니다. `CODEX_HOME`이 없을 때 `<home>/.codex`를 사용합니다. |
| `CODEX_CLI` | `codex` 실행 파일 경로를 직접 지정합니다. |
| `GIT_BASH` | Windows에서 `git-bash.exe` 경로를 직접 지정합니다. |
| `WINDOWS_TERMINAL` | Windows에서 `wt.exe` 경로를 직접 지정합니다. |

## Development

```bash
git clone https://github.com/nowJDev/codex-session-manager.git
cd codex-session-manager
pnpm install

# Dev mode
pnpm tauri dev

# Production build
pnpm tauri build
```

Useful checks:

```bash
# Rust tests
cd src-tauri
cargo test -- --test-threads=1

# Frontend build
cd ..
pnpm build
```

`session-cli`는 GUI 없이 백엔드 동작을 확인하는 하네스입니다.

```bash
cd src-tauri
cargo build --bin session-cli
./target/debug/session-cli paths
./target/debug/session-cli list
./target/debug/session-cli check-env
./target/debug/session-cli resume-plan <session-id> [cwd]
```

## Project Structure

```text
src-tauri/
├── src/
│   ├── scanner.rs          # Codex sessions/archived_sessions 스캔 + JSONL 파싱
│   ├── config.rs           # ~/.codex-sessions/config.json 읽기/쓰기
│   ├── cloud.rs            # 클라우드 폴더 업로드/체크아웃/체크인
│   ├── terminal.rs         # 터미널 감지 + codex resume 명령 생성
│   ├── environment.rs      # Codex CLI + 터미널 진단
│   ├── resume.rs           # 터미널 실행
│   ├── summary.rs          # codex exec 기반 자동 요약
│   └── types.rs            # Session / Config / Settings 타입
├── tests/integration.rs
└── Cargo.toml

src/
├── App.tsx
├── components/
├── lib/ipc.ts
└── i18n/{en,ko}.json
```

Installers are written to `src-tauri/target/release/bundle/`.

## Troubleshooting

디버그 로그는 다음 위치에 남습니다.

- Windows: `%USERPROFILE%\.codex-sessions\debug.log`.
- macOS/Linux: `~/.codex-sessions/debug.log`.

문제가 생기면 앱의 설정 화면에서 로그 마지막 부분을 복사한 뒤 [GitHub Issues](https://github.com/nowJDev/codex-session-manager/issues/new)에 등록해 주세요.

## Fork Lineage

이 프로젝트는 [glowElephant/claude-session-manager](https://github.com/glowElephant/claude-session-manager)를 fork해서 Codex용으로 이식한 프로젝트입니다. 첫 Codex 릴리스 라인은 `v0.5.x`입니다.

`v0.5.0`은 최초 포트 태그로 남겨 두었고, GitHub Actions의 Node 24 대응 이후 `v0.5.1`부터 공개 릴리스 워크플로가 안정화되었습니다. `v0.5.6` 이상을 사용하면 새 아이콘, 요약 재시도 수정, 설치형 자동 업데이트, portable Windows asset, 다중 선택 삭제, 개선된 테이블 레이아웃을 사용할 수 있습니다.

## License

MIT. See [LICENSE](./LICENSE).
