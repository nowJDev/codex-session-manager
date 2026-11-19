# 세션 매니저 UI·사용성 구현 계획

목표는 승인된 삭제 안전성, 목록, 오류 처리, 설정 설계를 구현하는 것이다. [설계](../specs/2026-11-19-ui-usability.md)와 기존 React·TypeScript·Tauri·Rust 계약을 따른다.

- [x] 삭제 확인과 재개 안전성. `useSessionCommands.ts`, `App.tsx`, `DeleteConfirmDialog.tsx`의 계획 ID와 화면을 연결한다. 먼저 숨겨진 선택 재계산, 재승인과 제목 외 더블클릭 동작을 재현한다.
- [x] 목록과 상세. `SessionTable.tsx`, `SessionDetail.tsx`, `sessionTableState.ts`에서 정렬과 제목의 키보드 동작 및 가지 시각을 검증하고 고정 열과 좁은 패널을 적용한다.
- [x] 조회·편집 상태. `useSessionData.ts`, `EditDialog.tsx`, 공통 번역·스타일에서 초기/빈 상태와 실패 후 입력 유지, IME 조합을 검증한다.
- [x] 설정과 동기화. `SettingsDialog.tsx`, 설정 액션·Tauri·서비스에서 취소와 갱신으로 편집값 또는 저장 경로가 바뀌지 않는지 검증한다.
- [x] 변경 전후 Node·Rust 회귀, 빌드, 격리한 Windows 실행과 전체 diff를 확인한다.

[검증과 독립 검토](../../reviews/2026-11-19-ui-usability.md)에 결과를 기록했다.
