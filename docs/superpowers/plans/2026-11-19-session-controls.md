# 세션 트리, Guardian 삭제 및 Windows 재개 구현 계획.

- [x] `tests/full-tree-ui.test.mjs`와 `tests/session-tree.test.mjs`에서 초기 자손 노출을 재현하고 `SessionTable.tsx`의 기본 펼침 집합을 비운다. 새로고침, 검색, 전체 펼치기·접기를 확인한다.
- [x] `tests/full-snapshot.test.mjs` 및 `registered_sessions.rs`에서 Guardian 버튼 누락과 혼합 자손 선택을 재현한다. 앱의 기존 삭제 흐름에 전체 Guardian 대상과 확인 안내를 연결하고 대상을 다시 검증한다.
- [x] `full_catalog.rs`에서 API가 생략한 직접 관계를 등록 목록으로 보강한다. 보강 전 자손의 부모가 비어 있는 회귀를 확인하고 보강 이후 기존 관계의 우선순위를 검증한다.
- [x] `resume.rs`에서 Windows Terminal에 전달되는 세미콜론을 확인하기 위한 회귀를 작성하고 이스케이프를 적용한다. 직접 PowerShell 실행과 실제 Windows Terminal 파싱을 확인한다.
- [x] Rust·Node 전체 테스트, 삭제 흐름, 타입 검사와 Vite production 빌드를 확인한다.
- [x] 최종 Tauri 실행 파일에서 트리, 재개 프로세스 시작 및 격리 Guardian 삭제를 확인한다. 발견 사항과 재리뷰는 [검증 기록](../../reviews/2026-11-19-session-controls.md)에 남긴다.
