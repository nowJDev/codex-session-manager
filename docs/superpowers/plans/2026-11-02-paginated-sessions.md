# 등록 세션 조회와 검증된 삭제 구현 계획.

> 작업자는 test-driven-development로 각 회귀를 먼저 실패시킨 뒤 최소 구현한다. 독립 조사·리뷰는 서브에이전트로 수행하고 구현 계약이 연결된 변경은 순서대로 통합한다.

**목표.** DB에만 있는 Codex 세션을 표시하고 ID 삭제와 사후 조회로 결과를 확인한다.

**구조.** 기존 application port와 outbound 경계를 유지한다. stdio JSON-RPC 연결은 한 작업 묶음 동안 재사용한다. JSONL은 선택적 정보이며 SQLite는 변경하지 않는다.

**기술.** 기존 Rust/Tauri, React/TypeScript, tokio, serde_json, Node assert 검사를 사용한다.

## 작업 1. Codex API 연결과 페이지 조회.

- [x] `adapters/outbound/codex_app_server.rs`, 해당 테스트와 mod export를 추가한다. 실제 0.166.0 schema에서 thread ID, historyMode, parentThreadId와 source의 형태를 확인한다.
- [x] 가짜 stdio 서버로 초기화·미보관/보관·전체 source·중복 ID·빈 중간 페이지·반복 cursor·중간 실패·transport timeout 회귀를 작성한다. 구현 없는 함수/모듈로 RED를 확인한다.
- [x] `AppServer::connect`, `request`, `list_threads`, `read_excerpt`를 구현한다. 종료 코드는 정리하고 프로세스와 파이프가 남지 않게 한다.
- [x] `cargo test codex_app_server -- --test-threads=1`을 실행하고 실제 사용자 홈은 읽기 전용 thread/list만 비교한다.

## 작업 2. ID 계약과 삭제·등록 목록 통합.

- [x] `types.rs`, `application/ports.rs`, `session_service.rs`, `outbound/session_ports.rs`, `scanner.rs`, `inbound/tauri_commands.rs`, `bin/cli.rs`와 해당 테스트를 전체 읽고 수정한다.
- [x] 파일 없는 등록 thread가 목록에서 유지되는 테스트와 성공 응답 후 잔존·통신 실패·부모/자식 중복·사후 메타 경고 테스트를 먼저 RED로 실행한다.
- [x] Session의 경로·크기를 Option으로 하고 preview/historyMode/registered/hidden 정보를 추가한다. 삭제 target은 ID만 받는다. `prepare_deletion`으로 영향 ID를 계산하고 `delete_sessions`는 승인 범위를 검사한다. 결과는 deleted/alreadyMissing/failed/unconfirmed와 독립 warning을 반환한다.
- [x] Port를 통해 한 묶음 삭제를 adapter에 전달하고 결과 확인 후에만 앱 메타데이터를 정리한다. 파일 스캔은 추가 파일 보강만 유지한다.
- [x] `cargo test -- --test-threads=1`로 기존 fixture와 호출자까지 일관되게 수정한다.

## 작업 3. 기존 요약·클라우드·재개 호환.

- [x] `summary.rs`, `cloud.rs`, `resume.rs`, `terminal.rs`, 자동요약 서비스와 테스트를 읽고 파일 없는 수동 요약·paginated 자동 제외·불완전 백업 차단·재개 home 테스트를 RED로 실행한다.
- [x] 수동요약은 API excerpt를 재사용하고 기존 legacy 파일 추출을 유지한다. paginated 과거 기록을 자동 큐에서 제외한다. cloud 복원은 등록 검증 뒤에만 정상 반환한다. 재개 환경은 API와 일치시킨다.
- [x] 관련 Rust 대상 테스트와 전체 회귀를 실행한다.

## 작업 4. 화면과 IPC.

- [x] `src/types.ts`, `lib/ipc.ts`, `adapters/tauriGateway.ts`, application hook들, `App.tsx`, 목록/상세/확인창, 정렬 로직, 번역과 기존 Node 테스트를 읽는다.
- [x] nullable 크기와 ID 삭제·영향 범위·갱신 오류·unconfirmed 재검사 테스트를 먼저 RED로 실행한다.
- [x] 계약을 연결하고 미리보기·미확인 크기·등록/표시/선택 수·오류를 표시한다. 화면 전체선택은 현재 표시 행으로 명시한다. 미등록 파일의 API 작업과 불완전 클라우드 업로드는 지원하지 않는 이유를 표시한다.
- [x] `node --test tests/*.test.mjs`, `node scripts/check-delete-workflow.mjs`, `npm.cmd run build`로 확인한다.

## 작업 5. 실제 검증과 마무리.

- [x] 격리 CODEX_HOME에서 실제 app-server로 legacy/paginated 세션을 준비하여 조회·본문·삭제·부재 재확인을 검사한다. 실제 사용자 홈에 삭제 요청을 보내지 않는다.
- [x] Rust·Node·production build, Windows release 실행·창 생성과 별도 브라우저의 프런트 상호작용을 확인했다. 네이티브 창 직접 조작은 앱 승인 대기 시간 초과로 수행하지 못했으며 리뷰 문서에 제한을 기록했다.
- [x] 승인 설계 대비 스펙 리뷰 후 품질 리뷰를 수행하고 발견 사항을 테스트와 함께 수정한다.
- [x] README와 현재 작업 검증 기록을 갱신하고 `git diff --check`와 전체 변경을 점검한다.
최종 검증 후 커밋·로컬 main 통합·직접 만든 worktree 정리를 수행한다. 결과는 Git 이력과 최종 작업 보고로 확인한다. 원격 푸시와 설치본 교체는 포함하지 않는다.

## 기준선 증거.

2026-11-02 기준 Rust 단위 78개·아키텍처 14개·응답성 2개, Node 8개·삭제 hook 4개 통과했다. npm production build는 샌드박스 폴더 접근 제한 때문에 한 번 실패한 뒤 승인된 권한으로 통과했다. 기존 Browserslist 경고는 별도 의존성 갱신 범위 밖이다. 관리형 worktree 도구는 현재 태스크의 cwd가 저장소가 아니어서 실패했고 Git worktree fallback으로 sibling 작업 공간을 만들었다. sandbox 사용자 소유권 차이는 전역 설정을 바꾸지 않고 Git 호출의 safe.directory 옵션으로 한정한다.
