# Codex 등록 세션 조회와 검증된 삭제 설계.

2026-11-02 사용자가 대화에서 상세 설계를 승인하고 구현을 요청했다. 기준선은 `75474d3`, 지원 검증 대상은 Codex CLI 0.166.0과 Windows다.

## 목표와 경계.

JSONL 유무와 무관하게 Codex API에 등록된 세션을 표시하고 ID로 삭제한다. 미보관·보관 및 모든 지원 source를 끝까지 페이지 조회한다. 누락·파일 파싱 실패를 빈 목록으로 오인하지 않는다. SQLite와 session_index는 직접 변경하지 않는다. 새 전역 초기화나 paginated 백업 형식은 추가하지 않는다.

## 구조와 데이터.

기존 React → Tauri → SessionService → Port 경계를 유지하고 outbound에 stdio app-server adapter를 추가한다. CLI 경로와 CODEX_HOME은 조회·명령·재개에서 통일한다. 작업 한 묶음은 연결을 재사용하고 초기화, 응답 ID, stderr 배출, 제한 시간과 자기 자식 프로세스 정리를 처리한다. 새 의존성은 추가하지 않는다.

Session은 유지하되 filePath·size를 nullable로 바꾸고 preview, historyMode, registered와 표시 제외 정보를 추가한다. path는 실제 파일이 있을 때만 채우며 paginated 잔여 JSONL을 전체 대화로 취급하지 않는다. API 목록의 존재 판단과 화면 제외 설정을 분리한다. 추가 디렉터리의 미등록 파일은 registered=false로 유지한다. 파일형 기록과 cloud-only는 API 삭제 대상으로 취급하지 않는다.

## 사용자 흐름.

목록은 미보관·보관의 페이지를 모두 읽고 ID로 합친 뒤 앱 이름·설명·즐겨찾기를 보강한다. 목록 조회 중 한 페이지라도 실패하면 이전 목록을 유지하고 갱신 실패를 표시한다. 화면에는 로컬 등록·필터 결과·선택 수를 구분한다. preview는 첫 사용자 메시지와 구분하고 알 수 없는 크기는 대시로 표시한다.

삭제 전 선택 ID와 연쇄 하위 ID를 조회하여 확인창에 영향 범위를 표시한다. 선택 항목과 승인된 영향 ID를 서버에서 다시 확인하고 범위가 늘면 재확인을 요구한다. 중복 루트 요청은 줄이며 독립 항목의 실패는 격리한다. 요청과 사후 등록 목록 조회를 통해 deleted·alreadyMissing·failed·unconfirmed를 구분한다. 조회 실패와 프로토콜 오류는 부재로 처리하지 않는다. 메타데이터 정리 실패는 경고로 분리한다. 연결이 깨진 뒤 결과 불명 삭제를 자동 재전송하지 않는다. 삭제의 하위 영향은 실행 시점의 Codex 동작이며 타 프로세스 변경에 대한 원자적 스냅샷을 보장하지 않는다.

## 연관 기능.

수동 요약은 ID로 API의 userMessage·agentMessage를 제한된 양만 읽어 기존 요약 실행에 전달한다. 새로 발견한 paginated 과거 세션은 자동 요약 큐에서 제외한다. legacy 파일만 기존 자동 요약을 유지한다. 재개는 동일 CLI·CODEX_HOME을 사용한다. cloud 업로드는 완전한 legacy 파일만 허용하고 paginated·형식 미확인 파일을 거부한다. 복원은 파일 복사 후 등록 여부를 확인하며 등록되지 않으면 미확인을 보고한다. 기존 백업과 개인 데이터는 보존한다.

## 검증과 완료 조건.

가짜 stdio 서버로 handshake, 페이지, 반복 cursor, 중복 ID, 중간 오류, malformed 응답, timeout, stderr, 삭제 결과를 검증한다. 서비스·프런트 회귀는 nullable 경로·정렬·검색·연쇄 삭제·부분 실패·결과 미확인·메타 경고·갱신 실패를 포함한다. Rust 전체 테스트, Node 전체 검사, TypeScript와 production build, Windows 앱을 검증한다. 실제 CLI schema 및 읽기 조회와 일치하는지 확인한다. 실제 삭제는 격리 CODEX_HOME에 만든 세션만 사용한다. 사용자 저장소는 읽기 비교만 한다. 검증 후 전체 리뷰, 한글 커밋, 로컬 main 통합과 임시 작업 공간 정리를 수행하며 원격 배포·푸시는 하지 않는다.
