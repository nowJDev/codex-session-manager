<p align="center">
  <img src="src-tauri/icons/128x128.png" width="80" height="80" alt="Codex Session Manager 아이콘">
</p>

<h1 align="center">Codex Session Manager</h1>

<p align="center">
  <strong>Codex의 대화를 찾고, 정리하고, 바로 이어가세요.</strong><br>
  부모·서브 세션을 한눈에 관리하는 Windows 데스크톱 앱입니다.
</p>

<p align="center">
  <a href="https://github.com/nowJDev/codex-session-manager/releases/latest"><img src="https://img.shields.io/github/v/release/nowJDev/codex-session-manager?style=flat-square&amp;color=0891b2" alt="최신 GitHub 릴리스"></a>
  <img src="https://img.shields.io/badge/Windows-x64-334155?style=flat-square" alt="Windows x64">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-16a34a?style=flat-square" alt="MIT 라이선스"></a>
</p>

<p align="center">
  <a href="https://github.com/nowJDev/codex-session-manager/releases/latest"><strong>최신 버전 다운로드</strong></a> ·
  <a href="#빠른-시작">빠른 시작</a> ·
  <a href="CHANGELOG.md">변경 내역</a> ·
  <a href="https://github.com/nowJDev/codex-session-manager/issues/new">문제 신고</a>
</p>

## 다운로드

[최신 릴리스](https://github.com/nowJDev/codex-session-manager/releases/latest)에서 사용 방식에 맞는 Windows x64 파일을 선택하세요. **Codex CLI는 별도로 설치해야 합니다.**

전체 세션 트리를 복구한 로컬 소스·검증 빌드는 v0.5.13입니다. 이번 작업에서는 공개 릴리스를 발행하지 않았으며, 기존 v0.5.12 발행 기록과 다운로드 링크를 유지합니다.

- **설치형 · 자동 업데이트 지원.** `*_x64-setup.exe`를 설치하고 실행하세요. 앱에서 업데이트를 확인할 수 있습니다.
- **포터블 · 설치 없이 실행.** `*_x64-portable.zip`의 압축을 풀고 `codex-session-manager.exe`를 실행하세요.

<details>
<summary>자동 업데이트용 파일 안내.</summary>

`latest.json`과 `.sig`는 설치형 앱의 업데이트 검증에 쓰는 보조 파일입니다. 직접 실행할 필요가 없습니다. 포터블은 최신 ZIP을 받아 실행 파일을 교체하세요. 자동 업데이트가 없는 v0.5.3 이하 설치본은 최신 설치 파일을 한 번 직접 받아 설치해야 합니다.

</details>

## 한눈에 보는 세션

![부모 아래 서브 세션을 펼치고 대화 미리보기와 상세 정보를 확인하는 Codex Session Manager 화면](docs/screenshot.png)

<sub>공개용 샘플 데이터로 실행한 v0.5.12 프런트엔드 화면입니다. 실제 사용자 세션과 개인 경로는 사용하지 않았습니다.</sub>

## 작업을 이어가는 데 필요한 기능

- **찾기·정리.** 이름·설명·프로젝트·ID·대화 미리보기로 검색하고, 즐겨찾기와 정렬로 작업을 모읍니다.
- **전체 세션 트리.** 작업 서브·Guardian·요약용 내부 기록을 포함합니다. 가지는 접힌 상태로 시작하며, 모두 펼치기·접기와 개별 펼치기를 지원합니다. 하위 세션을 검색하면 부모도 함께 표시하고, 검색을 지우면 이전 펼침 상태로 돌아갑니다.
- **기간 기준 일괄 삭제.** 최근 1일·7일·30일 중 유지할 기간을 고릅니다. 오래된 등록 세션만 체크하고 삭제 영향 범위를 확인합니다.
- **Guardian 전체 삭제.** 보관된 항목을 포함한 등록 Guardian을 한 번에 삭제할 수 있습니다. 다른 종류의 기록에도 영향이 미치는 가지를 제외하고 확인창에 표시합니다.
- **터미널에서 재개.** 세션을 더블클릭해 `codex resume`으로 이어갑니다. Windows Terminal·PowerShell·cmd·Git Bash와 사용자 지정 터미널을 지원합니다.
- **확인하고 삭제.** 여러 세션을 선택하고 하위 세션까지의 영향 범위를 확인합니다. 확인창에 삭제 요청 건수와 경과 시간을 표시하고, 요청이 여러 건일 때는 남은 시간을 추정합니다. 삭제 후 Codex 등록 목록에서 사라졌는지 다시 검사합니다.
- **대화 요약.** Codex CLI로 대화를 읽어 이름과 설명을 생성합니다. 요약은 Codex CLI의 인증 상태를 사용합니다.
- **폴더 동기화.** 온전한 legacy JSONL과 메타데이터를 Google Drive 같은 로컬 동기화 폴더에 보관하고 다른 PC에서 가져옵니다.

## 빠른 시작

1. **Codex CLI를 준비하세요.** [OpenAI Codex CLI](https://github.com/openai/codex)를 설치하고 `codex --version`이 실행되는지 확인하세요. Manager는 `PATH`의 Codex를 찾습니다.
2. **Manager를 실행하세요.** 설치형 또는 포터블을 실행하면 등록 세션을 불러옵니다. 설정에서 사용할 터미널을 선택할 수 있습니다.
3. **이어갈 작업을 고르세요.** 접힌 트리에서 검색하거나 가지를 펼쳐 원하는 세션을 더블클릭하세요. 이름·설명·즐겨찾기로 다음에 찾기 쉽게 정리할 수 있습니다.

## 사용 전에 알아둘 점

- **현재 검증 환경은 Windows x64와 Codex CLI 0.169.0입니다.** 0.166.0 응답 fixture의 호환 검사도 통과했지만 이번 작업에서 해당 실제 바이너리를 실행하지는 않았습니다. 세션 목록·삭제·대화 조회를 위한 app-server API가 필요합니다.
- **전체 수·표시 수·선택 수를 구분합니다.** 숨김 체크 없이 현재 Codex 홈의 지속 세션을 모두 조회합니다. 검색·접기에 따라 표시 수가 달라지며, 전체 선택은 현재 논리 표시 행 중 등록된 세션 전체를 대상으로 합니다. 접기·검색 후에도 기존 선택은 유지합니다.
- **부모를 삭제하면 하위 세션도 영향을 받습니다.** 확인창에서 전체 영향 범위를 확인하세요. 결과가 미확인이거나 실패한 항목은 재확인·재시도할 수 있습니다.
- **paginated 세션의 클라우드 파일 백업은 지원하지 않습니다.** ID와 형식을 확인한 온전한 legacy JSONL만 동기화합니다. 불완전한 파일로 기존 백업을 덮어쓰지 않습니다.

<details>
<summary>조회·삭제·요약의 동작을 더 알아보기.</summary>

- JSONL 파일이 없는 DB 등록 세션도 읽기 전용 등록 ID 대조와 Codex API의 조상·단일 세션 조회로 확인합니다. 실제 직접 부모 관계로 연결하고, 부모가 없는 서브 세션은 최상위에 표시합니다. 종류·부모·깊이를 확인할 수 있으며 파일 크기를 확인할 수 없으면 `—`로 표시합니다.
- 미보관·보관 세션을 함께 조회하며, 메뉴에서 보관·보관 해제를 실행할 수 있습니다.
- 조회에 실패하면 이전 완전 목록과 선택을 유지하고 오류를 표시합니다. 긴 오류는 높이를 제한한 별도 영역에서 스크롤하고 키보드로 확인할 수 있어 세션 목록을 가리지 않습니다. 첫 조회가 불완전하면 확보한 행과 누락·오류를 표시하며 삭제를 차단합니다. 전체 조회는 60초 예산을 공유하고 지원하지 않는 DB 스키마도 불완전 조회로 처리합니다. API 실패를 파일 강제 삭제로 우회하거나 Codex의 SQLite·세션 인덱스를 직접 수정하지 않습니다.
- 삭제는 ID로 요청하고 미보관·보관 목록에서 부재를 확인합니다. 앱 메타데이터 정리 경고는 삭제 결과와 구분하며, 확인 이후 영향 범위가 늘어나면 다시 확인합니다. 다른 프로세스와의 원자적 스냅샷은 보장하지 않습니다.
- 동기화 폴더 아래 `Codex Sessions`를 사용하며 부모 관계도 보존합니다. 복원은 기존 로컬 파일을 덮어쓰지 않고 Codex 등록을 확인합니다. 등록 미확인 시 복사 파일은 보존하고 오류를 알립니다.
- 수동 요약은 ID로 대화 내용을 읽습니다. 내부 기록의 표시와 자동요약 실행 정책은 별개입니다. Guardian·요약 실행 자신·기존 제외 경로·paginated·사용할 기록이 없는 세션은 자동요약 대상으로 늘어나지 않습니다.
- 기본 재개 명령과 사용자 지정 `{codex_invoke}`는 목록 조회와 같은 CLI 경로·`CODEX_HOME`을 사용합니다. 사용자 지정 명령은 Bash·Cmd·PowerShell 문법을 구분합니다.

</details>

## 설정과 개발

일반 설정은 앱의 설정 화면에서 변경할 수 있습니다. 자세한 설정과 개발 명령은 아래에서 펼쳐 보세요.

<details>
<summary>설정 파일·환경 변수·로그.</summary>

앱 전용 이름·설명·즐겨찾기·설정은 `~/.codex-sessions/config.json`에 저장됩니다. Codex의 기본 저장소는 `~/.codex`입니다. 추가 세션 경로, 선호 터미널과 사용자 지정 재개 명령은 설정 화면에서 지정할 수 있습니다. 스캔 제외 경로 편집 UI는 제거했으며 기존 값은 자동요약 실행 정책에 보존합니다.

| 환경 변수 | 용도 |
| --- | --- |
| `CODEX_HOME` | Codex 홈을 직접 지정합니다. |
| `CODEX_SESSION_HOME` | 테스트·격리 실행용 홈입니다. `CODEX_HOME`이 없으면 이 경로 아래 `.codex`를 사용합니다. 앱 설정도 이 홈 아래에 저장합니다. |
| `CODEX_CLI` | `codex` 실행 파일 경로를 직접 지정합니다. |
| `GIT_BASH` | Windows의 `git-bash.exe` 경로를 지정합니다. |
| `WINDOWS_TERMINAL` | Windows의 `wt.exe` 경로를 지정합니다. |

재개 옵션에는 `--dangerously-bypass-approvals-and-sandbox`, `--debug`, `--verbose`와 자유 입력을 지정할 수 있습니다.

디버그 로그는 Windows의 `%USERPROFILE%\.codex-sessions\debug.log`, macOS/Linux의 `~/.codex-sessions/debug.log`에 저장됩니다. 설정에서 마지막 로그를 복사해 [문제를 신고](https://github.com/nowJDev/codex-session-manager/issues/new)할 수 있습니다. 공개할 로그에 대화·개인 경로가 포함돼 있는지 먼저 확인하세요.

</details>

<details>
<summary>소스 실행·빌드·테스트.</summary>

Node.js 24+, pnpm 11.8.0, Rust와 [Tauri 2 플랫폼 준비 사항](https://v2.tauri.app/start/prerequisites/)이 필요합니다. 데스크톱 앱은 Tauri·Rust·React로 구성됩니다.

```bash
git clone https://github.com/nowJDev/codex-session-manager.git
cd codex-session-manager
pnpm install --frozen-lockfile
pnpm tauri dev
```

Windows 설치 파일을 빌드하려면 다음 명령을 사용하세요. 산출물은 `src-tauri/target/release/bundle/`에 생성됩니다.

```bash
pnpm tauri build --bundles nsis
```

문제에 맞는 검사를 실행할 수 있습니다.

```bash
pnpm build
node --test tests/*.test.mjs
node scripts/check-delete-workflow.mjs

cd src-tauri
cargo test -- --test-threads=1
cargo build --bin session-cli
cd ..
```

`session-cli`는 GUI 없이 백엔드를 확인하는 하네스입니다. `paths`, `list`, `check-env`, `resume-plan <session-id> [cwd]` 명령을 제공합니다. 실제 API 검사는 프로젝트 루트에서 실행하며 모델 요청 없이 임시 세션만 사용합니다.

```bash
python scripts/check-codex-api.py --manager src-tauri/target/debug/session-cli.exe
```

Rust 코드는 [`domain`](src-tauri/src/domain)·[`application`](src-tauri/src/application)·[`adapters`](src-tauri/src/adapters) 경계를 사용합니다. React의 화면 흐름은 [`src/application`](src/application) 훅에서 조율합니다.

</details>

## 문서와 지원

[변경 내역](CHANGELOG.md) · [현재 기능과 검증 범위](docs/current-state.md) · [릴리스 검증 기록](docs/release-qa.md) · [문제 신고](https://github.com/nowJDev/codex-session-manager/issues/new)

## 라이선스와 출처

[MIT 라이선스](LICENSE)로 배포합니다. [glowElephant/claude-session-manager](https://github.com/glowElephant/claude-session-manager)를 fork해 OpenAI Codex CLI용으로 이식한 프로젝트입니다.
