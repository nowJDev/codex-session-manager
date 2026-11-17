# 전체 세션 트리 복구 구현 계획.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox tracking.

**Goal:** 모든 지속 세션을 처음부터 표시하고 실제 부모 관계의 트리·검색·삭제 확인을 복구한다.
**Architecture:** Rust의 읽기 전용 등록 ID 대조와 app-server 조회를 공통 스냅샷으로 조립하고 기존 React 테이블에서 전체 펼침·종류·완전성을 표시한다.
**Tech Stack:** Rust/Tauri 2, rusqlite, React 19, TypeScript, Node 테스트.
**Spec:** docs/superpowers/specs/2026-11-17-full-session-tree-design.md.

## Global Constraints.

- 원본 세션 DB·JSONL과 개인 설정은 직접 수정하거나 재등록하지 않는다. DB는 읽기 전용이다.
- 기존 0.166.0과 현재 0.169.0의 조회 형태를 검증하고 숨김 토글 없이 모든 지속 세션을 표시한다.
- 기본 목록 부재만으로 삭제 성공을 판단하지 않는다. 불완전 조회에서는 삭제를 실행하지 않는다.
- 새 외부 라이브러리를 기본적으로 추가하지 않는다. 실제 사용자 삭제·모델 실행·원격 발행을 하지 않는다.
- 소스 전체를 읽고 수정한다. 새 소스 첫 줄에는 한국어 역할 주석을 둔다. 커밋은 검증 후 한글 제목과 빈 줄 및 bullet 본문이다.

## Review Focus.

- 기본 API에 없는 자손·보관 경계를 넘는 자손·부모 누락은 ID 대조와 자손 조회 회귀로 검증한다.
- DB 잠금·스키마 변화·갱신 중 ID 변동은 완전성 false와 삭제 중단으로 검증한다.
- Guardian·자동요약·종전 제외 경로의 기본 표시가 자동요약 확대로 이어지지 않도록 회귀로 검증한다.
- 검색·접기·가상화와 전체 선택의 집합이 어긋나지 않도록 UI 이벤트 검사로 검증한다.
- 실제 Windows 실행 파일이 브라우저 모의 화면과 같은 트리·전체 표시를 제공하는지 수동 검증한다.

## Task 1. 공통 전체 세션 수집과 안전한 실행 정책.

**Files:** src-tauri/src/adapters/outbound/codex_app_server.rs, registered_sessions.rs, src-tauri/src/session_state.rs, types.rs, scanner.rs, cloud.rs, application/ports.rs, application/session_service.rs, inbound 명령·bin/cli.rs, integration_tests.rs 및 해당 단위 테스트. 추가할 작은 수집 모듈은 기존 모듈 경계에 둔다.
**Interfaces:** Session은 sessionKind(main|worker|guardian|otherInternal|unknown), isSummaryRun을 추가하고 hidden을 제거한다. isSubagent는 과거 호환 필드로 유지하되 worker를 가리키도록 한다. 조회 외부 계약은 SessionSnapshot {sessions: Session[], complete: boolean, missingCount: number, errors: string[]}이다. SessionService와 CLI·IPC는 동일한 완전성 정책을 사용한다. parentId는 직접 부모다.

- [x] ID 대조·자손 페이지·조상 필터·고아·중복·보관 혼합·읽기 실패·변동 재시도·삭제 부재 오판 방지와 자동요약 회귀 테스트를 먼저 추가한다.
- [x] 대상 Rust 테스트를 실행해 기존 구현의 누락·hidden 의존을 재현하는 RED를 기록한다.
- [x] 동일 홈의 읽기 전용 등록 목록, 기본 및 조상별 페이지 수집, metadata-only 보충, 1회 변동 재시도, 60초 갱신 예산을 구현한다. 모든 미해결 누락·RPC 실패는 incomplete이며 삭제 전용 호출은 오류로 중단한다.
- [x] Guardian·요약·종전 제외 경로는 표시하되 실행 정책은 별도로 보존한다. 추가 파일 스캔의 숨김 필터를 제거하고 클라우드 메타 종류의 이전 형식 보강을 추가한다.
- [x] Rust 전체 검사·cargo fmt와 Task 1 리뷰를 통과한 뒤 관련 파일만 커밋한다.

## Task 2. 전체 펼침과 종류·완전성 UI.

**Files:** src/types.ts, src/lib/ipc.ts, adapters/tauriGateway.ts, application/useSessionData.ts, useSessionManager.ts, useSessionSelection.ts, useSessionCommands.ts, components/SessionTable.tsx, SessionDetail.tsx, SettingsDialog.tsx 및 하위 설정, App.tsx, lib/sessionTableState.ts, sessionDisplay.ts, i18n/ko.json, en.json, tests/session-tree.test.mjs, session-contract.test.mjs 및 필요 회귀.
**Interfaces:** Task 1의 SessionSnapshot과 종류 값들을 그대로 사용한다. 표시 행과 일치하는 count·선택 범위를 테이블에서 상위로 전달한다. 숨김 UI·필터는 제거하고 자동요약 정책과 혼동하지 않는다.

- [x] 기본 펼침·Guardian 표시·종류 검색·부모 연결·접기 후 refresh·검색 후 복원·표시 행 전체 선택·불완전 조회 삭제 차단을 실제 컴포넌트/훅 테스트로 RED 확인한다.
- [x] 모두 펼치기/접기, 종류 배지, 고아 설명, 상세 부모·종류, 가지 최근 활동 정렬, 정확한 표시·선택 수를 구현한다. 새로고침은 접기와 선택·스크롤을 유지한다.
- [x] 제외 경로 UI를 제거하되 설정 값은 보존한다. 신규 조회 실패 시 이전 완전 스냅샷을 유지하고 첫 실패 때는 partial rows와 경고를 표시한다.
- [x] Node 회귀·삭제 스크립트·타입 검사·production 빌드와 Task 2 리뷰 후 커밋한다.

## Task 3. 실제 데이터·Windows 실행 파일·통합.

**Files:** 필요한 회귀 보강, package.json, src-tauri/Cargo.toml, Cargo.lock, tauri.conf.json(버전 0.5.13), README.md, CHANGELOG.md, docs/current-state.md, docs/reviews/2026-11-17-full-session-tree.md, 본 계획과 승인 설계.

- [x] 실제 DB 읽기 전용 대조로 전체 ID·부모·종류 누락을 확인하고 사용자 기록 삭제는 수행하지 않는다.
- [x] Rust 전체·Node 전체·삭제 회귀·타입·프런트 및 Windows release 빌드·diff 검사를 수행한다.
- [x] 실제 네이티브 앱의 전체 펼침·숨김 체크 부재·종류·중첩·검색·선택·새로고침·수천행 성능을 검증한다. 필요 시 기존 테이블의 렌더링만 최적화하고 테스트한다.
- [ ] 전체 브랜치 코드 리뷰와 중요 발견 수정·재검증 후 문서와 버전을 일치시켜 커밋한다.
- [ ] main에 검증된 브랜치를 통합하고 바탕화면 바로가기의 로컬 실행 파일을 갱신한다. 실행 중인 사용자 앱은 임의 종료하지 않고 새 빌드로 여는 방법을 보고한다. 승인 범위 밖 원격 푸시·공개 릴리스는 하지 않는다.
- [ ] 에이전트가 만든 임시 파일·브랜치·worktree만 참조와 복구 필요를 확인하여 정리한다.
