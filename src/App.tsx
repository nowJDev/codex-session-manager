// 세션 목록, 상세 및 작업 상태 화면을 조립한다.
import { useEffect, useMemo, useRef, useState } from "react";
import { AlertTriangle, RefreshCw, Search, Settings as SettingsIcon, Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SessionTable } from "@/components/SessionTable";
import { SessionDetail } from "@/components/SessionDetail";
import { EditDialog } from "@/components/EditDialog";
import { DeleteConfirmDialog } from "@/components/DeleteConfirmDialog";
import { SettingsDialog } from "@/components/SettingsDialog";
import { useSessionManager } from "@/application/useSessionManager";
import { sessionName } from "@/lib/sessionDisplay";
import type { Session } from "@/types";

const COMPACT_LAYOUT = "(width < 1120px)";

function App() {
  const manager = useSessionManager();
  const {
    t, query, setQuery, handleVisibleRowsChange, displayedCount, visibleSelectedCount, workerCount, total,
    selectedDepth, selectedParentMissing, selectedBranchLastTimestamp, branchActivities, canDelete, snapshotStale, missingCount,
    loading, initialLoading, refreshError, locale, config, selectedId, setSelectedId, editMode, editTarget,
    settingsOpen, setSettingsOpen, codexCliMissing, codexStatus, codexStatusLoading, codexStatusError,
    selectedForDeleteIds, pendingDelete, deleting, preparingDelete, deleteReport, dismissDeleteReport,
    filtered, selected, selectedForDeleteCount, localCount, cloudCount, refresh, refreshCodexStatus,
    settingsActions, handleResume, handleDelete, handleBulkDelete, handleDeleteByAge, guardianCount,
    handleDeleteGuardians, handleRetryFailedDelete, confirmDelete, cancelDelete, handleUnselectHiddenDelete,
    handleRecheckDelete, handleToggleArchive, handleToggleCloud, handleGenerateSummary, handleToggleFavorite,
    handleToggleSelected, handleToggleVisibleSelection, openEdit, submitEdit, closeEdit, notice, dismissNotice,
  } = manager;
  const [retentionDays, setRetentionDays] = useState(7);
  const [compact, setCompact] = useState(() => typeof window !== "undefined" && !!window.matchMedia?.(COMPACT_LAYOUT).matches);
  const [detailOpen, setDetailOpen] = useState(false);
  const detailClose = useRef<HTMLButtonElement>(null);
  const detailOpener = useRef<HTMLElement | null>(null);
  const search = useRef<HTMLInputElement>(null);
  const deleteOpener = useRef<{ control: HTMLElement | null; rowId?: string; index: number } | null>(null);

  useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return;
    const media = window.matchMedia(COMPACT_LAYOUT);
    const update = (event: MediaQueryListEvent) => setCompact(event.matches);
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);
  useEffect(() => {
    if (typeof document !== "undefined") document.documentElement.lang = locale;
  }, [locale]);
  useEffect(() => {
    if (compact && detailOpen && selectedId) detailClose.current?.focus();
    if (!selectedId && detailOpen && compact) setDetailOpen(false);
  }, [compact, detailOpen, selectedId]);

  function selectSession(session: Session) {
    if (typeof document !== "undefined") detailOpener.current =
      document.querySelector<HTMLElement>(`[data-session-id="${CSS.escape(session.sessionId)}"] [data-session-title]`);
    setSelectedId(session.sessionId);
    setDetailOpen(true);
  }

  function closeDetail() {
    setDetailOpen(false);
    if (detailOpener.current?.isConnected) detailOpener.current.focus();
    else search.current?.focus();
  }

  function withDeleteFocus(action: () => void, sessionId?: string) {
    if (typeof document !== "undefined") {
      const control = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      const rowId = sessionId ?? control?.closest<HTMLElement>("[data-session-id]")?.dataset.sessionId;
      const rows = [...document.querySelectorAll<HTMLElement>("[data-session-id]")];
      const index = rows.findIndex((row) => rowId ? row.dataset.sessionId === rowId : selectedForDeleteIds.has(row.dataset.sessionId ?? ""));
      deleteOpener.current = { control, rowId, index };
    }
    action();
  }

  function returnDeleteFocus() {
    const opener = deleteOpener.current;
    const removed = new Set(deleteReport?.results.filter((item) => item.status === "deleted" || item.status === "alreadyMissing").map((item) => item.sessionId));
    if (opener?.control?.isConnected && opener.control !== document.body && !opener.control.matches(":disabled") && !removed.has(opener.rowId ?? "")) {
      opener.control.focus();
      return;
    }
    const rows = [...document.querySelectorAll<HTMLElement>("[data-session-id]")];
    const isRemaining = (row: HTMLElement) => !removed.has(row.dataset.sessionId ?? "");
    const row = rows.find((entry) => entry.dataset.sessionId === opener?.rowId && isRemaining(entry))
      ?? rows.slice(Math.max(opener?.index ?? 0, 0)).find(isRemaining)
      ?? rows.reverse().find(isRemaining);
    (row?.querySelector<HTMLElement>("[data-session-title]") ?? search.current)?.focus();
  }

  const byId = useMemo(
    () => new Map((manager.allSessions ?? filtered).map((session) => [session.sessionId, session])),
    [manager.allSessions, filtered],
  );
  const preview = useMemo(() => {
    if (!pendingDelete) return { rows: [], hidden: [] as string[], descendants: 0 };
    const direct = new Set(pendingDelete.sessions.map((target) => target.sessionId));
    const filteredIds = new Set(filtered.map((session) => session.sessionId));
    const visible = new Set((manager.visibleSessions ?? []).map((session) => session.sessionId));
    const hidden: string[] = [];
    const name = (id: string) => {
      const session = byId.get(id);
      return session ? sessionName(session, t) : t("delete.unknownTitle", { id });
    };
    const rows = pendingDelete.affectedIds.map((id) => {
      const session = byId.get(id);
      let reason: string;
      if (direct.has(id)) {
        reason = t(pendingDelete.retentionDays !== undefined ? "delete.selectedByAge"
          : pendingDelete.sessions[0]?.guardianOnly ? "delete.selectedGuardian" : "delete.direct");
        if (pendingDelete.fromSelection && !visible.has(id)) {
          hidden.push(id);
          reason += " · " + t(query.trim() && !filteredIds.has(id) ? "delete.hiddenBySearch" : "delete.hiddenByCollapse");
        }
      } else {
        const seen = new Set([id]);
        let parent = session?.parentId;
        while (parent && !direct.has(parent) && !seen.has(parent)) {
          seen.add(parent);
          parent = byId.get(parent)?.parentId;
        }
        reason = parent && direct.has(parent) ? t("delete.includedByParent", { name: name(parent) }) : t("delete.includedDescendant");
      }
      return { id, title: name(id), project: session?.project || "—", reason };
    });
    return { rows, hidden, descendants: rows.length - direct.size };
  }, [pendingDelete, filtered, manager.visibleSessions, byId, t, query]);

  const results = deleteReport?.results ?? [];
  const deleteFailures = results.filter((item) => item.status === "failed" || item.status === "unconfirmed" || item.warning);
  const skippedGuardians = pendingDelete?.skippedGuardianCount ?? 0;
  const skippedByAge = pendingDelete?.skippedAgeCount ?? 0;
  const retention = pendingDelete?.retentionDays;
  const cutoff = pendingDelete?.requestedTargets?.find((target) => target.activityBefore !== undefined)?.activityBefore;
  const directCount = pendingDelete?.sessions.length ?? 0;
  const impact = pendingDelete?.affectedIds.length ?? 0;
  const ageNotes = [
    retention !== undefined && t("delete.ageRetention", { days: retention }),
    cutoff !== undefined && t("delete.cutoff", { date: new Date(cutoff * 1000).toLocaleString(locale) }),
    skippedByAge > 0 && t("delete.ageSkipped", { count: skippedByAge }),
  ].filter(Boolean).join(" ");
  const ageDeleteDisabled = deleting || preparingDelete || !canDelete || localCount === 0;
  const hiddenCount = selectedForDeleteCount - visibleSelectedCount;
  const noticeFailed = notice?.kind === "resumeFailed" || notice?.kind === "folderSyncFailed";

  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      {codexCliMissing && (
        <div className="flex items-center gap-2 border-b border-amber-500/40 bg-amber-500/10 px-4 py-2 text-xs text-amber-300">
          <AlertTriangle className="h-4 w-4 shrink-0" />
          <span className="flex-1">{t("warning.codexCliMissing")}</span>
          <a href="https://github.com/openai/codex" target="_blank" rel="noreferrer" className="underline">{t("warning.codexCliInstall")}</a>
        </div>
      )}
      <header className="shrink-0 border-b border-border/60">
        <div className="flex flex-wrap items-center gap-2 px-4 py-2">
          <h1 className="text-base font-semibold">{t("app.title")}</h1>
          <div className="relative min-w-36 flex-1">
            <Search aria-hidden="true" className="pointer-events-none absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input ref={search} value={query} aria-label={t("action.search")} placeholder={t("action.search")}
              onChange={(event) => setQuery(event.target.value)} className="h-9 pl-8 pr-9" />
            {query && <Button variant="ghost" size="icon" className="absolute right-0 top-0 h-9 w-9" aria-label={t("action.clearSearch")}
              onClick={() => { setQuery(""); search.current?.focus(); }}><X aria-hidden="true" /></Button>}
          </div>
          <Button variant="ghost" size="icon" onClick={refresh} disabled={loading} title={t("action.refresh")} aria-label={t("action.refresh")}>
            <RefreshCw className={loading ? "animate-spin" : ""} />
          </Button>
          <Button variant="ghost" size="icon" onClick={() => setSettingsOpen(true)} title={t("settings.title")} aria-label={t("settings.title")}><SettingsIcon /></Button>
        </div>
        <div role="group" aria-label={t("list.actions")} className="flex flex-wrap items-center gap-2 border-t border-border/60 px-4 py-2">
          <span className="mr-auto text-xs text-muted-foreground">
            {!initialLoading && t("list.counts", { total, workers: workerCount, registered: localCount, displayed: displayedCount, selected: selectedForDeleteCount, selectedVisible: visibleSelectedCount })}
            {!initialLoading && cloudCount > 0 && " · " + t("list.cloudCount", { count: cloudCount })}
            {hiddenCount > 0 && " · " + t("delete.hiddenSelected", { count: hiddenCount })}
          </span>
          <div className="flex items-center gap-1.5">
            <select aria-label={t("action.retentionDays")} value={retentionDays}
              onChange={(event) => setRetentionDays(Number(event.currentTarget.value))} disabled={ageDeleteDisabled}
              className="h-8 rounded-md border border-input bg-background px-3 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50">
              {[1, 7, 30].map((days) => <option key={days} value={days}>{t("action.keepRecentDays", { days })}</option>)}
            </select>
            <Button variant="outline" size="sm" title={t("action.deleteByAge")} disabled={ageDeleteDisabled}
              onClick={() => withDeleteFocus(() => handleDeleteByAge(retentionDays))}>{t("action.deleteByAge")}</Button>
          </div>
          {guardianCount > 0 && <Button variant="outline" size="sm" onClick={() => withDeleteFocus(handleDeleteGuardians)}
            disabled={deleting || preparingDelete || !canDelete} title={t("action.deleteGuardians", { count: guardianCount })}>
            <Trash2 />{t("action.deleteGuardians", { count: guardianCount })}
          </Button>}
          {selectedForDeleteCount > 0 && <Button variant="destructive" size="sm" onClick={() => withDeleteFocus(handleBulkDelete)}
            disabled={deleting || preparingDelete || !canDelete} title={t("action.deleteSelected", { count: selectedForDeleteCount })}>
            <Trash2 />{t("action.deleteSelected", { count: selectedForDeleteCount })}
          </Button>}
        </div>
      </header>

      <div className="max-h-[32vh] shrink-0 overflow-auto empty:hidden">
        {refreshError && <section className="border-b border-border bg-card px-4 py-2 text-sm">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p role="alert" className="text-[hsl(var(--error))]">
              {t(snapshotStale ? "warning.staleSnapshot" : "warning.partialSnapshot", { missing: missingCount })}
            </p>
            <Button variant="outline" size="sm" onClick={refresh} disabled={loading}>{t("common.retry")}</Button>
          </div>
          <details className="mt-1">
            <summary className="cursor-pointer">{t("common.technicalDetails")}</summary>
            <p tabIndex={0} className="mt-1 max-h-28 overflow-auto whitespace-pre-wrap break-all text-xs text-muted-foreground">{refreshError}</p>
          </details>
        </section>}
        {loading && !initialLoading && <p role="status" className="border-b border-border px-4 py-2 text-sm">{t("list.refreshing")}</p>}
        {preparingDelete && !pendingDelete && <p role="status" className="border-b border-border px-4 py-2 text-sm">{t("delete.preparing")}</p>}
        {notice && <section className="border-b border-border px-4 py-2 text-sm">
          <div className="flex flex-wrap items-center gap-2">
            <p role={noticeFailed ? "alert" : "status"} className={noticeFailed ? "mr-auto text-[hsl(var(--error))]" : "mr-auto"}>
              {t("notice." + notice.kind, { name: sessionName(notice.session, t) })}
              {!noticeFailed && notice.kind === "folderCopied" && notice.detail && <span className="ml-2 break-all text-muted-foreground">{notice.detail}</span>}
            </p>
            {notice.kind === "resumeFailed" && <>
              <Button variant="outline" size="sm" onClick={() => handleResume(notice.session)}>{t("common.retry")}</Button>
              <Button variant="ghost" size="sm" onClick={() => setSettingsOpen(true)}>{t("notice.openTerminalSettings")}</Button>
            </>}
            <Button variant="ghost" size="sm" onClick={dismissNotice}>{t("common.close")}</Button>
          </div>
          {noticeFailed && <details><summary>{t("common.technicalDetails")}</summary><p className="break-all text-muted-foreground">{notice.detail}</p></details>}
        </section>}
        {deleteReport && <section className="border-b border-border bg-muted/40 px-4 py-2 text-sm" aria-label={t("delete.resultTitle")}>
          <div className="flex flex-wrap items-start justify-between gap-2">
            <p role={deleteReport.error ? "alert" : "status"}>
              {deleteReport.error ? t(deleteReport.phase === "prepare" ? "delete.prepareFailed" : "delete.resultUnknown")
                : t("delete.resultSummary", {
                  deleted: results.filter((result) => result.status === "deleted").length,
                  missing: results.filter((result) => result.status === "alreadyMissing").length,
                  failed: results.filter((result) => result.status === "failed").length,
                  unconfirmed: results.filter((result) => result.status === "unconfirmed").length,
                })}
            </p>
            <div className="flex gap-2">
              {deleteReport.recheck && <Button variant="outline" size="sm" disabled={loading || deleting || preparingDelete}
                onClick={() => withDeleteFocus(handleRecheckDelete)}>{t("delete.recheck")}</Button>}
              {deleteFailures.length > 0 && <Button variant="outline" size="sm" disabled={deleting || preparingDelete || !canDelete}
                onClick={() => withDeleteFocus(handleRetryFailedDelete)}>{t("delete.retryFailed")}</Button>}
              <Button variant="ghost" size="sm" onClick={dismissDeleteReport}>{t("delete.dismiss")}</Button>
            </div>
          </div>
          {deleteReport.error && <details><summary>{t("common.technicalDetails")}</summary>
            <p className="break-all text-muted-foreground">{deleteReport.error}</p>
          </details>}
          {deleteFailures.length > 0 && <details className="mt-1">
            <summary className="cursor-pointer">{t("delete.failureDetails")}</summary>
            <ul className="mt-2 max-h-32 space-y-1 overflow-auto">
              {deleteFailures.map((result) => {
                const item = byId.get(result.sessionId);
                return <li key={result.sessionId} className="break-all text-[hsl(var(--error))]">
                  {item ? sessionName(item, t) : t("delete.unknownTitle", { id: result.sessionId })} · {result.error || result.warning}
                </li>;
              })}
            </ul>
          </details>}
        </section>}
      </div>

      <main className="relative flex min-h-0 flex-1 overflow-hidden">
        <section className="min-w-0 flex-1 overflow-hidden" aria-label={t("list.title")}>
          {filtered.length === 0 && (
            <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center text-sm text-muted-foreground">
              <p role="status">
                {t(initialLoading ? "list.scanning" : refreshError ? "list.unavailable"
                  : query.trim() ? "list.noSearchResults" : "list.noRecords", { query })}
              </p>
              {!initialLoading && !refreshError && query.trim() && <Button variant="outline" size="sm"
                onClick={() => { setQuery(""); search.current?.focus(); }}>{t("action.clearSearch")}</Button>}
            </div>
          )}
          <div hidden={filtered.length === 0} className="h-full">
            <SessionTable sessions={filtered} query={query} branchActivities={branchActivities} canDelete={canDelete} onVisibleRowsChange={handleVisibleRowsChange}
              selectedId={selectedId} locale={locale} t={t} onSelect={selectSession} onResume={handleResume}
              onRename={(session) => openEdit("rename", session)} onDescribe={(session) => openEdit("describe", session)}
              onDelete={(session) => withDeleteFocus(() => handleDelete(session), session.sessionId)}
              onToggleArchive={handleToggleArchive} onToggleCloud={handleToggleCloud} onGenerateSummary={handleGenerateSummary}
              onToggleFavorite={handleToggleFavorite} selectedSessionIds={selectedForDeleteIds}
              onToggleSelected={handleToggleSelected} onToggleVisibleSelection={handleToggleVisibleSelection} />
          </div>
        </section>
        {(!compact || (detailOpen && selected)) && (
          <aside aria-label={t("detail.title")} onKeyDown={(event) => {
            if (compact && event.key === "Escape") { event.preventDefault(); closeDetail(); }
          }} className={compact
            ? "absolute inset-y-0 right-0 z-30 flex w-[min(430px,100%)] flex-col border-l border-border bg-card shadow-xl"
            : "flex w-[350px] shrink-0 flex-col border-l border-border/60 bg-card"}>
            {compact && <div className="flex shrink-0 items-center justify-between border-b border-border px-3 py-1">
              <span className="text-sm font-medium">{t("detail.title")}</span>
              <Button ref={detailClose} size="icon" variant="ghost" aria-label={t("common.close")} onClick={closeDetail}><X /></Button>
            </div>}
            <div className="min-h-0 flex-1">
              <SessionDetail session={selected} depth={selectedDepth} parentMissing={selectedParentMissing}
                branchLastTimestamp={selectedBranchLastTimestamp} locale={locale} t={t} codexStatus={codexStatus}
                codexStatusLoading={codexStatusLoading} codexStatusError={codexStatusError}
                onRefreshCodexStatus={refreshCodexStatus} onOpenUsagePage={settingsActions.openUsagePage} onResume={handleResume} />
            </div>
          </aside>
        )}
      </main>

      <EditDialog open={editMode !== null} t={t} title={t(editMode === "rename" ? "action.rename" : "action.describe")}
        label={t(editMode === "rename" ? "prompt.enterName" : "prompt.enterDescription")}
        initialValue={(editMode === "rename" ? editTarget?.name : editTarget?.description) || ""}
        onSubmit={submitEdit} onClose={closeEdit} />

      <SettingsDialog open={settingsOpen} current={config.settings} locale={locale} t={t}
        onClose={() => setSettingsOpen(false)} onSaved={refresh} onPickDirectory={settingsActions.pickDirectory}
        onPickCloudFolder={settingsActions.pickCloudFolder} onLoadDebugLog={settingsActions.loadDebugLog}
        onOpenDebugLogFolder={settingsActions.openDebugLogFolder} onDetectGoogleDrive={settingsActions.detectGoogleDrive}
        onCheckEnvironment={settingsActions.checkEnvironment} onCheckForUpdates={settingsActions.checkForUpdates}
        onOpenReleases={settingsActions.openReleases} onSaveSettings={settingsActions.saveSettings} />

      <DeleteConfirmDialog open={pendingDelete !== null} count={directCount} pending={deleting} preparing={preparingDelete}
        canDelete={canDelete} title={impact > 0 ? t("delete.titleBulk", { count: impact })
          : retention !== undefined || skippedByAge > 0 ? t("delete.ageUnavailable")
          : skippedGuardians > 0 ? t("delete.guardianUnavailable") : t("delete.nothingSelected")}
        description={(impact ? t("delete.scope", { selected: directCount, affected: impact, descendants: preview.descendants })
          : (retention !== undefined || skippedByAge > 0) ? t("delete.ageUnavailableDescription") : "")
          + (skippedGuardians ? " " + t("delete.guardiansSkipped", { count: skippedGuardians }) : "")
          + (ageNotes ? " " + ageNotes : "")}
        previewRows={preview.rows} hiddenSelectedCount={preview.hidden.length}
        onUnselectHidden={() => handleUnselectHiddenDelete(preview.hidden)} returnFocus={returnDeleteFocus}
        confirmLabel={t("delete.confirmCount", { count: impact })} cancelLabel={t(impact ? "settings.cancel" : "common.close")}
        t={t} onConfirm={confirmDelete} onCancel={cancelDelete} />
    </div>
  );
}

export default App;
