// Codex 실행 환경과 사용량 링크를 접을 수 있는 요약으로 표시한다.
import { AlertCircle, CheckCircle2, ChevronRight, Clock3, ExternalLink, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { CodexStatus } from "@/types";

interface Props {
  status: CodexStatus | null;
  loading: boolean;
  error?: string | null;
  t: (k: string, p?: Record<string, string | number>) => string;
  onRefresh: () => void;
  onOpenUsagePage: () => void;
}

function formatCheckedAt(value: string | null | undefined) {
  if (!value) return null;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

export function CodexStatusPanel({ status, loading, error, t, onRefresh, onOpenUsagePage }: Props) {
  const checkedAt = formatCheckedAt(status?.checkedAt);
  const state = loading ? "status.checking" : error ? "status.failed" : !status
    ? "status.unavailable" : status.cliFound ? "status.cliFound" : "status.cliMissing";

  return (
    <details className="group shrink-0 border-t border-border/60 bg-background">
      <summary className="flex min-h-10 cursor-pointer flex-wrap items-center gap-x-2 gap-y-1 px-4 py-2 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring">
        <ChevronRight aria-hidden="true" className="h-4 w-4 shrink-0 group-open:rotate-90" />
        <span className="font-medium">{t("status.title")}</span>
        <span className="inline-flex min-w-0 items-center gap-1 text-xs text-muted-foreground" role="status">
          {loading ? <RefreshCw aria-hidden="true" className="h-3 w-3 shrink-0 motion-safe:animate-spin" />
            : error || status?.cliFound === false ? <AlertCircle aria-hidden="true" className="h-3 w-3 shrink-0 text-amber-400" />
            : status?.cliFound ? <CheckCircle2 aria-hidden="true" className="h-3 w-3 shrink-0 text-sky-400" /> : null}
          {t(state)}
        </span>
      </summary>

      <div className="space-y-3 px-4 pb-4 pt-1">
        {error && !loading && <p className="text-xs text-amber-400 [overflow-wrap:anywhere]">{error}</p>}
        {checkedAt && (
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Clock3 aria-hidden="true" className="h-3 w-3 shrink-0" />
            <time dateTime={status?.checkedAt ?? undefined}>{checkedAt}</time>
          </div>
        )}
        {status?.cliVersion && <p className="text-sm [overflow-wrap:anywhere]">{status.cliVersion}</p>}
        <dl className="grid grid-cols-2 gap-3 text-xs">
          <div className="min-w-0">
            <dt className="font-medium text-muted-foreground">{t("status.model")}</dt>
            <dd className="[overflow-wrap:anywhere]">{status?.model || "—"}</dd>
          </div>
          <div className="min-w-0">
            <dt className="font-medium text-muted-foreground">{t("status.reasoning")}</dt>
            <dd className="[overflow-wrap:anywhere]">{status?.modelReasoningEffort || "—"}</dd>
          </div>
        </dl>
        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            size="sm"
            className="h-auto min-h-8 max-w-full whitespace-normal"
            onClick={onRefresh}
            disabled={loading}
          >
            <RefreshCw aria-hidden="true" />
            {t("status.refresh")}
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="h-auto min-h-8 max-w-full whitespace-normal"
            onClick={onOpenUsagePage}
          >
            <ExternalLink aria-hidden="true" />
            {t("status.openUsage")}
          </Button>
        </div>
        <p className="text-xs text-muted-foreground [overflow-wrap:anywhere]">
          {status?.note || t("status.usageNote")}
        </p>
      </div>
    </details>
  );
}
