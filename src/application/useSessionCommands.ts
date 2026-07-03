// 세션에 대한 사용자 명령 workflow를 관리한다.
import { useState, type Dispatch, type SetStateAction } from "react";
import { tauriGateway } from "@/adapters/tauriGateway";
import type { Session } from "@/types";

type EditMode = "rename" | "describe" | null;
type PendingDelete = { sessions: Session[] } | null;

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
    setPendingDelete({ sessions: [s] });
  }

  function handleBulkDelete() {
    if (selectedForDelete.length === 0) return;
    setPendingDelete({ sessions: selectedForDelete });
  }

  async function confirmDelete() {
    const targets = pendingDelete?.sessions ?? [];
    if (targets.length === 0 || deleting) return;
    setDeleting(true);
    try {
      if (targets.length === 1) {
        const target = targets[0];
        await tauriGateway.deleteSession(target.sessionId, target.filePath);
      } else {
        await tauriGateway.deleteSessions(
          targets.map((s) => ({
            sessionId: s.sessionId,
            filePath: s.filePath,
          })),
        );
      }
      const deletedIds = new Set(targets.map((s) => s.sessionId));
      setSelectedId((cur) => (cur && deletedIds.has(cur) ? null : cur));
      setSelectedForDeleteIds((cur) => {
        if (targets.length > 1) return new Set();
        const next = new Set(cur);
        for (const id of deletedIds) next.delete(id);
        return next;
      });
      await refresh();
      setPendingDelete(null);
    } catch (err) {
      console.error(err);
      alert(String(err));
    } finally {
      setDeleting(false);
    }
  }

  function cancelDelete() {
    if (deleting) return;
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
    handleResume,
    handleDelete,
    handleBulkDelete,
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
