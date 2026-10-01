// 세션 관리 화면의 application hook들을 조립한다.
import { useMemo, useState } from "react";
import { createT } from "@/i18n";
import { useSessionCommands } from "@/application/useSessionCommands";
import { useSessionData } from "@/application/useSessionData";
import { useSessionSelection } from "@/application/useSessionSelection";
import { useSettingsActions } from "@/application/useSettingsActions";

export function useSessionManager() {
  const [query, setQuery] = useState("");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const data = useSessionData();
  const selection = useSessionSelection(data.sessions);
  const settingsActions = useSettingsActions();
  const commands = useSessionCommands({
    refresh: data.refresh,
    setSessions: data.setSessions,
    setSelectedId: selection.setSelectedId,
    setSelectedForDeleteIds: selection.setSelectedForDeleteIds,
    selectedForDelete: selection.selectedForDelete,
  });

  const t = useMemo(() => createT(data.locale), [data.locale]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return data.sessions;
    return data.sessions.filter((s) => {
      const hay = [
        s.name,
        s.description,
        s.autoSummary,
        s.project,
        s.sessionId,
        s.firstUserMessage,
      ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      return hay.includes(q);
    });
  }, [data.sessions, query]);

  const total = data.sessions.length;
  const localCount = data.sessions.filter(
    (s) => s.storageType !== "cloud" && s.storageType !== "cloud-only",
  ).length;
  const cloudCount = data.sessions.filter(
    (s) => s.storageType === "cloud" || s.storageType === "cloud-only",
  ).length;

  return {
    t,
    query,
    setQuery,
    loading: data.loading,
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
