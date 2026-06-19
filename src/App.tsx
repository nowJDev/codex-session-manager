import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { AlertTriangle, RefreshCw, Search, Settings as SettingsIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SessionTable } from "@/components/SessionTable";
import { SessionDetail } from "@/components/SessionDetail";
import { EditDialog } from "@/components/EditDialog";
import { SettingsDialog } from "@/components/SettingsDialog";
import { ipc } from "@/lib/ipc";
import { createT, detectLocale, type Locale } from "@/i18n";
import type { AppConfig, Session } from "@/types";

type EditMode = "rename" | "describe" | null;

function App() {
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

  const t = useMemo(() => createT(locale), [locale]);

  useEffect(() => {
    ipc.checkEnvironment().then((r) => {
      setCodexCliMissing(!r.codexCliFound);
      // codex CLI 있을 때만 자동 요약 워커 시작
      if (r.codexCliFound) {
        ipc.startAutoSummary().catch(() => {});
      }
    }).catch(() => {});

    // 자동 요약 진행될 때마다 목록 새로고침
    const unlisten = listen<string>("auto-summary-progress", () => {
      refresh();
    });
    return () => {
      unlisten.then((fn) => fn()).catch(() => {});
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
    refresh();
  }, [refresh]);

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

  async function handleDelete(s: Session) {
    const ok = confirm(t("prompt.confirmDelete"));
    if (!ok) return;
    try {
      await ipc.deleteSession(s.sessionId, s.filePath);
      setSelectedId((cur) => (cur === s.sessionId ? null : cur));
      await refresh();
    } catch (err) {
      console.error(err);
      alert(String(err));
    }
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
      // storage_type 분기:
      //  - cloud-only: 다운로드 (다른 PC에서 만든 세션 → 로컬로)
      //  - synced: 재동기화 (로컬 → 클라우드 덮어쓰기)
      //  - local-only / local: 첫 업로드
      //  - 레거시 "cloud": cloud-only로 취급
      const st = s.storageType;
      if (st === "cloud-only" || st === "cloud") {
        await ipc.checkoutSession(s);
      } else {
        // local-only 또는 synced → 둘 다 upload_session() 호출 (v0.4.4 동작: 로컬 유지 + 클라우드 덮어쓰기)
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
  const localCount = sessions.filter((s) => s.storageType !== "cloud").length;
  const cloudCount = sessions.filter((s) => s.storageType === "cloud").length;

  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      {codexCliMissing && (
        <div className="flex items-center gap-2 border-b border-amber-500/40 bg-amber-500/10 px-5 py-2 text-xs text-amber-300">
          <AlertTriangle className="h-4 w-4 shrink-0" />
          <span className="flex-1">
            {(t("warning.codexCliMissing") !== "warning.codexCliMissing"
              ? t("warning.codexCliMissing")
              : "Codex CLI not found on PATH. Resume actions won't work.")}
          </span>
          <a
            href="https://github.com/openai/codex"
            target="_blank"
            rel="noreferrer"
            className="underline hover:text-amber-200"
          >
            {(t("warning.codexCliInstall") !== "warning.codexCliInstall"
              ? t("warning.codexCliInstall")
              : "Install guide")}
          </a>
        </div>
      )}
      <header className="flex items-center gap-3 border-b border-border/60 px-5 py-3">
        <div className="flex flex-col">
          <h1 className="text-base font-semibold leading-tight">{t("app.title")}</h1>
          <p className="text-[11px] text-muted-foreground">
            {t("list.total", { count: total })} · local {localCount} / cloud {cloudCount}
          </p>
        </div>
        <div className="relative ml-6 flex-1 max-w-md">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search…"
            className="pl-8"
          />
        </div>
        <div className="flex items-center gap-1.5">
          <Button
            variant="ghost"
            size="icon"
            onClick={refresh}
            disabled={loading}
            title="Refresh"
          >
            <RefreshCw className={loading ? "animate-spin" : ""} />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setSettingsOpen(true)}
            title="Settings"
          >
            <SettingsIcon />
          </Button>
        </div>
      </header>

      <main className="flex flex-1 overflow-hidden">
        <section className="flex-1 overflow-auto">
          <SessionTable
            sessions={filtered}
            selectedId={selectedId}
            locale={locale}
            t={t}
            onSelect={(s) => setSelectedId(s.sessionId)}
            onResume={handleResume}
            onRename={(s) => openEdit("rename", s)}
            onDescribe={(s) => openEdit("describe", s)}
            onDelete={handleDelete}
            onToggleArchive={handleToggleArchive}
            onToggleCloud={handleToggleCloud}
            onGenerateSummary={handleGenerateSummary}
            onToggleFavorite={handleToggleFavorite}
          />
        </section>
        <aside className="w-[380px] shrink-0 border-l border-border/60 bg-card/30">
          <SessionDetail session={selected} locale={locale} t={t} onResume={handleResume} />
        </aside>
      </main>

      <EditDialog
        open={editMode !== null}
        title={editMode === "rename" ? t("action.rename") : t("action.describe")}
        label={editMode === "rename" ? t("prompt.enterName") : t("prompt.enterDescription")}
        initialValue={
          editTarget
            ? editMode === "rename"
              ? editTarget.name || ""
              : editTarget.description || ""
            : ""
        }
        onSubmit={submitEdit}
        onClose={() => {
          setEditMode(null);
          setEditTarget(null);
        }}
      />

      <SettingsDialog
        open={settingsOpen}
        current={config.settings}
        locale={locale}
        t={t}
        onClose={() => setSettingsOpen(false)}
        onSaved={refresh}
      />
    </div>
  );
}

export default App;
