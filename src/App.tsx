// 세션 목록의 상태와 API 삭제 확인 및 결과 화면을 조립한다.
import { AlertTriangle, RefreshCw, Search, Settings as SettingsIcon, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SessionTable } from "@/components/SessionTable";
import { SessionDetail } from "@/components/SessionDetail";
import { EditDialog } from "@/components/EditDialog";
import { DeleteConfirmDialog } from "@/components/DeleteConfirmDialog";
import { SettingsDialog } from "@/components/SettingsDialog";
import { useSessionManager } from "@/application/useSessionManager";

function App() {
  const manager = useSessionManager();
  const {
    t,
    query,
    setQuery,
    showHidden,
    setShowHidden,
    loading,
    refreshError,
    locale,
    config,
    selectedId,
    setSelectedId,
    editMode,
    editTarget,
    settingsOpen,
    setSettingsOpen,
    codexCliMissing,
    codexStatus,
    codexStatusLoading,
    selectedForDeleteIds,
    pendingDelete,
    deleting,
    preparingDelete,
    deleteReport,
    dismissDeleteReport,
    filtered,
    selected,
    selectedForDeleteCount,
    localCount,
    cloudCount,
    refresh,
    refreshCodexStatus,
    settingsActions,
    handleResume,
    handleDelete,
    handleBulkDelete,
    handleRetryFailedDelete,
    confirmDelete,
    cancelDelete,
    handleToggleArchive,
    handleToggleCloud,
    handleGenerateSummary,
    handleToggleFavorite,
    handleToggleSelected,
    handleToggleVisibleSelection,
    openEdit,
    submitEdit,
    closeEdit,
  } = manager;

  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      {codexCliMissing && (
        <div className="flex items-center gap-2 border-b border-amber-500/40 bg-amber-500/10 px-5 py-2 text-xs text-amber-300">
          <AlertTriangle className="h-4 w-4 shrink-0" />
          <span className="flex-1">
            {(t("warning.codexCliMissing") !== "warning.codexCliMissing"
              ? t("warning.codexCliMissing")
              : "Codex CLI not found on PATH. Resume actions won't work.")}
          </span>
          <a
            href="https://github.com/openai/codex"
            target="_blank"
            rel="noreferrer"
            className="underline hover:text-amber-200"
          >
            {(t("warning.codexCliInstall") !== "warning.codexCliInstall"
              ? t("warning.codexCliInstall")
              : "Install guide")}
          </a>
        </div>
      )}
      <header className="flex items-center gap-3 border-b border-border/60 px-5 py-3">
        <div className="flex flex-col">
          <h1 className="text-base font-semibold leading-tight">{t("app.title")}</h1>
          <p className="text-[11px] text-muted-foreground">
            {t("list.counts", { registered: localCount, displayed: filtered.length, selected: selectedForDeleteCount })} · cloud {cloudCount}
          </p>
        </div>
        <div className="relative ml-6 flex-1 max-w-md">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search…"
            className="pl-8"
          />
        </div>
        <label className="flex shrink-0 items-center gap-1.5 text-xs text-muted-foreground">
          <input type="checkbox" checked={showHidden} onChange={(e) => setShowHidden(e.currentTarget.checked)} />
          {t("list.showHidden")}
        </label>
        <div className="flex items-center gap-1.5">
          {selectedForDeleteCount > 0 && (
            <Button
              variant="destructive"
              size="sm"
              onClick={handleBulkDelete}
              disabled={deleting || preparingDelete}
              title={t("action.deleteSelected", { count: selectedForDeleteCount })}
            >
              <Trash2 className="h-4 w-4" />
              {t("action.deleteSelected", { count: selectedForDeleteCount })}
            </Button>
          )}
          <Button
            variant="ghost"
            size="icon"
            onClick={refresh}
            disabled={loading}
            title="Refresh"
          >
            <RefreshCw className={loading ? "animate-spin" : ""} />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setSettingsOpen(true)}
            title="Settings"
          >
            <SettingsIcon />
          </Button>
        </div>
      </header>

      {refreshError && <p role="alert" className="border-b border-destructive/40 bg-destructive/10 px-5 py-2 text-sm text-destructive">{t("error.refreshFailed", { error: refreshError })}</p>}
      {preparingDelete && <p role="status" className="border-b border-border px-5 py-2 text-sm">{t("delete.preparing")}</p>}

      {deleteReport && (
        <section className="border-b border-border bg-muted/40 px-5 py-3 text-sm" aria-label={t("delete.resultTitle")}>
          <div className="flex items-start justify-between gap-3">
            <p role="status">
              {deleteReport.error
                ? t(deleteReport.phase === "prepare" ? "delete.prepareFailed" : "delete.resultUnknown")
                : t("delete.resultSummary", {
                    deleted: deleteReport.results.filter((result) => result.status === "deleted").length,
                    missing: deleteReport.results.filter((result) => result.status === "alreadyMissing").length,
                    failed: deleteReport.results.filter((result) => result.status === "failed").length,
                    unconfirmed: deleteReport.results.filter((result) => result.status === "unconfirmed").length,
                  })}
            </p>
            <div className="flex shrink-0 gap-2">
              {deleteReport.results.some((result) => result.status === "failed" || result.status === "unconfirmed" || result.warning) && (
                <Button variant="outline" size="sm" disabled={deleting || preparingDelete} onClick={handleRetryFailedDelete}>
                  {t("delete.retryFailed")}
                </Button>
              )}
              <Button variant="ghost" size="sm" onClick={dismissDeleteReport}>
                {t("delete.dismiss")}
              </Button>
            </div>
          </div>
          {deleteReport.error && <p className="break-all text-destructive">{deleteReport.error}</p>}
          {deleteReport.results.some((result) => result.status === "failed" || result.status === "unconfirmed") && (
            <details className="mt-1">
              <summary className="cursor-pointer">{t("delete.failureDetails")}</summary>
              <ul className="mt-2 max-h-40 space-y-1 overflow-auto">
                {deleteReport.results.filter((result) => result.status === "failed" || result.status === "unconfirmed").map((result) => (
                  <li key={result.sessionId} className="break-all text-destructive">
                    {result.sessionId} — {result.error}
                  </li>
                ))}
              </ul>
            </details>
          )}
          {deleteReport.results.filter((result) => result.warning).map((result) => (
            <p key={result.sessionId} className="mt-1 break-all text-amber-500">{t("delete.warning")} · {result.sessionId} — {result.warning}</p>
          ))}
        </section>
      )}

      <main className="flex flex-1 overflow-hidden">
        <section className="min-w-0 flex-1 overflow-auto">
          <SessionTable
            sessions={filtered}
            autoExpand={!!query.trim()}
            selectedId={selectedId}
            locale={locale}
            t={t}
            onSelect={(s) => setSelectedId(s.sessionId)}
            onResume={handleResume}
            onRename={(s) => openEdit("rename", s)}
            onDescribe={(s) => openEdit("describe", s)}
            onDelete={handleDelete}
            onToggleArchive={handleToggleArchive}
            onToggleCloud={handleToggleCloud}
            onGenerateSummary={handleGenerateSummary}
            onToggleFavorite={handleToggleFavorite}
            selectedSessionIds={selectedForDeleteIds}
            onToggleSelected={handleToggleSelected}
            onToggleVisibleSelection={handleToggleVisibleSelection}
          />
        </section>
        <aside className="w-[380px] shrink-0 border-l border-border/60 bg-card/30">
          <SessionDetail
            session={selected}
            locale={locale}
            t={t}
            codexStatus={codexStatus}
            codexStatusLoading={codexStatusLoading}
            onRefreshCodexStatus={refreshCodexStatus}
            onOpenUsagePage={settingsActions.openUsagePage}
            onResume={handleResume}
          />
        </aside>
      </main>

      <EditDialog
        open={editMode !== null}
        title={editMode === "rename" ? t("action.rename") : t("action.describe")}
        label={editMode === "rename" ? t("prompt.enterName") : t("prompt.enterDescription")}
        initialValue={
          editTarget
            ? editMode === "rename"
              ? editTarget.name || ""
              : editTarget.description || ""
            : ""
        }
        onSubmit={submitEdit}
        onClose={closeEdit}
      />

      <SettingsDialog
        open={settingsOpen}
        current={config.settings}
        locale={locale}
        t={t}
        onClose={() => setSettingsOpen(false)}
        onSaved={refresh}
        onPickDirectory={settingsActions.pickDirectory}
        onPickCloudFolder={settingsActions.pickCloudFolder}
        onLoadDebugLog={settingsActions.loadDebugLog}
        onOpenDebugLogFolder={settingsActions.openDebugLogFolder}
        onConnectGoogleDrive={settingsActions.connectGoogleDrive}
        onCheckEnvironment={settingsActions.checkEnvironment}
        onCheckForUpdates={settingsActions.checkForUpdates}
        onOpenReleases={settingsActions.openReleases}
        onSaveSettings={settingsActions.saveSettings}
      />

      <DeleteConfirmDialog
        open={pendingDelete !== null}
        count={pendingDelete?.sessions.length ?? 0}
        pending={deleting}
        title={
          pendingDelete?.sessions.length === 1
            ? t("delete.titleSingle")
            : t("delete.titleBulk", { count: pendingDelete?.sessions.length ?? 0 })
        }
        description={
          t("delete.scope", {
            selected: pendingDelete?.sessions.length ?? 0,
            affected: pendingDelete?.affectedIds.length ?? 0,
            descendants: pendingDelete?.affectedIds.filter((id) => !pendingDelete.sessions.some((s) => s.sessionId === id)).length ?? 0,
          })
        }
        confirmLabel={t("delete.confirm")}
        cancelLabel={t("settings.cancel")}
        pendingLabel={t("delete.pending")}
        onConfirm={confirmDelete}
        onCancel={cancelDelete}
      />
    </div>
  );
}

export default App;
