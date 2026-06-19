import { Archive, Cloud, HardDrive, Play } from "lucide-react";
import { Button } from "@/components/ui/button";
import { isAutoSummaryStatus } from "@/lib/sessionDisplay";
import { formatBytes, formatRelativeTime } from "@/lib/utils";
import type { Session } from "@/types";
import type { Locale } from "@/i18n";

interface Props {
  session: Session | null;
  locale: Locale;
  t: (k: string, p?: Record<string, string | number>) => string;
  onResume: (s: Session) => void;
}

export function SessionDetail({ session, locale, t, onResume }: Props) {
  if (!session) {
    return (
      <div className="flex h-full items-center justify-center p-8 text-sm text-muted-foreground">
        {t("action.select")}
      </div>
    );
  }

  const isCloud = session.storageType === "cloud" || session.storageType === "cloud-only";

  return (
    <div className="flex h-full flex-col gap-4 p-6 overflow-y-auto">
      <div>
        <div className="mb-1 flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
          {session.archived ? (
            <Archive className="h-3.5 w-3.5 text-amber-400" />
          ) : isCloud ? (
            <Cloud className="h-3.5 w-3.5 text-sky-400" />
          ) : (
            <HardDrive className="h-3.5 w-3.5 text-emerald-400" />
          )}
          <span>{session.archived ? "archived" : isCloud ? "cloud" : "local"}</span>
          <span>•</span>
          <span>{formatBytes(session.size)}</span>
          {session.lastTimestamp && (
            <>
              <span>•</span>
              <span>{formatRelativeTime(session.lastTimestamp, locale)}</span>
            </>
          )}
        </div>
        <h2 className="text-xl font-semibold">
          {session.name || <span className="font-mono text-muted-foreground">{session.sessionId.slice(0, 12)}</span>}
        </h2>
        {session.name && (
          <p className="mt-0.5 font-mono text-xs text-muted-foreground/70">{session.sessionId}</p>
        )}
      </div>

      <Button onClick={() => onResume(session)} className="self-start">
        <Play className="h-4 w-4" />
        {t("action.resumeNew")}
      </Button>

      <div className="space-y-3 rounded-lg border border-border/60 bg-card/60 p-4 text-sm">
        {session.description && (
          <Field label={t("list.description")} value={session.description} />
        )}
        {session.autoSummary &&
          session.autoSummary !== session.description &&
          !isAutoSummaryStatus(session.autoSummary) && (
            <Field
              label={locale === "ko" ? "자동 요약" : "Auto summary"}
              value={session.autoSummary}
            />
          )}
        <Field label={t("list.project")} value={session.project} mono />
        {session.cwd && <Field label="cwd" value={session.cwd} mono />}
        {session.version && <Field label="version" value={session.version} mono />}
        <Field
          label={t("list.sessionId")}
          value={session.sessionId}
          mono
        />
      </div>

      {session.firstUserMessage && (
        <div className="rounded-lg border border-border/60 bg-card/60 p-4">
          <div className="mb-2 text-xs uppercase tracking-wide text-muted-foreground">
            {t("detail.firstMessage") !== "detail.firstMessage" ? t("detail.firstMessage") : "First message"}
          </div>
          <p className="whitespace-pre-wrap text-sm text-foreground/90">
            {session.firstUserMessage}
          </p>
        </div>
      )}
    </div>
  );
}

function Field({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="text-[11px] uppercase tracking-wide text-muted-foreground">{label}</span>
      <span className={mono ? "font-mono text-xs break-all" : "text-sm"}>{value}</span>
    </div>
  );
}
