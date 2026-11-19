# 세션 트리, Guardian 삭제 및 Windows 재개 검증.

## 결과.

- Rust 테스트 144개, Node 테스트 37개, 삭제 흐름 검사 4개가 통과했다. TypeScript 검사와 Vite·Tauri Windows production 빌드가 성공했다.
- 같은 Windows Terminal 버전에서 이스케이프된 명령과 Codex 홈이 PowerShell로 전달됐다. 최종 Windows 실행 파일의 상세 버튼으로 별도 테스트 세션의 실제 `codex.exe resume` 프로세스 시작을 확인했다.
- 최종 Windows 앱에서 초기 접기와 새로고침 후 펼침 유지, 보관된 Guardian의 삭제, 다른 종류의 자손이 있는 Guardian 가지의 보존을 확인했다. Codex CLI 0.170.0과 생성한 별도 홈을 사용했다.
- 독립 리뷰에서 API가 생략한 직접 부모 링크가 삭제 영향 검사에서 빠질 수 있는 경로를 발견했다. 동일 등록 목록의 관계를 보강하고 재리뷰했다.

## 검증 범위.

새로 만든 세션으로 삭제와 실제 재개 프로세스 시작을 확인했다. 모델 요청과 사용자 세션 삭제는 검증 범위에 포함하지 않았다. Windows 실행 파일을 빌드했다.
