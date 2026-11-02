# README 제품 소개형 개편 설계.

사용자는 로고·짧은 소개·버전 배지, 최신 다운로드, 화면 예시, 핵심 기능, 3단계 시작 안내 순서의 제품 소개형 구성을 승인했다. 한국어 중심으로 쓰고 고급 설정·개발 명령을 접으며 기능 제한·MIT·원작자 표기를 보존한다.

## 범위와 구조.

`README.md`를 첫 방문자의 다운로드와 첫 실행을 우선하는 문서로 정리한다. 최신 버전 링크는 `/releases/latest`를 사용해 v0.5.11 고정을 제거한다. Release·Windows x64·MIT 배지만 사용하며 실행되지 않은 CI의 성공 배지를 추가하지 않는다.

상단에는 기존 앱 아이콘, 한 줄 소개, 다운로드와 주요 문서 링크를 둔다. 핵심 기능은 찾기·트리·재개·삭제·요약·legacy 동기화로 짧게 비교한다. Codex CLI 0.166.0 검증 환경과 paginated 백업 제한, 하위 세션 연쇄 삭제는 사용자가 찾기 쉬운 위치에 둔다.

다운로드와 핵심 기능은 작은 화면에서 한글 열이 세로로 쪼개지지 않도록 짧은 목록으로 표시한다. 환경 변수 같은 고급 정보의 표는 접는 섹션 안에 둔다.

`docs/screenshot.png`는 현재 production 프런트엔드를 공개용 샘플 데이터로 실행해 촬영한다. 부모·하위 세션과 미리보기를 보여 주며 실제 사용자 데이터나 경로를 사용하지 않는다. 브라우저 프런트엔드 예시임을 캡션에 명시한다. 제품 소스·설정·버전·릴리스 자산은 바꾸지 않는다.

## 검증과 완료 조건.

GitHub의 Markdown 렌더러와 실제 저장소 화면으로 제목·표·접기·링크·이미지를 확인한다. 상대 링크·이미지·섹션 앵커, 최신 다운로드와 배지 응답, 개인 정보 미포함을 검사한다. 문서 변경에 Rust 빌드나 제품 회귀를 재실행하지 않는다. 검증 후 커밋·main 통합·푸시하고 직접 만든 서버·탭·worktree·중간 빌드를 정리한다. 기존 공개 exe와 바탕화면 바로가기는 보존한다.

## 기준 자료.

- [GitHub README 안내](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes).
- [GitHub 접는 섹션 안내](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/organizing-information-with-collapsed-sections).
- [Shields GitHub Release 배지](https://shields.io/badges/git-hub-release).

승인한 구성과 보존 범위가 일치하고 미정 항목·상충 요구가 없음을 자체 검토했다.
