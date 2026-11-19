// 세션 화면의 원천 데이터 조회와 외부 이벤트 구독을 관리한다.
import { useCallback, useEffect, useRef, useState } from "react";
import { tauriGateway } from "@/adapters/tauriGateway";
import { detectLocale, type Locale } from "@/i18n";
import type { AppConfig, CodexStatus, Session } from "@/types";

export function useSessionData() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [loading, setLoading] = useState(false);
  const [config, setConfig] = useState<AppConfig>({ sessions: {}, settings: {} });
  const [locale, setLocale] = useState<Locale>(detectLocale());
  const [codexCliMissing, setCodexCliMissing] = useState(false);
  const [codexStatus, setCodexStatus] = useState<CodexStatus | null>(null);
  const [codexStatusLoading, setCodexStatusLoading] = useState(false);
  const [initialLoading, setInitialLoading] = useState(true);
  const [codexStatusError, setCodexStatusError] = useState<string | null>(null);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const [snapshotInfo, setSnapshotInfo] = useState({ complete: false, stale: false, missingCount: 0 });
  const hasCompleteSnapshot = useRef(false);
  const deletionAllowed = useRef(false);
  const canDeleteNow = useCallback(() => deletionAllowed.current, []);
  const refreshInFlight = useRef<Promise<void> | null>(null);
  const refreshQueued = useRef(false);

  const refreshCodexStatus = useCallback(async () => {
    setCodexStatusLoading(true);
    try {
      setCodexStatus(await tauriGateway.getCodexStatus());
      setCodexStatusError(null);
    } catch (err) {
      setCodexStatusError(String(err));
    } finally {
      setCodexStatusLoading(false);
    }
  }, []);

  const refresh = useCallback(() => {
    deletionAllowed.current = false;
    refreshQueued.current = true;
    if (refreshInFlight.current) return refreshInFlight.current;
    setLoading(true);
    refreshInFlight.current = Promise.resolve().then(async () => {
      try {
        while (refreshQueued.current) {
          refreshQueued.current = false;
          try {
            const [list, cfg] = await Promise.all([
              tauriGateway.listSessions(),
              tauriGateway.getConfig(),
            ]);
            if (refreshQueued.current) continue;
            if (list.complete || !hasCompleteSnapshot.current) setSessions(list.sessions);
            setSnapshotInfo({ complete: list.complete, stale: !list.complete && hasCompleteSnapshot.current, missingCount: list.missingCount });
            deletionAllowed.current = list.complete;
            if (list.complete) hasCompleteSnapshot.current = true;
            setRefreshError(list.complete ? null : list.errors.join("\n") || "Session snapshot incomplete");
            setConfig(cfg);
            const savedLocale = cfg.settings.locale;
            if (savedLocale === "en" || savedLocale === "ko") setLocale(savedLocale);
          } catch (err) {
            console.error(err);
            if (!refreshQueued.current) {
              deletionAllowed.current = false;
              setSnapshotInfo((previous) => ({ ...previous, complete: false, stale: hasCompleteSnapshot.current, missingCount: 0 }));
              setRefreshError(String(err));
            }
          }
        }
      } finally {
        refreshInFlight.current = null;
        setLoading(false);
        setInitialLoading(false);
      }
    });
    return refreshInFlight.current;
  }, []);

  useEffect(() => {
    tauriGateway.checkEnvironment().then((r) => {
      setCodexCliMissing(!r.codexCliFound);
      if (r.codexCliFound) {
        tauriGateway.startAutoSummary().catch(() => {});
      }
    }).catch(() => {});

    const unlisten = tauriGateway.listenAutoSummaryProgress(() => {
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

  return {
    sessions,
    setSessions,
    loading,
    initialLoading,
    codexStatusError,
    refreshError,
    snapshotComplete: snapshotInfo.complete,
    snapshotStale: snapshotInfo.stale,
    missingCount: snapshotInfo.missingCount,
    canDelete: snapshotInfo.complete && !snapshotInfo.stale && !loading,
    canDeleteNow,
    config,
    locale,
    codexCliMissing,
    codexStatus,
    codexStatusLoading,
    refresh,
    refreshCodexStatus,
  };
}
