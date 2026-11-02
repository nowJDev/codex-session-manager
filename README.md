# Codex Session Manager

OpenAI Codex CLI 세션을 데스크톱에서 빠르게 찾고, 이름을 붙이고, 이어서 실행할 수 있게 해주는 Tauri 앱입니다.

[Download latest release](https://github.com/nowJDev/codex-session-manager/releases/tag/v0.5.11) · [Report an issue](https://github.com/nowJDev/codex-session-manager/issues/new)

![Codex Session Manager dark UI](docs/screenshot.png)

## What It Does

- Codex app-server API로 미보관·보관 세션을 조회합니다. JSONL 없이 DB에 저장된 세션도 표시합니다.
- 세션에 이름, 설명, 즐겨찾기, 자동 요약을 붙여 다시 찾기 쉽게 만듭니다.
- 부모 세션 아래에 서브 세션을 트리로 묶고 이름 앞 화살표로 펼치거나 접습니다.
- 이름, 설명, 프로젝트, 세션 ID, 대화 미리보기로 검색하고 컬럼별로 정렬합니다.
- 세션을 더블클릭하거나 메뉴에서 `codex resume <session-id>`로 새 터미널에서 이어갑니다.
- 체크박스로 여러 세션을 선택해 한 번에 삭제할 수 있습니다.
- Google Drive 같은 로컬 동기화 폴더에 세션 JSONL과 메타데이터를 동기화합니다.
- 설치형 앱은 GitHub Release의 `latest.json`을 사용해 자동 업데이트를 확인합니다.

## Latest Release

Current release: [Codex Session Manager v0.5.11](https://github.com/nowJDev/codex-session-manager/releases/tag/v0.5.11).

| Type | Download |
|---|---|
| Windows installer | `Codex.Session.Manager_0.5.11_x64-setup.exe` |
| Windows portable | `Codex.Session.Manager_v0.5.11_x64-portable.zip` |

`latest.json`과 `.sig` 파일은 설치형 앱의 자동 업데이트 검증용 보조 asset입니다.

설치형 앱의 자동 업데이트는 `v0.5.4` 이후 빌드부터 동작합니다. `v0.5.3` 이하에서 자동 업데이트 기능을 받으려면 릴리즈 페이지에서 설치 파일을 한 번 직접 받아 설치해야 합니다.

## Requirements

- [OpenAI Codex CLI](https://github.com/openai/codex)가 `PATH`에 있어야 합니다.
- app-server 목록·삭제·대화 조회 API가 필요하며 Codex CLI `0.166.0`에서 검증했습니다. 호환되지 않는 CLI에서는 조회 오류를 표시하고 파일 스캔 결과로 대체하지 않습니다.
- 소스에서 빌드하려면 Node.js 24+, pnpm, Rust toolchain, Tauri 2 platform prerequisites가 필요합니다.

## Core Features

### Session Management

- API의 모든 지원 세션 종류와 페이지를 조회합니다. 등록된 세션은 파일 유무나 메시지 파싱 결과 때문에 제외하지 않습니다. JSONL 스캐너는 추가 디렉터리와 구형 파일 정보에 사용합니다.
- 서브 세션은 부모 ID로 연결하며 에이전트 이름과 서브 표시를 제공합니다. 부모가 없는 기록은 최상위에 표시합니다.
- 트리는 처음에 접혀 있으며 검색 결과에는 일치한 서브 세션의 부모도 함께 표시하고 자동으로 펼칩니다. 기존 정렬은 부모끼리와 같은 부모의 자식끼리 적용됩니다. 전체 선택은 현재 표시된 등록 세션만 선택합니다.
- 부모 삭제는 Codex의 연쇄 삭제 동작을 따릅니다. 확인창에 선택한 수·전체 영향 수·추가 자손 수를 표시하고, 확인 이후 영향 범위가 늘어나면 새 확인을 요구합니다. 다른 프로세스와의 원자적 스냅샷은 보장하지 않습니다.
- 등록 수·표시 수·선택 수를 구분합니다. 제외 경로와 내부 기록은 표시 필터이며, `숨긴 세션 포함`으로 확인할 수 있습니다. 미확인 파일 크기는 `—`, API 미등록 파일은 제한된 작업으로 표시합니다.
- 클라우드 동기화에서도 부모 관계를 보존합니다. 예전 클라우드 메타데이터는 저장된 JSONL에서 관계를 읽습니다.
- 앱 전용 메타데이터는 `~/.codex-sessions/config.json`에 저장합니다.
- Archive / Unarchive 메뉴는 `codex archive <session-id>`와 `codex unarchive <session-id>`를 사용합니다.
- 삭제는 ID로 `thread/delete`를 요청한 뒤 미보관·보관 목록에서 부재를 확인합니다. 명확한 개별 실패는 나머지 작업과 분리하고, 통신 실패 뒤에는 후속 삭제를 자동 전송하지 않습니다.
- 결과에 삭제·이미 없음·실패·확인 안 됨 수를 표시하고 항상 목록을 갱신합니다. 메타데이터 정리 경고는 삭제 결과와 별도로 표시하며 결과 패널에서 새 조회 후 재시도할 수 있습니다.
- API 실패를 파일 강제 삭제로 우회하지 않습니다. SQLite·세션 인덱스를 직접 수정하지 않고 부재가 확인된 세션의 앱 메타데이터만 정리합니다. 조회 실패 시 이전 목록과 오류 안내를 유지합니다.
- 새로고침과 시작 시 환경·상태 조회는 백그라운드에서 실행하며, 겹치는 목록 갱신 요청을 병합합니다.

### Resume And Terminal

- Windows Terminal, PowerShell, cmd, Git Bash 등을 감지합니다.
- 설정에서 선호 터미널과 custom terminal command를 지정할 수 있습니다.
- 기본 터미널과 custom `{codex_invoke}` 토큰은 목록과 같은 CLI 경로와 `CODEX_HOME`을 사용합니다. custom 명령은 Bash·Cmd·PowerShell 문법을 구분합니다.
- `--dangerously-bypass-approvals-and-sandbox`, `--debug`, `--verbose`와 자유 입력 resume flags를 지원합니다.

### Sync And Summary

- Google Drive 등 로컬 동기화 폴더 아래 `Codex Sessions` 폴더를 사용합니다.
- 로컬 세션 업로드, 클라우드 세션 체크아웃, 재동기화를 지원합니다.
- 파일 동기화는 실제 메타와 ID가 일치하는 온전한 legacy JSONL만 허용합니다. paginated 잔여 파일·미확인 형식은 기존 백업을 덮어쓰지 않습니다. DB 대화의 새 백업 형식은 제공하지 않습니다.
- 클라우드 복원은 기존 로컬 파일을 덮어쓰지 않으며 복사 후 Codex 등록을 확인합니다. 등록을 확인하지 못하면 복사 파일을 보존하고 미확인 상태를 알립니다.
- 로컬 `codex exec`를 사용해 이름과 설명을 자동 생성합니다. 이 기능은 Codex CLI 인증 상태에 의존합니다.
- 수동 요약은 ID로 대화 내용을 읽습니다. 새로 발견된 과거 paginated 세션을 자동 일괄 요약하지 않으며 기존 legacy 자동 요약만 유지합니다.

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

# 삭제·새로고침·세션 트리 프런트엔드 회귀 검사.
node scripts/check-delete-workflow.mjs
node --test tests/*.test.mjs

# 실제 Codex API와 격리된 테스트 세션의 삭제·재조회 검사. 모델 요청 없음.
python scripts/check-codex-api.py --manager <session-cli-path>
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
│   ├── domain/             # 순수 세션, transcript, 버전 비교 규칙
│   ├── application/        # use case 서비스와 포트
│   ├── adapters/           # Tauri inbound adapter와 파일/CLI outbound adapter
│   ├── scanner.rs          # Codex sessions/archived_sessions 스캔 + JSONL 파싱
│   ├── config.rs           # ~/.codex-sessions/config.json 읽기/쓰기
│   ├── cloud.rs            # 클라우드 폴더 업로드/체크아웃/체크인
│   ├── terminal.rs         # 터미널 감지 + codex resume 명령 생성
│   ├── environment.rs      # Codex CLI + 터미널 진단
│   ├── resume.rs           # 터미널 실행
│   ├── summary.rs          # codex exec 기반 자동 요약
│   ├── integration_tests.rs # private infra를 포함한 백엔드 통합 테스트
│   └── types.rs            # Session / Config / Settings 타입
├── tests/
│   ├── architecture.rs     # 헥사고날 경계 회귀 테스트
└── Cargo.toml

src/
├── App.tsx
├── application/             # 프론트엔드 application hook과 Tauri side effect 조율
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

app-server 오류에는 RPC 오류나 제한된 stderr 진단이 포함됩니다. 상태 DB·대화 DB·세션 인덱스는 Codex가 관리하며 Manager가 직접 복구하거나 삭제하지 않습니다. 목록 API가 실패하면 기존 목록이 최신 결과가 아님을 오류 안내로 구분합니다.

## Fork Lineage

이 프로젝트는 [glowElephant/claude-session-manager](https://github.com/glowElephant/claude-session-manager)를 fork해서 Codex용으로 이식한 프로젝트입니다. 첫 Codex 릴리스 라인은 `v0.5.x`입니다.

`v0.5.0`은 최초 포트 태그로 남겨 두었고, GitHub Actions의 Node 24 대응 이후 `v0.5.1`부터 공개 릴리스 워크플로가 안정화되었습니다. `v0.5.10` 이상을 사용하면 새 아이콘, 요약 재시도 수정, 설치형 자동 업데이트, portable Windows asset, 다중 선택 삭제, 대량 삭제 응답성 개선, 개선된 테이블 레이아웃, Codex 상태 패널, usage 페이지 이동 버튼, 헥사고날 구조로 정리된 백엔드 경계와 프론트엔드 어댑터 분리 개선을 사용할 수 있습니다. 이후 릴리스는 Windows 설치 파일과 portable zip 중심으로 배포합니다.

## License

MIT. See [LICENSE](./LICENSE).
