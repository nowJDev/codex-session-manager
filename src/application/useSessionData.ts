// 세션 화면의 원천 데이터 조회와 외부 이벤트 구독을 관리한다.
import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { detectLocale, type Locale } from "@/i18n";
import { ipc } from "@/lib/ipc";
import type { AppConfig, CodexStatus, Session } from "@/types";

export function useSessionData() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [loading, setLoading] = useState(false);
  const [config, setConfig] = useState<AppConfig>({ sessions: {}, settings: {} });
  const [locale, setLocale] = useState<Locale>(detectLocale());
  const [codexCliMissing, setCodexCliMissing] = useState(false);
  const [codexStatus, setCodexStatus] = useState<CodexStatus | null>(null);
  const [codexStatusLoading, setCodexStatusLoading] = useState(false);

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
