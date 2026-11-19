// 세션에 대한 사용자 명령 workflow를 관리한다.
import { useRef, useState, type Dispatch, type SetStateAction } from "react";
import { tauriGateway } from "@/adapters/tauriGateway";
import type { DeleteProgress, DeleteSessionResult, DeleteSessionTarget, Session } from "@/types";

type EditMode = "rename" | "describe" | null;
type PendingDelete = {
  requestedTargets: DeleteSessionTarget[];
  sessions: DeleteSessionTarget[];
  fromSelection?: boolean;
  affectedIds: string[];
  skippedGuardianCount?: number;
  skippedAgeCount?: number;
  retentionDays?: number;
} | null;

type PrepareDeleteOptions = {
  refreshFirst?: boolean;
  retention?: { days: number; activityBefore: number };
  fromSelection?: boolean;
  keepPreview?: boolean;
};

interface UseSessionCommandsParams {
  refresh: () => Promise<void>;
  canDeleteNow: () => boolean;
  setSessions: Dispatch<SetStateAction<Session[]>>;
  setSelectedId: Dispatch<SetStateAction<string | null>>;
  setSelectedForDeleteIds: Dispatch<SetStateAction<Set<string>>>;
  selectedForDelete: Session[];
}

export function useSessionCommands({
  refresh,
  canDeleteNow,
  setSessions,
  setSelectedId,
  setSelectedForDeleteIds,
  selectedForDelete,
}: UseSessionCommandsParams) {
  const deletionGate = useRef(canDeleteNow);
  deletionGate.current = canDeleteNow;
  const deletionBlocked = () => !deletionGate.current();
  const [editMode, setEditMode] = useState<EditMode>(null);
  const [editTarget, setEditTarget] = useState<Session | null>(null);
  const [pendingDelete, setPendingDelete] = useState<PendingDelete>(null);
  const approvedPreview = useRef<PendingDelete>(null);
  const [deleting, setDeleting] = useState(false);
  const [preparingDelete, setPreparingDelete] = useState(false);
  const deletingRef = useRef(false);
  const ageCutoffs = useRef(new Map<string, number>());
  const resumeInFlight = useRef(false);
  const [notice, setNotice] = useState<{
    kind: "resumeFailed" | "folderCopied" | "folderRestored" | "folderSyncFailed";
    session: Session;
    detail: string;
  } | null>(null);
  const [deleteReport, setDeleteReport] = useState<{
    results: DeleteSessionResult[];
    error: string | null;
    phase: "prepare" | "delete";
    guardianOnly?: boolean;
    activityBefore?: number;
    recheck?: Pick<NonNullable<PendingDelete>, "requestedTargets" | "fromSelection" | "retentionDays">;
  } | null>(null);

  async function handleResume(s: Session) {
    const cloudOnly = s.storageType === "cloud" || s.storageType === "cloud-only";
    if (!s.registered && !cloudOnly) return;
    if ((cloudOnly && s.historyMode !== "legacy") || resumeInFlight.current) return;
    resumeInFlight.current = true;
    setNotice(null);
    let resumeTarget = s;
    try {
      if (cloudOnly) {
        await tauriGateway.checkoutSession(s);
        resumeTarget = { ...s, registered: true, storageType: "synced" };
      }
      await tauriGateway.resumeSession(s.sessionId, s.cwd);
    } catch (error) {
      setNotice({ kind: "resumeFailed", session: resumeTarget, detail: String(error) });
    } finally {
      resumeInFlight.current = false;
    }
  }

  async function prepareDelete(targets: DeleteSessionTarget[], { refreshFirst = false, retention, fromSelection, keepPreview }: PrepareDeleteOptions = {}) {
    if ((!targets.length && !keepPreview) || deletingRef.current || (!refreshFirst && deletionBlocked())) return;
    deletingRef.current = true;
    approvedPreview.current = null;
    setPreparingDelete(true);
    if (!keepPreview) setPendingDelete(null);
    try {
      if (refreshFirst) await refresh();
      if (deletionBlocked()) return;
      setDeleteReport(null);
      const plan = targets.length ? await tauriGateway.prepareSessionDeletion(targets) : { targets: [], affectedIds: [], skippedGuardianCount: 0, skippedAgeCount: 0 };
      if (deletionBlocked()) {
        setPendingDelete(null);
        return;
      }
      if (retention) {
        const selectedIds = plan.targets.map((target) => target.sessionId);
        ageCutoffs.current = new Map(selectedIds.map((id) => [id, retention.activityBefore]));
        setSelectedForDeleteIds(new Set(selectedIds));
      }
      const preview = {
        requestedTargets: targets,
        fromSelection,
        sessions: plan.targets,
        affectedIds: plan.affectedIds,
        skippedGuardianCount: plan.skippedGuardianCount,
        skippedAgeCount: plan.skippedAgeCount,
        retentionDays: retention?.days,
      };
      approvedPreview.current = preview;
      setPendingDelete(preview);
    } catch (err) {
      console.error(err);
      setPendingDelete(null);
      setDeleteReport({ results: [], error: String(err), phase: "prepare", recheck: { requestedTargets: targets, fromSelection, retentionDays: retention?.days } });
    } finally {
      deletingRef.current = false;
      setPreparingDelete(false);
    }
  }

  async function handleDelete(s: Session) {
    if (!s.registered) return;
    await prepareDelete([{ sessionId: s.sessionId }]);
  }

  async function handleBulkDelete() {
    const targets = selectedForDelete.filter((s) => s.registered).map(({ sessionId }) => {
      const activityBefore = ageCutoffs.current.get(sessionId);
      return activityBefore === undefined ? { sessionId } : { sessionId, activityBefore };
    });
    await prepareDelete(targets, { fromSelection: true });
  }

  async function handleUnselectHiddenDelete(ids: string[]) {
    if (!pendingDelete?.fromSelection || pendingDelete !== approvedPreview.current || !ids.length || deletingRef.current || deletionBlocked()) return;
    const hidden = new Set(ids);
    const cutoff = pendingDelete.sessions.find((target) => target.activityBefore !== undefined)?.activityBefore;
    const targets = pendingDelete.requestedTargets.filter((target) => !hidden.has(target.sessionId)).map((target) => ({
      ...target, activityBefore: cutoff ?? target.activityBefore,
    }));
    if (cutoff !== undefined) {
      for (const target of targets) ageCutoffs.current.set(target.sessionId, cutoff);
    }
    for (const id of hidden) ageCutoffs.current.delete(id);
    setSelectedForDeleteIds((previous) => new Set([...previous].filter((id) => !hidden.has(id))));
    await prepareDelete(targets, { fromSelection: true, keepPreview: true });
  }

  async function handleRecheckDelete() {
    const recheck = deleteReport?.recheck;
    if (!recheck) return;
    const cutoff = recheck.requestedTargets.find((target) => target.activityBefore !== undefined)?.activityBefore;
    const retention = recheck.retentionDays !== undefined && cutoff !== undefined
      ? { days: recheck.retentionDays, activityBefore: cutoff } : undefined;
    await prepareDelete(recheck.requestedTargets, { refreshFirst: true, fromSelection: recheck.fromSelection, retention });
  }

  async function handleDeleteByAge(sessions: Session[], days: number) {
    if (!Number.isSafeInteger(days) || days <= 0) return;
    const activityBefore = Math.floor(Date.now() / 1000) - days * 86_400;
    const targets = sessions.filter((s) => s.registered).map((s) => ({ sessionId: s.sessionId, activityBefore }));
    await prepareDelete(targets, { retention: { days, activityBefore } });
  }

  function forgetAgeSelection(sessions: Session[]) {
    for (const session of sessions) ageCutoffs.current.delete(session.sessionId);
  }

  async function handleRetryFailedDelete() {
    const targets = deleteReport?.results
      .filter((result) => result.status === "failed" || result.status === "unconfirmed" || result.warning)
      .map((result) => ({
        sessionId: result.sessionId,
        ...(deleteReport.guardianOnly ? { guardianOnly: true } : {}),
        ...(deleteReport.activityBefore !== undefined ? { activityBefore: deleteReport.activityBefore } : {}),
      })) ?? [];
    await prepareDelete(targets, { refreshFirst: true });
  }

  async function confirmDelete(reportProgress: (update: DeleteProgress) => void = () => {}) {
    const preview = pendingDelete;
    if (!preview?.sessions.length || preview !== approvedPreview.current || deletingRef.current || deletionBlocked()) return;
    const targets = preview.sessions;
    approvedPreview.current = null;
    deletingRef.current = true;
    setDeleting(true);
    setDeleteReport(null);
    let active = true;
    try {
      reportProgress({ phase: "checkingScope" });
      const results = await tauriGateway.deleteSessions(targets, preview.affectedIds, (update) => {
        if (active) reportProgress(update);
      });
      const activityBefore = targets.find((target) => target.activityBefore !== undefined)?.activityBefore;
      setDeleteReport({ results, error: null, phase: "delete", guardianOnly: targets.some((target) => target.guardianOnly), activityBefore });
      const deletedIds = new Set(
        results.filter((result) => result.status === "deleted" || result.status === "alreadyMissing").map((result) => result.sessionId),
      );
      for (const id of deletedIds) ageCutoffs.current.delete(id);
      if (activityBefore !== undefined) {
        for (const result of results) {
          if (result.status === "failed" || result.status === "unconfirmed") {
            ageCutoffs.current.set(result.sessionId, activityBefore);
          }
        }
      }
      setSelectedId((cur) => (cur && deletedIds.has(cur) ? null : cur));
      setSelectedForDeleteIds((cur) => {
        const next = new Set(cur);
        for (const id of deletedIds) next.delete(id);
        for (const result of results) {
          if (result.status === "failed" || result.status === "unconfirmed") next.add(result.sessionId);
        }
        return next;
      });
    } catch (err) {
      console.error(err);
      setDeleteReport({ results: [], error: String(err), phase: "delete", recheck: preview });
    } finally {
      active = false;
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
    approvedPreview.current = null;
    setPendingDelete(null);
  }

  async function handleToggleArchive(s: Session) {
    if (!s.registered) return;
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

  async function handleToggleCloud(s: Session, destination: string) {
    if (s.historyMode !== "legacy" || !s.filePath) return;
    try {
      const restoring = s.storageType === "cloud-only" || s.storageType === "cloud";
      if (restoring) {
        await tauriGateway.checkoutSession(s);
      } else {
        await tauriGateway.uploadToCloud(s);
      }
      setNotice({ kind: restoring ? "folderRestored" : "folderCopied", session: s, detail: destination });
      await refresh();
    } catch (error) {
      setNotice({ kind: "folderSyncFailed", session: s, detail: String(error) });
    }
  }

  async function handleGenerateSummary(s: Session) {
    if (!s.registered) return;
    try {
      await tauriGateway.generateSummary(s.sessionId);
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
    preparingDelete,
    deleteReport,
    dismissDeleteReport: () => setDeleteReport(null),
    handleResume,
    handleDelete,
    handleBulkDelete,
    handleUnselectHiddenDelete,
    handleRecheckDelete,
    notice,
    dismissNotice: () => setNotice(null),
    handleDeleteByAge,
    forgetAgeSelection,
    prepareDelete,
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
