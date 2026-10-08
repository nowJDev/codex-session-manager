# 부모·서브 세션 트리 표시.

## 승인 설계.

사용자는 부모 아래에 서브 세션을 묶는 트리 표시를 요청했다. 기준선은 `f037d90`이며 깨끗한 작업 디렉터리에서 `feat/session-tree` 브랜치로 작업한다. 기존 테이블을 사용하고 별도 트리 컴포넌트·라이브러리·저장 설정은 만들지 않는다.

JSONL `source.subagent`와 부모 ID를 읽어 `Session.parentId`, `isSubagent`, `agentNickname`을 전달한다. 부모 ID는 payload 값을 우선하고 thread_spawn 내부 값을 대체값으로 사용한다. forked_from_id만으로 자식으로 판단하지 않는다. 일반 서브에이전트는 사용자 메시지가 없어도 표시하며 guardian 내부 세션의 기존 표시 조건을 유지한다. 클라우드 메타데이터에 관계를 보존하고 오래된 메타데이터는 JSONL에서 관계를 읽는다. 원본 세션과 앱 개인 설정은 수정하지 않는다.

부모와 같은 부모의 자식 각각에 기존 정렬·즐겨찾기 우선순위를 적용한다. 처음에는 자식을 접고 이름 앞 버튼으로 펼친다. 중첩 서브 세션은 깊이별로 들여쓴다. 서브 세션은 배지와 에이전트 이름으로 식별한다. 부모가 없으면 최상위에 남기며 자기 참조·순환 관계에서도 각 행은 한 번만 나타난다. 검색은 일치하는 세션과 존재하는 조상을 남기고 검색 결과는 자동으로 펼친다. 전체 선택은 실제 표시된 행만 대상으로 하며 개별 삭제는 자식으로 전파하지 않는다. 기존 재개·이름 변경·삭제·저장 상태는 그대로 각 세션에 적용한다.

## 실행 계획.

Goal은 부모·서브 관계를 실제 세션 목록에서 탐색하는 것이다. Architecture는 기존 스캐너/클라우드 어댑터에서 관계를 전달하고 기존 테이블 상태 유틸에서 표시 행을 계산한다. Tech Stack은 기존 Rust/Tauri·React/TypeScript다. executing-plans 방식으로 진행하며 독립 백엔드 구현은 적용 지침에 따라 위임한다.

- [x] 백엔드. `scanner.rs`, `types.rs`, `cloud.rs`, `application/session_service.rs`와 `integration_tests.rs`를 변경한다. 관계 없는 일반 세션·일반 서브의 무메시지 표시·부모 우선순위·guardian 기존 정책·클라우드 왕복·이전 메타데이터 fallback 테스트를 먼저 작성한다. `cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1`에서 관계 단언 RED를 확인하고 관계 필드 파싱·직렬화·표시를 구현하여 GREEN을 확인한다.
- [x] 프런트엔드 계산. `src/types.ts`, `src/lib/sessionTableState.ts`와 `tests/session-tree.test.mjs`를 변경한다. `buildSessionTreeRows(sortedSessions, expandedIds)`는 부모를 먼저 반환하며 `filterSessionsWithAncestors(sessions, query)`는 검색에 조상을 포함한다. 테스트는 `assert.deepEqual(rows.map(r => [r.session.sessionId, r.depth]), [["parent", 0], ["child", 1], ["grandchild", 2]])`와 접기·고아·순환·검색을 확인한다. 구현 전 트리 계산 함수가 없는 상태에서 단언 실패를 확인한 뒤 최소 함수를 구현한다. `node --test tests/session-tree.test.mjs`로 검증한다.
- [x] 화면 연결. `SessionTable.tsx`, `useSessionManager.ts`, `App.tsx`, 한·영 번역에 관계 계산과 접기 버튼을 연결한다. 버튼은 키보드로 사용할 수 있고 aria-expanded와 번역된 이름을 제공한다. 검색 중에는 결과 가지가 보이고 검색 결과에서도 사용자가 가지를 다시 접을 수 있다. 전체 선택 대상은 계산된 표시 행이다.
- [x] 검증·리뷰. 전체 Rust/Node 테스트, 기존 삭제 검사, `node node_modules/typescript/bin/tsc`, `node node_modules/vite/bin/vite.js build`, `git diff --check`를 실행한다. 실제 브라우저에서 로컬 세션 스냅샷으로 접기·중첩·검색·정렬·전체 선택을 검증한다. 전체 diff를 리뷰하고 README와 이 문서에 결과를 기록한 뒤 한글 커밋을 남긴다. 배포·원격 푸시는 수행하지 않는다.

## 검증·리뷰 결과.

2026-10-08 Windows에서 검증했다.

- 수정 전 Rust 88개와 기존 Node 새로고침 검사 1개가 통과했다.
- 새 백엔드 관계 테스트 6개가 런타임 단언에서 RED를 보였고 구현 후 전체 Rust 94개(단위 78·아키텍처 14·응답성 2)가 통과했다.
- 트리 계산 테스트 6개는 기능 부재 RED 후 GREEN을 확인했다. 최종 트리 검사 7개(실제 테이블 이벤트 포함)와 새로고침 1개, 기존 삭제 검사 4개가 통과했다. 총 자동 검사 106개가 통과했고 실패·스킵은 없다.
- 타입 검사, Vite production build, MSVC 환경의 desktop/session-cli release build와 git diff --check가 통과했다. 빌드의 기존 Browserslist 데이터 경고와 sandbox 경로 canonicalize 경고는 실행 실패를 유발하지 않았다.
- 빌드한 session-cli로 실제 로컬 세션 104개를 조회했다. 서브 91개의 부모가 모두 연결됐고 같은 실제 데이터의 트리 계산에서 접힌 루트 13개·펼친 행 104개를 중복·누락 없이 확인했다.
- 실제 SessionTable 코드를 실행한 이벤트 검사에서 처음 접힘·펼침·중첩·검색 자동 펼침과 다시 접기·표시된 행만 전체 선택·펼침 버튼의 클릭/더블클릭 전파 차단을 확인했다.
- 전체 변경을 독립 리뷰했으며 확신 높은 결함은 발견되지 않았다. 로컬 기존 브랜치와 사용자 데이터는 보존했다.
- Chrome에서 production 프런트엔드와 release session-cli의 실제 로컬 스냅샷 113개(서브 100개·루트 13개)를 연결해 수동 검증했다. 처음 13행, 부모 펼침 16행, 중첩 펼침 18행과 접기·Enter/Space 키 조작·닉네임 검색 시 조상 유지·검색 중 다시 접기·표시 행만 전체 선택·정렬 후 가족 묶음 유지를 확인했다. 펼침 조작은 세션 선택이나 재개를 유발하지 않았고 브라우저 오류 로그는 없었다.
- 브라우저 검증은 임시 읽기 전용 IPC 연결을 사용했다. Windows native CUA 연결은 사용할 수 없어 Tauri 네이티브 IPC의 화면 검증은 범위에 포함하지 않았다. 원본 세션 데이터는 변경하지 않았다.
- index.html에 남아 있던 Claude Session Manager 제목을 Codex Session Manager로 수정했다. 프런트엔드와 Windows release 실행 파일을 다시 빌드했고 Chrome의 실제 탭 제목을 확인했다.
- 들여쓰기는 이름 열 가독성을 위해 6단계까지만 늘리며 실제 관계·표시·검색은 더 깊은 세션도 지원한다. 펼침 상태는 실행 중 메모리에 유지된다.
