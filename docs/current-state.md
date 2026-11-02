# 현재 소스 상태.

현재 소스 버전은 v0.5.12이며 Codex CLI 0.166.0의 등록 목록·삭제 API 대응을 포함한다. 추가 의존성은 없다. 공개 릴리스 발행과 산출물 검증은 `superpowers/plans/2026-11-02-v0.5.12-release.md`와 `release-qa.md`를 따른다. 상세 설계는 `superpowers/specs/2026-11-02-paginated-sessions-design.md`, 실행 계획은 `superpowers/plans/2026-11-02-paginated-sessions.md`, 검증과 제한은 `reviews/2026-11-02-paginated-sessions.md`에 기록한다.

등록된 세션은 JSONL 유무와 무관하게 표시하며, 삭제 전 연쇄 영향 범위와 삭제 후 API 부재를 확인한다. SQLite·인덱스를 직접 수정하지 않는다. paginated 파일 백업은 지원하지 않으며 실제 legacy 파일의 ID·형식을 검증한다.

Windows/Codex 0.166.0에서 Rust 122개, Node 18개와 삭제 검사 4개, 프런트 production build, 격리 실제 API 검증을 확인했다. Windows shim 프로세스 트리 종료 테스트는 sandbox의 OS 권한 제한 때문에 정상 Windows 권한에서 실행한다.

네이티브 앱은 정상 Windows 권한과 격리 홈에서 실제 목록·등록 수·미리보기를 화면과 접근성 트리로 확인했다. 별도 브라우저에서는 실제 프런트와 가짜 데이터로 삭제와 실패 흐름을 검증했다. 실제 API로 생성한 paginated 테스트 세션의 Manager CLI 삭제와 새 연결의 부재·재시도도 확인했다. 사용자는 프로젝트의 수정 빌드가 정상 작동한다고 보고했다.

사용자 요청으로 이전 설치본과 바로가기를 제거한 뒤 프로젝트의 release 실행 파일에 연결한 수정 버전 바로가기를 만들었다. 소스와 최종 실행 파일, 사용자 세션·설정은 보존한다. 후속 요청에 따라 v0.5.12 공개 릴리스 발행·소스 통합·원격 반영·중간 산출물 정리를 진행한다.

JSONL 없는 사용자 세션의 실제 삭제, 네이티브 삭제 버튼 클릭, 실제 모델 요약과 실제 계정 간 클라우드 동기화는 수행하지 않았다. 로컬 응답으로 생성한 테스트 세션도 JSONL이 없으면 Codex API가 제외하므로 DB-only 삭제는 단위 테스트와 실제 사용자 목록의 읽기 전용 대조까지 확인했다.
