// 세션 선택과 다중 선택 파생 상태를 관리한다.
import { useEffect, useMemo, useState } from "react";
import type { Session } from "@/types";

export function useSessionSelection(sessions: Session[]) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedForDeleteIds, setSelectedForDeleteIds] = useState<Set<string>>(() => new Set());

  const selected = useMemo(
    () => sessions.find((s) => s.sessionId === selectedId) || null,
    [sessions, selectedId],
  );
  const selectedForDelete = useMemo(
    () => sessions.filter((s) => selectedForDeleteIds.has(s.sessionId)),
    [sessions, selectedForDeleteIds],
  );

  useEffect(() => {
    setSelectedForDeleteIds((cur) => {
      const existing = new Set(sessions.map((s) => s.sessionId));
      const next = new Set([...cur].filter((id) => existing.has(id)));
      return next.size === cur.size ? cur : next;
    });
  }, [sessions]);

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

  return {
    selectedId,
    setSelectedId,
    selected,
    selectedForDeleteIds,
    setSelectedForDeleteIds,
    selectedForDelete,
    selectedForDeleteCount: selectedForDelete.length,
    handleToggleSelected,
    handleToggleVisibleSelection,
  };
}
