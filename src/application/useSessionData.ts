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
  const refreshInFlight = useRef<Promise<void> | null>(null);
  const refreshQueued = useRef(false);

  const refreshCodexStatus = useCallback(async () => {
    setCodexStatusLoading(true);
    try {
      setCodexStatus(await tauriGateway.getCodexStatus());
    } catch (err) {
      console.error(err);
    } finally {
      setCodexStatusLoading(false);
    }
  }, []);

  const refresh = useCallback(() => {
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
            setSessions(list);
            setConfig(cfg);
            const savedLocale = cfg.settings.locale;
            if (savedLocale === "en" || savedLocale === "ko") setLocale(savedLocale);
          } catch (err) {
            console.error(err);
          }
        }
      } finally {
        refreshInFlight.current = null;
        setLoading(false);
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
    config,
    locale,
    codexCliMissing,
    codexStatus,
    codexStatusLoading,
    refresh,
    refreshCodexStatus,
  };
}
