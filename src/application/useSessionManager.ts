// 세션 관리 화면의 application hook들을 조립한다.
import { useCallback, useMemo, useState } from "react";
import { createT } from "@/i18n";
import { buildSessionTreeRows, filterSessionsWithAncestors } from "@/lib/sessionTableState";
import type { Session } from "@/types";
import { useSessionCommands } from "@/application/useSessionCommands";
import { useSessionData } from "@/application/useSessionData";
import { useSessionSelection } from "@/application/useSessionSelection";
import { useSettingsActions } from "@/application/useSettingsActions";

export function useSessionManager() {
  const [query, setQuery] = useState("");
  const [visibleSessions, setVisibleSessions] = useState<Session[]>([]);
  const handleVisibleRowsChange = useCallback((rows: Session[]) => setVisibleSessions(rows), []);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const data = useSessionData();
  const selection = useSessionSelection(data.sessions, data.canDelete);
  const settingsActions = useSettingsActions();
  const commands = useSessionCommands({
    refresh: data.refresh,
    canDeleteNow: data.canDeleteNow,
    setSessions: data.setSessions,
    setSelectedId: selection.setSelectedId,
    setSelectedForDeleteIds: selection.setSelectedForDeleteIds,
    selectedForDelete: selection.selectedForDelete,
  });

  const t = useMemo(() => createT(data.locale), [data.locale]);

  const filtered = useMemo(
    () => filterSessionsWithAncestors(data.sessions, query),
    [data.sessions, query],
  );

  const allRows = useMemo(() => buildSessionTreeRows(data.sessions, new Set(data.sessions.map((s) => s.sessionId))), [data.sessions]);
  const selectedRow = allRows.find((row) => row.session.sessionId === selection.selectedId);
  const total = data.sessions.length;
  const workerCount = data.sessions.filter((s) => s.sessionKind === "worker").length;
  const visibleSelectedCount = visibleSessions.filter((s) => s.registered && selection.selectedForDeleteIds.has(s.sessionId)).length;
  const localCount = data.sessions.filter(
    (s) => s.registered,
  ).length;
  const cloudCount = data.sessions.filter(
    (s) => s.storageType === "cloud" || s.storageType === "cloud-only",
  ).length;

  return {
    t,
    query,
    setQuery,
    handleVisibleRowsChange,
    displayedCount: visibleSessions.length,
    visibleSelectedCount,
    workerCount,
    selectedDepth: selectedRow?.depth ?? 0,
    selectedParentMissing: selectedRow?.parentMissing ?? false,
    canDelete: data.canDelete,
    snapshotStale: data.snapshotStale,
    missingCount: data.missingCount,
    loading: data.loading,
    refreshError: data.refreshError,
    locale: data.locale,
    config: data.config,
    selectedId: selection.selectedId,
    setSelectedId: selection.setSelectedId,
    editMode: commands.editMode,
    editTarget: commands.editTarget,
    settingsOpen,
    setSettingsOpen,
    codexCliMissing: data.codexCliMissing,
    codexStatus: data.codexStatus,
    codexStatusLoading: data.codexStatusLoading,
    selectedForDeleteIds: selection.selectedForDeleteIds,
    pendingDelete: commands.pendingDelete,
    deleting: commands.deleting,
    preparingDelete: commands.preparingDelete,
    deleteReport: commands.deleteReport,
    dismissDeleteReport: commands.dismissDeleteReport,
    filtered,
    selected: selection.selected,
    selectedForDeleteCount: selection.selectedForDeleteCount,
    total,
    localCount,
    cloudCount,
    refresh: data.refresh,
    refreshCodexStatus: data.refreshCodexStatus,
    settingsActions,
    handleResume: commands.handleResume,
    handleDelete: commands.handleDelete,
    handleBulkDelete: commands.handleBulkDelete,
    handleRetryFailedDelete: commands.handleRetryFailedDelete,
    confirmDelete: commands.confirmDelete,
    cancelDelete: commands.cancelDelete,
    handleToggleArchive: commands.handleToggleArchive,
    handleToggleCloud: commands.handleToggleCloud,
    handleGenerateSummary: commands.handleGenerateSummary,
    handleToggleFavorite: commands.handleToggleFavorite,
    handleToggleSelected: selection.handleToggleSelected,
    handleToggleVisibleSelection: selection.handleToggleVisibleSelection,
    openEdit: commands.openEdit,
    submitEdit: commands.submitEdit,
    closeEdit: commands.closeEdit,
  };
}
