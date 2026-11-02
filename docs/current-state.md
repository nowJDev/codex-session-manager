# 현재 소스 상태.

공개 릴리스는 v0.5.11이며 현재 소스에는 Codex CLI 0.166.0의 등록 목록·삭제 API 대응이 추가됐다. 추가 의존성은 없다. 상세 설계는 `superpowers/specs/2026-11-02-paginated-sessions-design.md`, 실행 계획은 `superpowers/plans/2026-11-02-paginated-sessions.md`, 검증과 제한은 `reviews/2026-11-02-paginated-sessions.md`에 기록한다.

등록된 세션은 JSONL 유무와 무관하게 표시하며, 삭제 전 연쇄 영향 범위와 삭제 후 API 부재를 확인한다. SQLite·인덱스를 직접 수정하지 않는다. paginated 파일 백업은 지원하지 않으며 실제 legacy 파일의 ID·형식을 검증한다.

Windows/Codex 0.166.0에서 Rust 122개, Node 18개와 삭제 검사 4개, 프런트 production build, 격리 실제 API 검증을 확인했다. Windows shim 프로세스 트리 종료 테스트는 sandbox의 OS 권한 제한 때문에 정상 Windows 권한에서 실행한다.

네이티브 앱은 빌드·실행·창 생성까지 확인했고 직접 UI 조작은 승인 대기 시간 초과로 미검증이다. 별도 브라우저에서 실제 프런트와 가짜 데이터로 화면 흐름을 대체 검증했다. JSONL 없는 사용자 세션의 실제 삭제, 실제 모델 요약, 실제 계정 간 클라우드 동기화는 수행하지 않았다. 사용자 세션은 읽기만 했으며 설치본 교체·공개 릴리스·원격 푸시는 하지 않는다.
