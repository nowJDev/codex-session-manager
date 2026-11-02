// 세션 관리 화면의 application hook들을 조립한다.
import { useMemo, useState } from "react";
import { createT } from "@/i18n";
import { filterSessionsWithAncestors } from "@/lib/sessionTableState";
import { useSessionCommands } from "@/application/useSessionCommands";
import { useSessionData } from "@/application/useSessionData";
import { useSessionSelection } from "@/application/useSessionSelection";
import { useSettingsActions } from "@/application/useSettingsActions";

export function useSessionManager() {
  const [query, setQuery] = useState("");
  const [showHidden, setShowHidden] = useState(false);
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

  const filtered = useMemo(
    () => filterSessionsWithAncestors(data.sessions.filter((session) => showHidden || !session.hidden), query),
    [data.sessions, query, showHidden],
  );

  const total = data.sessions.length;
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
    showHidden,
    setShowHidden,
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
