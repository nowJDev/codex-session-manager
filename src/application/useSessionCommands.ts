// 세션에 대한 사용자 명령 workflow를 관리한다.
import { useRef, useState, type Dispatch, type SetStateAction } from "react";
import { tauriGateway } from "@/adapters/tauriGateway";
import type { DeleteSessionResult, Session } from "@/types";

type EditMode = "rename" | "describe" | null;
type PendingDelete = { sessions: Pick<Session, "sessionId" | "filePath">[] } | null;

interface UseSessionCommandsParams {
  refresh: () => Promise<void>;
  setSessions: Dispatch<SetStateAction<Session[]>>;
  setSelectedId: Dispatch<SetStateAction<string | null>>;
  setSelectedForDeleteIds: Dispatch<SetStateAction<Set<string>>>;
  selectedForDelete: Session[];
}

export function useSessionCommands({
  refresh,
  setSessions,
  setSelectedId,
  setSelectedForDeleteIds,
  selectedForDelete,
}: UseSessionCommandsParams) {
  const [editMode, setEditMode] = useState<EditMode>(null);
  const [editTarget, setEditTarget] = useState<Session | null>(null);
  const [pendingDelete, setPendingDelete] = useState<PendingDelete>(null);
  const [deleting, setDeleting] = useState(false);
  const deletingRef = useRef(false);
  const [deleteReport, setDeleteReport] = useState<{
    results: DeleteSessionResult[];
    error: string | null;
  } | null>(null);

  async function handleResume(s: Session) {
    try {
      if (s.storageType === "cloud") {
        await tauriGateway.checkoutSession(s);
      }
      await tauriGateway.resumeSession(s.sessionId, s.cwd);
    } catch (err) {
      console.error(err);
      alert(String(err));
    }
  }

  function handleDelete(s: Session) {
    if (deletingRef.current) return;
    setPendingDelete({ sessions: [s] });
  }

  function handleBulkDelete() {
    if (selectedForDelete.length === 0 || deletingRef.current) return;
    setPendingDelete({ sessions: selectedForDelete });
  }

  function handleRetryFailedDelete() {
    if (deletingRef.current) return;
    const sessions = deleteReport?.results.filter((result) => result.status === "failed") ?? [];
    if (sessions.length > 0) setPendingDelete({ sessions });
  }

  async function confirmDelete() {
    const targets = pendingDelete?.sessions ?? [];
    if (targets.length === 0 || deletingRef.current) return;
    deletingRef.current = true;
    setDeleting(true);
    setDeleteReport(null);
    try {
      const results = await tauriGateway.deleteSessions(
        targets.map((s) => ({ sessionId: s.sessionId, filePath: s.filePath })),
      );
      setDeleteReport({ results, error: null });
      const deletedIds = new Set(
        results.filter((result) => result.status !== "failed").map((result) => result.sessionId),
      );
      setSelectedId((cur) => (cur && deletedIds.has(cur) ? null : cur));
      setSelectedForDeleteIds((cur) => {
        const next = new Set(cur);
        for (const id of deletedIds) next.delete(id);
        for (const result of results) {
          if (result.status === "failed") next.add(result.sessionId);
        }
        return next;
      });
    } catch (err) {
      console.error(err);
      setDeleteReport({ results: [], error: String(err) });
    } finally {
      setPendingDelete(null);
      try {
        await refresh();
      } finally {
        deletingRef.current = false;
        setDeleting(false);
      }
    }
  }

  function cancelDelete() {
    if (deletingRef.current) return;
    setPendingDelete(null);
  }

  async function handleToggleArchive(s: Session) {
    try {
      if (s.archived) {
        await tauriGateway.unarchiveSession(s.sessionId);
      } else {
        await tauriGateway.archiveSession(s.sessionId);
      }
      setSelectedId(null);
      await refresh();
    } catch (err) {
      console.error(err);
      alert(String(err));
    }
  }

  async function handleToggleCloud(s: Session) {
    try {
      const st = s.storageType;
      if (st === "cloud-only" || st === "cloud") {
        await tauriGateway.checkoutSession(s);
      } else {
        await tauriGateway.uploadToCloud(s);
      }
      await refresh();
    } catch (err) {
      console.error(err);
      alert(String(err));
    }
  }

  async function handleGenerateSummary(s: Session) {
    try {
      await tauriGateway.generateSummary(s.sessionId, s.filePath);
      await refresh();
    } catch (err) {
      alert(String(err));
    }
  }

  async function handleToggleFavorite(s: Session) {
    setSessions((prev) =>
      prev.map((x) => (x.sessionId === s.sessionId ? { ...x, favorite: !x.favorite } : x))
    );
    try {
      await tauriGateway.saveSessionMeta(s.sessionId, { favorite: !s.favorite });
      await refresh();
    } catch (err) {
      alert(String(err));
    }
  }

  function openEdit(mode: EditMode, s: Session) {
    setEditMode(mode);
    setEditTarget(s);
  }

  async function submitEdit(value: string) {
    if (!editTarget || !editMode) return;
    const patch = editMode === "rename" ? { name: value || null } : { description: value || null };
    await tauriGateway.saveSessionMeta(editTarget.sessionId, patch);
    setEditMode(null);
    setEditTarget(null);
    await refresh();
  }

  return {
    editMode,
    editTarget,
    pendingDelete,
    deleting,
    deleteReport,
    dismissDeleteReport: () => setDeleteReport(null),
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
    openEdit,
    submitEdit,
    closeEdit: () => {
      setEditMode(null);
      setEditTarget(null);
    },
  };
}
