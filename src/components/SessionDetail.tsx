// 세션의 활동과 상세 정보, 고정된 이어하기 동작을 표시한다.
import { Archive, Cloud, HardDrive, Play } from "lucide-react";
import { Button } from "@/components/ui/button";
import { CodexStatusPanel } from "@/components/CodexStatusPanel";
import { isAutoSummaryStatus, sessionName, sessionKindLabels } from "@/lib/sessionDisplay";
import { formatBytes, formatRelativeTime } from "@/lib/utils";
import type { CodexStatus, Session } from "@/types";
import type { Locale } from "@/i18n";

interface Props {
  session: Session | null;
  depth: number;
  parentMissing: boolean;
  branchLastTimestamp?: string | null;
  locale: Locale;
  t: (k: string, p?: Record<string, string | number>) => string;
  codexStatus: CodexStatus | null;
  codexStatusLoading: boolean;
  codexStatusError?: string | null;
  onRefreshCodexStatus: () => void;
  onOpenUsagePage: () => void;
  onResume: (s: Session) => void;
}

export function SessionDetail({
  session,
  depth,
  parentMissing,
  branchLastTimestamp,
  locale,
  t,
  codexStatus,
  codexStatusLoading,
  codexStatusError,
  onRefreshCodexStatus,
  onOpenUsagePage,
  onResume,
}: Props) {
  const isCloud = session?.storageType === "cloud" || session?.storageType === "cloud-only";
  const resumeDisabled = session && (isCloud ? session.historyMode !== "legacy" : !session.registered);
  const resumeReason = resumeDisabled ? t(isCloud ? "action.legacyRequired" : "action.registeredRequired") : undefined;
  const storageLabel = session?.archived ? "list.archived" : isCloud ? "list.cloudOnly"
    : session?.storageType === "synced" ? "list.synced" : "list.localOnly";

  return (
    <div className="flex h-full min-h-0 min-w-0 flex-col">
      {session && (
        <div className="shrink-0 border-b border-border/60 bg-background p-3">
          <Button
            disabled={Boolean(resumeDisabled)}
            title={resumeReason}
            onClick={() => onResume(session)}
            className="h-auto min-h-9 max-w-full whitespace-normal text-left"
          >
            <Play aria-hidden="true" />
            {t("action.resumeNew")}
          </Button>
          {resumeReason && <p className="mt-1 text-xs text-muted-foreground">{resumeReason}</p>}
        </div>
      )}
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="flex min-h-full min-w-0 flex-col">
          {session ? (
            <div className="min-w-0 flex-1 space-y-4 p-4">
              <div>
                <div className="mb-1 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                  {session.archived ? (
                    <Archive aria-hidden="true" className="h-3.5 w-3.5 text-amber-400" />
                  ) : isCloud || session.storageType === "synced" ? (
                    <Cloud aria-hidden="true" className="h-3.5 w-3.5 text-sky-400" />
                  ) : (
                    <HardDrive aria-hidden="true" className="h-3.5 w-3.5 text-emerald-400" />
                  )}
                  <span>{t(storageLabel)}</span>
                  <span aria-hidden="true">·</span>
                  <span>{session.size === null ? "—" : formatBytes(session.size)}</span>
                </div>
                <h2 className="text-xl font-semibold [overflow-wrap:anywhere]">{sessionName(session, t)}</h2>
              </div>

              <div className="min-w-0 space-y-3 rounded-lg border border-border/60 bg-card/60 p-4 text-sm">
                <div className="flex flex-wrap gap-1">{sessionKindLabels(session).map((kind) => (
                  <span key={kind} className="rounded bg-muted px-2 py-1 text-xs">{t("kind." + kind)}</span>
                ))}</div>
                <Field label={t("detail.ownActivity")} value={session.lastTimestamp ? formatRelativeTime(session.lastTimestamp, locale) : "—"} title={session.lastTimestamp ?? undefined} />
                {branchLastTimestamp && (
                  <Field label={t("detail.branchActivity")} value={formatRelativeTime(branchLastTimestamp, locale)} title={branchLastTimestamp} />
                )}
                <Field label={t("detail.parent")} value={session.parentId || "—"} mono />
                <Field label={t("detail.depth")} value={String(depth)} />
                {parentMissing && <p className="text-xs text-amber-400">{t("list.parentMissing")}</p>}
                {session.description && <Field label={t("list.description")} value={session.description} />}
                {session.preview && <Field label={t("detail.preview")} value={session.preview} />}
                {session.autoSummary && session.autoSummary !== session.description && !isAutoSummaryStatus(session.autoSummary) && (
                  <Field label={t("detail.autoSummary")} value={session.autoSummary} />
                )}
                <Field label={t("list.project")} value={session.project} mono />
                {session.cwd && <Field label={t("detail.workingDirectory")} value={session.cwd} mono />}
                {session.version && <Field label={t("detail.version")} value={session.version} mono />}
                <Field label={t("list.sessionId")} value={session.sessionId} mono />
              </div>

              {session.firstUserMessage && (
                <div className="min-w-0 rounded-lg border border-border/60 bg-card/60 p-4">
                  <h3 className="mb-2 text-xs font-medium text-muted-foreground">{t("detail.firstMessage")}</h3>
                  <p className="whitespace-pre-wrap text-sm text-foreground/90 [overflow-wrap:anywhere]">
                    {session.firstUserMessage}
                  </p>
                </div>
              )}
            </div>
          ) : (
            <div className="flex flex-1 items-center justify-center p-4 text-center text-sm text-muted-foreground">
              {t("action.select")}
            </div>
          )}
          <CodexStatusPanel
            status={codexStatus}
            loading={codexStatusLoading}
            error={codexStatusError}
            t={t}
            onRefresh={onRefreshCodexStatus}
            onOpenUsagePage={onOpenUsagePage}
          />
        </div>
      </div>
    </div>
  );
}

function Field({ label, value, mono, title }: { label: string; value: string; mono?: boolean; title?: string }) {
  return (
    <div className="flex min-w-0 flex-col gap-0.5">
      <span className="text-xs font-medium text-muted-foreground">{label}</span>
      <span title={title} className={mono ? "break-all font-mono text-xs" : "text-sm [overflow-wrap:anywhere]"}>
        {value}
      </span>
    </div>
  );
}
