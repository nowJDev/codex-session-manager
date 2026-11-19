# 세션 삭제 진행 표시 구현 계획.

- [x] `registered_sessions.rs`에서 요청 집계, 계층 중복 제거, 연결 중단과 결과 확인 순서를 검증하는 회귀를 준비한다.
- [x] 삭제 어댑터와 서비스, Tauri 명령에 진행 콜백을 연결한다. 실제 전송 요청과 응답 상태를 채널로 전달한다.
- [x] `session-contract.test.mjs`와 `full-tree-ui.test.mjs`에서 진행 콜백 및 표시 누락을 재현한다.
- [x] `useSessionCommands.ts`에 호출별 상태를 연결하고 `DeleteConfirmDialog.tsx`에서 두 건 이상의 관측값으로 남은 시간을 계산하고 경과 시간을 표시한다.
- [x] Node 회귀와 삭제 흐름 검사 및 TypeScript 검사를 실행한다.
- [x] 최종 Windows 앱에서 지연된 단일·일괄 삭제의 경과 시간, 한국어 남은 시간 및 결과 검증을 확인한다. [검증 기록](../../reviews/2026-11-19-delete-progress.md)을 갱신한다.

- [x] 표시 후속 변경에서 요청 건수, 예상 남은 시간과 경과 시간을 각각 한 줄에 놓는다. 한국어·영어 시간 표기, 빌드와 960×600 렌더링을 검증한다.
