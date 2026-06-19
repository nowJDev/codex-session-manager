// 세션 목록과 상세 화면에 표시할 설명 텍스트를 정리한다.
import type { Session } from "@/types";

export function isAutoSummaryStatus(value: string | null | undefined): boolean {
  const text = value?.trim() ?? "";
  if (!text) return true;

  return (
    text.startsWith("(") ||
    text.includes("자동 요약 실패") ||
    text.includes("요약 누락") ||
    text.includes("codex CLI") ||
    text.includes("batch file arguments are invalid")
  );
}

export function sessionDescriptionText(session: Session): string {
  if (session.description) return session.description;
  if (!isAutoSummaryStatus(session.autoSummary)) return session.autoSummary ?? "";
  return session.firstUserMessage ?? "";
}
