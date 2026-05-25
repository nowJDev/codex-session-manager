# Claude Session Manager

A desktop app for managing [Claude Code](https://docs.anthropic.com/en/docs/claude-code) sessions — list, name, resume, and sync across devices.

Built with **Tauri 2 + React + TypeScript + Tailwind + shadcn/ui**. Modern dark theme, works on Windows, macOS, Linux.

![dark UI](docs/screenshot.png)

## Features

- **세션 목록** — `~/.claude/projects/` 아래 모든 세션 표시 (이름·설명·프로젝트·마지막 활동·크기·저장 위치). **WSL 세션 자동 합산** (Windows): 모든 WSL 배포판의 `~/.claude/projects/`를 자동 탐지해서 한 화면에서 같이 보임. 설정에서 추가 외부 폴더도 등록 가능
- **즐겨찾기** — 별 아이콘 토글로 최상단 고정
- **이름/ID 분리** — 사용자 이름이 없으면 "이름 없음" 회색 표시, ID는 항상 별도 컬럼
- **자동 요약 + 이름 생성** — 백그라운드에서 `claude -p --model claude-haiku-4-5` subprocess로 호출, 5개씩 배치로 처리해 빈 description 자동 채움 (API 키 불필요 — claude CLI 자체 인증 사용)
- **무한 루프 방지** — 요약용 호출은 격리 cwd에서 실행, scanner가 격리 폴더 자동 skip
- **빠른 resume** — 더블클릭 또는 "..." 메뉴로 새 터미널에 세션 이어가기. Git Bash / Windows Terminal / PowerShell / cmd / Terminal.app / Custom(자유 입력)
- **Resume 플래그** — `--dangerously-skip-permissions` / `--debug` / `--verbose` 체크박스 + 자유 입력란. 실시간 미리보기
- **이름 변경 / 설명 편집** — `~/.claude-sessions/config.json`에 영구 저장
- **클라우드 동기화** — Google Drive 데스크탑 자동 감지(클릭 한 번), 업로드 후 로컬 jsonl 자동 삭제 (single source of truth), 락 파일로 다중 PC 동시 편집 방지
- **컬럼 폭 드래그 조절** — 헤더 핸들 드래그 + `localStorage` 보존
- **호버 툴팁** — 잘린 셀에 마우스 호버 시 전체 내용 표시
- **검색 & 필터** — 이름 / 설명 / 프로젝트 / 첫 메시지 즉시 검색
- **i18n** — 영어 · 한국어 자동 감지, 설정에서 변경 가능 (재시작 불필요)
- **환경 진단** — 설정 → 진단 실행으로 `claude` CLI / 터미널 감지 결과 확인

## Install

### Pre-built installer (recommended for end users)

Grab the latest installer from [Releases](https://github.com/glowElephant/claude-session-manager/releases):
- Windows: `.msi` or `-setup.exe`
- macOS: `.dmg`
- Linux: `.AppImage` / `.deb`

After install, launch from the Start Menu / Applications. **You still need [Claude Code](https://docs.anthropic.com/en/docs/claude-code) on `PATH`** for the "Open in new terminal" action to work.

### From source

```bash
git clone https://github.com/glowElephant/claude-session-manager.git
cd claude-session-manager
pnpm install

# Dev mode (hot reload, opens a window)
pnpm tauri dev

# Production build (OS-native installer)
pnpm tauri build
```

Installers land in `src-tauri/target/release/bundle/`.

#### Build prerequisites

- **Node.js 18+** and **pnpm**
- **Rust toolchain** (`rustup` — install via <https://rustup.rs>)
- **Tauri 2** platform prerequisites (WebView2 on Windows — Win11 has it; Xcode CLT on macOS; webkit2gtk on Linux) — see <https://v2.tauri.app/start/prerequisites/>

#### Runtime prerequisites

- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) on `PATH` — required to resume sessions **and** for auto-summary (no API key needed; we shell out to `claude -p` which uses its own auth)

## Configuration

Stored at `~/.claude-sessions/config.json`:

```json
{
  "sessions": {
    "abc123-uuid": {
      "name": "my-feature",
      "description": "Working on the new auth flow",
      "autoSummary": "Refactoring auth middleware for compliance",
      "storageType": "local",
      "updatedAt": "2026-04-15T..."
    }
  },
  "settings": {
    "locale": "en",
    "cloudPath": "G:/My Drive/Claude Sessions",
    "extraProjectDirs": ["D:/other/.claude/projects"],
    "wslAutoDetect": true
  }
}
```

> Auto-summary uses the local `claude` CLI as a subprocess (`claude -p --model claude-haiku-4-5`). It uses whatever auth your Claude Code install already has — **no Anthropic API key required**. The legacy `anthropicApiKey` field is ignored.

### Environment variables

| Variable | Purpose |
|---|---|
| `CLAUDE_SESSION_HOME` | Override the home directory used to resolve `~/.claude/projects/` and `~/.claude-sessions/`. Used mainly by the test suite and CLI harness for isolated runs. |
| (settings) `extraProjectDirs` | 추가로 스캔할 `~/.claude/projects/` 경로 목록. Settings → "추가 세션 경로"에서 폴더 추가/제거. |
| (settings) `wslAutoDetect` | Windows 전용. `wsl.exe -l -q`로 배포판을 자동 탐지하여 `\\wsl.localhost\<distro>\home\*\.claude\projects`를 스캔 대상에 포함. 기본값 `true`. |
| `GIT_BASH` | Windows: explicit path to `git-bash.exe`. |
| `WINDOWS_TERMINAL` | Windows: explicit path to `wt.exe`. |

### Terminal selection (Windows)

When you click **Open in new terminal**, the app picks a terminal in this order:

1. The terminal explicitly chosen in **Settings → Preferred terminal**
2. Otherwise, the first available among: Git Bash → Windows Terminal (`wt.exe`) → PowerShell (`pwsh.exe`/`powershell.exe`) → Command Prompt

Each kind is searched via:
- Explicit env var (`GIT_BASH` / `WINDOWS_TERMINAL`)
- Standard install locations (`%ProgramFiles%`, `%ProgramFiles(x86)%`, `%ProgramW6432%`, `%LOCALAPPDATA%`)
- `PATH`
- OS-shipped fallback (e.g. `%SystemRoot%\System32\cmd.exe`)

**Run Settings → Environment diagnostics** to see exactly what the app found on your machine.

On macOS the app uses Terminal.app via `osascript`; on Linux it tries `x-terminal-emulator`, `gnome-terminal`, `konsole`, then `xterm`.

## Cloud sync — how it works

### Setup (one-time)
1. Open **Settings → Cloud folder**. Click **자동 연결** (Auto-connect) — works for Google Drive desktop (English / Korean / 8+ other locales) and macOS CloudStorage. Falls back to **직접 선택** (Browse) for any cloud-synced local folder (OneDrive, Dropbox, etc.).
2. A `Claude Sessions` subfolder is created there. JSONL + `.meta.json` sidecars land in that folder.

### Storage states (since v0.4.4)

Each session shows one of three badges in the Type column:

| Badge | Meaning | Sync button action |
|---|---|---|
| `local` (🗄) | Lives only in `~/.claude/projects/` on this PC | ☁↑ Upload to cloud |
| `synced` (☁) | Exists in BOTH the cloud folder AND local | ↻ Re-sync: overwrite cloud with local (local stays) |
| `cloud only` (☁) | In the cloud folder but not on this PC's local | ☁↓ Check out: copy cloud → local |

The small icon button **inside the Type cell** is the primary one-click sync action. The `⋯` menu has the same action for keyboard / accessibility.

### Workflow — multi-PC

1. **PC-A**: work normally → click ☁↑ in the Type cell → cloud has a copy. **Local stays in place** so claude can keep appending to the same JSONL.
2. **PC-A**: keep working. When you want to share progress → click ↻ to overwrite cloud with the latest local.
3. **PC-B**: open csm → see the same session as `synced` (if PC-B has local too) or `cloud only`. Click ☁↓ to download. Then `claude --resume` (or double-click) — same context, same memory.

⚠️ The lock file (`<id>.lock` in the cloud folder) prevents two PCs from checking out the same session simultaneously. If you see "session locked by hostX" — the other PC currently has it.

### Important behaviour (v0.4.4+)

**"Upload to cloud" never deletes local anymore.** Earlier versions used a "single source of truth" rule that removed the local JSONL after upload — but the active claude session would just create a new JSONL with the same session_id, splitting the data. Now upload = "copy local → cloud, keep local". You're responsible for re-syncing when local has newer changes (just click ↻ again).

No vendor-specific APIs, no OAuth. Works with any sync provider that mounts locally.

## Architecture

```
src-tauri/
├── src/
│   ├── lib.rs              # Tauri command handlers (IPC entry points)
│   ├── main.rs             # GUI binary entry
│   ├── bin/cli.rs          # Headless CLI harness (session-cli)
│   ├── scanner.rs          # ~/.claude/projects/ scanning + JSONL parsing
│   ├── config.rs           # ~/.claude-sessions/config.json read/write
│   ├── cloud.rs            # Cloud folder upload / checkout / checkin
│   ├── terminal.rs         # Terminal detection + per-terminal command builders
│   ├── environment.rs      # Diagnostics: claude CLI + detected terminals
│   ├── resume.rs           # Picks a terminal and spawns it
│   ├── summary.rs          # Anthropic API call for auto-summary
│   └── types.rs            # Session / Config / Settings structs
├── tests/integration.rs    # 21 integration tests against tempfile-isolated home
└── Cargo.toml              # Two binaries: claude-session-manager (GUI) + session-cli

src/
├── App.tsx                 # Top-level layout
├── components/
│   ├── SessionTable.tsx    # Virtualized list with search/filter
│   ├── SessionDetail.tsx   # Right-side detail panel
│   ├── EditDialog.tsx      # Rename / edit description
│   └── SettingsDialog.tsx  # Language / cloud folder / API key
├── lib/ipc.ts              # Typed wrappers around Tauri invoke
└── i18n/{en,ko}.json       # Translations
```

The frontend calls the Rust backend over Tauri's IPC bridge. Each `#[tauri::command]` in `lib.rs` corresponds to one entry in `src/lib/ipc.ts`.

## Development

### Run tests

```bash
cd src-tauri
cargo test --tests
```

21 integration tests cover scanner JSONL parsing, config persistence, settings updates, terminal detection / command building per kind (Git Bash / Windows Terminal / PowerShell / cmd / macOS Terminal / Linux), and the environment diagnostics report. Tests use `CLAUDE_SESSION_HOME` to point at a `tempfile`-managed temp dir, so they never touch your real `~/.claude/`.

### Headless CLI harness

The `session-cli` binary exercises every backend operation without launching the GUI — useful for debugging, scripting, or smoke-testing in CI:

```bash
cd src-tauri
cargo build --bin session-cli

# List all sessions as JSON
./target/debug/session-cli list

# Print resolved paths
./target/debug/session-cli paths

# Persist a name/description
./target/debug/session-cli set-name <session-id> "my-feature"
./target/debug/session-cli set-desc <session-id> "auth refactor"
./target/debug/session-cli delete-meta <session-id>

# Inspect what `Open in new terminal` would run, without spawning anything
./target/debug/session-cli resume-plan <session-id> [cwd]

# Read first N user messages from a JSONL file
./target/debug/session-cli messages <file-path> 5

# Print current config.json contents
./target/debug/session-cli get-config

# Detect claude CLI + available terminals as JSON
./target/debug/session-cli check-env

# Set the preferred terminal (auto / git-bash / wt / powershell / cmd / terminal)
./target/debug/session-cli set-terminal wt
```

Use it against an isolated home to avoid touching real config:

```bash
CLAUDE_SESSION_HOME=/tmp/scratch ./target/debug/session-cli list
```

### Contributing

1. Fork → branch → make changes
2. `cargo test --tests` and `pnpm tauri dev` to verify
3. Submit a PR

## 문제 신고 / Bug reports

세션 실행, 자동 요약 등이 동작하지 않으면 csm이 자동으로 진단 로그를 남깁니다.

**로그 위치**
- Windows: `%USERPROFILE%\.claude-sessions\debug.log`
- macOS/Linux: `~/.claude-sessions/debug.log`

**신고 방법**
1. 앱 → 설정(톱니바퀴 아이콘) → 맨 아래 "문제 신고 / 디버그 로그" → **로그 불러오기** → **마지막 부분 복사**
2. [GitHub Issues](https://github.com/glowElephant/claude-session-manager/issues/new)에 새 이슈 생성
3. 어떤 동작을 시도했는지(예: "세션 더블클릭 → 새 git-bash 창 떴는데 즉시 닫힘") + 복사한 로그 텍스트 + `claude --version` 결과 첨부

claude CLI 자체 에러일 수도 있고 csm spawn 환경 문제일 수도 있어서, **로그가 없으면 원인 파악이 어렵습니다**. 신고 시 꼭 같이 보내주세요.

## License

MIT — see [LICENSE](./LICENSE).
