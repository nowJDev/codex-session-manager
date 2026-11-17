// 세션 선택과 다중 선택 파생 상태를 관리한다.
import { useEffect, useMemo, useState } from "react";
import type { Session } from "@/types";

export function useSessionSelection(sessions: Session[], authoritative = false) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedForDeleteIds, setSelectedForDeleteIds] = useState<Set<string>>(() => new Set());

  useEffect(() => {
    if (!authoritative) return;
    const existing = new Set(sessions.map((s) => s.sessionId));
    const registered = new Set(sessions.filter((s) => s.registered).map((s) => s.sessionId));
    setSelectedId((current) => current && !existing.has(current) ? null : current);
    setSelectedForDeleteIds((current) => {
      const next = new Set([...current].filter((id) => registered.has(id)));
      return next.size === current.size ? current : next;
    });
  }, [sessions, authoritative]);

  const selected = useMemo(
    () => sessions.find((s) => s.sessionId === selectedId) || null,
    [sessions, selectedId],
  );
  const selectedForDelete = useMemo(
    () => sessions.filter((s) => s.registered && selectedForDeleteIds.has(s.sessionId)),
    [sessions, selectedForDeleteIds],
  );

  function handleToggleSelected(s: Session) {
    if (!s.registered) return;
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
        if (!session.registered) continue;
        if (selected) {
          next.add(session.sessionId);
        } else {
          next.delete(session.sessionId);
        }
      }
      return next;
    });
  }

  return {
    selectedId,
    setSelectedId,
    selected,
    selectedForDeleteIds,
    setSelectedForDeleteIds,
    selectedForDelete,
    selectedForDeleteCount: selectedForDeleteIds.size,
    handleToggleSelected,
    handleToggleVisibleSelection,
  };
}
