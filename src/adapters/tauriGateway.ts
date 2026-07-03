// Tauri 런타임과 프론트엔드 application hook 사이의 gateway를 제공한다.
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import type { DownloadEvent } from "@tauri-apps/plugin-updater";
import { ipc } from "@/lib/ipc";
import type {
  AppConfig,
  CodexStatus,
  DeleteSessionTarget,
  EnvironmentReport,
  Session,
  SessionMeta,
  Settings,
  UpdateInfo,
} from "@/types";

export type DebugLogInfo = {
  path: string;
  exists: boolean;
  size: number;
  tail: string;
};

export type InstallerDownloadEvent = DownloadEvent;

export type InstallerUpdate = {
  currentVersion: string;
  version: string;
  downloadAndInstall: (onEvent: (event: InstallerDownloadEvent) => void) => Promise<void>;
};

async function checkInstallerUpdate(): Promise<InstallerUpdate | null> {
  const update = await check({ timeout: 30000 });
  if (!update) return null;
  return {
    currentVersion: update.currentVersion,
    version: update.version,
    downloadAndInstall: (onEvent) => update.downloadAndInstall(onEvent),
  };
}

export const tauriGateway = {
  listSessions: (): Promise<Session[]> => ipc.listSessions(),
  getConfig: (): Promise<AppConfig> => ipc.getConfig(),
  saveSessionMeta: (sessionId: string, patch: SessionMeta): Promise<void> =>
    ipc.saveSessionMeta(sessionId, patch),
  deleteSession: (sessionId: string, filePath: string): Promise<void> =>
    ipc.deleteSession(sessionId, filePath),
  deleteSessions: (targets: DeleteSessionTarget[]): Promise<void> => ipc.deleteSessions(targets),
  archiveSession: (sessionId: string): Promise<void> => ipc.archiveSession(sessionId),
  unarchiveSession: (sessionId: string): Promise<void> => ipc.unarchiveSession(sessionId),
  saveSettings: (patch: Settings): Promise<void> => ipc.saveSettings(patch),
  setCloudFolder: (root: string): Promise<string> => ipc.setCloudFolder(root),
  uploadToCloud: (session: Session): Promise<void> => ipc.uploadToCloud(session),
  checkoutSession: (session: Session): Promise<string> => ipc.checkoutSession(session),
  checkinSession: (session: Session): Promise<void> => ipc.checkinSession(session),
  resumeSession: (sessionId: string, cwd: string | null): Promise<void> =>
    ipc.resumeSession(sessionId, cwd),
  generateSummary: (sessionId: string, filePath: string): Promise<string> =>
    ipc.generateSummary(sessionId, filePath),
  checkEnvironment: (): Promise<EnvironmentReport> => ipc.checkEnvironment(),
  getCodexStatus: (): Promise<CodexStatus> => ipc.getCodexStatus(),
  checkUpdate: (): Promise<UpdateInfo> => ipc.checkUpdate(),
  startAutoSummary: (): Promise<boolean> => ipc.startAutoSummary(),
  detectGoogleDrive: (): Promise<{ found: boolean; path: string | null }> =>
    ipc.detectGoogleDrive(),
  connectGoogleDrive: (): Promise<string> => ipc.connectGoogleDrive(),
  getDebugLog: (): Promise<DebugLogInfo> => ipc.getDebugLog(),
  openDebugLogFolder: (): Promise<void> => ipc.openDebugLogFolder(),
  pickDirectory: async (): Promise<string | null> => {
    const result = await openDialog({ directory: true, multiple: false });
    return typeof result === "string" ? result : null;
  },
  openExternalUrl: (url: string): Promise<void> => openUrl(url),
  checkInstallerUpdate,
  relaunchApp: (): Promise<void> => relaunch(),
  listenAutoSummaryProgress: (
    handler: (sessionId: string) => void,
  ): Promise<UnlistenFn> => listen<string>("auto-summary-progress", (event) => handler(event.payload)),
};
