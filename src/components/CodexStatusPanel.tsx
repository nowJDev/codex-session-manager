// 현재 Codex 실행 환경 상태를 오른쪽 패널 하단에 표시한다.
import { openUrl } from "@tauri-apps/plugin-opener";
import { AlertCircle, CheckCircle2, Clock3, ExternalLink, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { CodexStatus } from "@/types";

const CODEX_WEB_URL = "https://chatgpt.com/codex/settings/usage";

interface Props {
  status: CodexStatus | null;
  loading: boolean;
  t: (k: string, p?: Record<string, string | number>) => string;
  onRefresh: () => void;
}

function tx(t: Props["t"], key: string, fallback: string) {
  const value = t(key);
  return value === key ? fallback : value;
}

function formatCheckedAt(value: string | null | undefined) {
  if (!value) return "-";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

function Field({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="min-w-0">
      <div className="text-[10px] uppercase tracking-wide text-muted-foreground">{label}</div>
      <div className="truncate text-xs text-foreground/90">{value}</div>
    </div>
  );
}

export function CodexStatusPanel({ status, loading, t, onRefresh }: Props) {
  const hasCli = !!status?.cliFound;

  async function openUsagePage() {
    await openUrl(CODEX_WEB_URL);
  }

  return (
    <div className="shrink-0 border-t border-border/60 bg-background/70 p-4">
      <div className="mb-3 flex items-center justify-between gap-3">
        <div>
          <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {tx(t, "status.title", "Codex status")}
          </div>
          <div className="mt-0.5 flex items-center gap-1.5 text-xs text-muted-foreground">
            <Clock3 className="h-3 w-3" />
            <span>{formatCheckedAt(status?.checkedAt)}</span>
          </div>
        </div>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={onRefresh}
          disabled={loading}
          title={tx(t, "status.refresh", "Refresh status")}
        >
          <RefreshCw className={loading ? "h-4 w-4 animate-spin" : "h-4 w-4"} />
        </Button>
      </div>

      <div className="mb-3 flex items-center gap-2 rounded-md border border-border/60 bg-card/50 px-3 py-2">
        {hasCli ? (
          <CheckCircle2 className="h-4 w-4 shrink-0 text-sky-400" />
        ) : (
          <AlertCircle className="h-4 w-4 shrink-0 text-amber-400" />
        )}
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">
            {hasCli
              ? status?.cliVersion || tx(t, "status.cliFound", "Codex CLI found")
              : tx(t, "status.cliMissing", "Codex CLI not found")}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <Field label={tx(t, "status.model", "Model")} value={status?.model || "-"} />
        <Field
          label={tx(t, "status.reasoning", "Reasoning")}
          value={status?.modelReasoningEffort || "-"}
        />
      </div>

      <div className="mt-4 space-y-2">
        <div className="text-[10px] uppercase tracking-wide text-muted-foreground">
          {tx(t, "status.usage", "Usage")}
        </div>
        <Button size="sm" className="w-full justify-center shadow-sm shadow-primary/15" onClick={openUsagePage}>
          <ExternalLink className="h-4 w-4" />
          {tx(t, "status.openUsage", "Open usage page")}
        </Button>
      </div>

      <p className="mt-3 line-clamp-2 text-[11px] leading-relaxed text-muted-foreground">
        {status?.note || tx(t, "status.usageNote", "Check remaining usage on the Codex usage page.")}
      </p>
    </div>
  );
}
