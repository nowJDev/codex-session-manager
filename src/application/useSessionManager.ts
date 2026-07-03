// 세션 관리 화면의 애플리케이션 상태와 IPC workflow를 조율한다.
import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { createT, detectLocale, type Locale } from "@/i18n";
import { ipc } from "@/lib/ipc";
import type { AppConfig, CodexStatus, Session } from "@/types";

type EditMode = "rename" | "describe" | null;
type PendingDelete = { sessions: Session[] } | null;

export function useSessionManager() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [config, setConfig] = useState<AppConfig>({ sessions: {}, settings: {} });
  const [locale, setLocale] = useState<Locale>(detectLocale());
  const [editMode, setEditMode] = useState<EditMode>(null);
  const [editTarget, setEditTarget] = useState<Session | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [codexCliMissing, setCodexCliMissing] = useState(false);
  const [selectedForDeleteIds, setSelectedForDeleteIds] = useState<Set<string>>(() => new Set());
  const [pendingDelete, setPendingDelete] = useState<PendingDelete>(null);
  const [deleting, setDeleting] = useState(false);
  const [codexStatus, setCodexStatus] = useState<CodexStatus | null>(null);
  const [codexStatusLoading, setCodexStatusLoading] = useState(false);

  const t = useMemo(() => createT(locale), [locale]);

  const refreshCodexStatus = useCallback(async () => {
    setCodexStatusLoading(true);
    try {
      setCodexStatus(await ipc.getCodexStatus());
    } catch (err) {
      console.error(err);
    } finally {
      setCodexStatusLoading(false);
    }
  }, []);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [list, cfg] = await Promise.all([ipc.listSessions(), ipc.getConfig()]);
      setSessions(list);
      setConfig(cfg);
      const savedLocale = cfg.settings.locale;
      if (savedLocale === "en" || savedLocale === "ko") setLocale(savedLocale);
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    ipc.checkEnvironment().then((r) => {
      setCodexCliMissing(!r.codexCliFound);
      if (r.codexCliFound) {
        ipc.startAutoSummary().catch(() => {});
      }
    }).catch(() => {});

    const unlisten = listen<string>("auto-summary-progress", () => {
      refresh();
    });
    return () => {
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, [refresh]);

  useEffect(() => {
    refresh();
    refreshCodexStatus();
  }, [refresh, refreshCodexStatus]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return sessions;
    return sessions.filter((s) => {
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
  }, [sessions, query]);

  const selected = useMemo(
    () => sessions.find((s) => s.sessionId === selectedId) || null,
    [sessions, selectedId],
  );
  const selectedForDelete = useMemo(
    () => sessions.filter((s) => selectedForDeleteIds.has(s.sessionId)),
    [sessions, selectedForDeleteIds],
  );
  const selectedForDeleteCount = selectedForDelete.length;

  useEffect(() => {
    setSelectedForDeleteIds((cur) => {
      const existing = new Set(sessions.map((s) => s.sessionId));
      const next = new Set([...cur].filter((id) => existing.has(id)));
      return next.size === cur.size ? cur : next;
    });
  }, [sessions]);

  async function handleResume(s: Session) {
    try {
      if (s.storageType === "cloud") {
        await ipc.checkoutSession(s);
      }
      await ipc.resumeSession(s.sessionId, s.cwd);
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
        await ipc.deleteSession(target.sessionId, target.filePath);
      } else {
        await ipc.deleteSessions(
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
        await ipc.unarchiveSession(s.sessionId);
      } else {
        await ipc.archiveSession(s.sessionId);
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
        await ipc.checkoutSession(s);
      } else {
        await ipc.uploadToCloud(s);
      }
      await refresh();
    } catch (err) {
      console.error(err);
      alert(String(err));
    }
  }

  async function handleGenerateSummary(s: Session) {
    try {
      await ipc.generateSummary(s.sessionId, s.filePath);
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
      await ipc.saveSessionMeta(s.sessionId, { favorite: !s.favorite });
      await refresh();
    } catch (err) {
      alert(String(err));
    }
  }

  function handleToggleSelected(s: Session) {
    setSelectedForDeleteIds((cur) => {
      const next = new Set(cur);
      if (next.has(s.sessionId)) {
        next.delete(s.sessionId);
      } else {
        next.add(s.sessionId);
      }
      return next;
    });
  }

  function handleToggleVisibleSelection(visibleSessions: Session[], selected: boolean) {
    setSelectedForDeleteIds((cur) => {
      const next = new Set(cur);
      for (const session of visibleSessions) {
        if (selected) {
          next.add(session.sessionId);
        } else {
          next.delete(session.sessionId);
        }
      }
      return next;
    });
  }

  function openEdit(mode: EditMode, s: Session) {
    setEditMode(mode);
    setEditTarget(s);
  }

  async function submitEdit(value: string) {
    if (!editTarget || !editMode) return;
    const patch = editMode === "rename" ? { name: value || null } : { description: value || null };
    await ipc.saveSessionMeta(editTarget.sessionId, patch);
    setEditMode(null);
    setEditTarget(null);
    await refresh();
  }

  const total = sessions.length;
  const localCount = sessions.filter(
    (s) => s.storageType !== "cloud" && s.storageType !== "cloud-only",
  ).length;
  const cloudCount = sessions.filter(
    (s) => s.storageType === "cloud" || s.storageType === "cloud-only",
  ).length;

  return {
    t,
    query,
    setQuery,
    loading,
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
    filtered,
    selected,
    selectedForDeleteCount,
    total,
    localCount,
    cloudCount,
    refresh,
    refreshCodexStatus,
    handleResume,
    handleDelete,
    handleBulkDelete,
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
    closeEdit: () => {
      setEditMode(null);
      setEditTarget(null);
    },
  };
}
