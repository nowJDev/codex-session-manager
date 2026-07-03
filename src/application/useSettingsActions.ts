// 설정 화면의 외부 연동 작업을 애플리케이션 계층에서 실행한다.
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import type { DownloadEvent } from "@tauri-apps/plugin-updater";
import { ipc } from "@/lib/ipc";
import type { EnvironmentReport, Settings, UpdateInfo } from "@/types";

const RELEASES_URL = "https://github.com/nowJDev/codex-session-manager/releases";
const CODEX_WEB_URL = "https://chatgpt.com/codex/settings/usage";

export type DebugLogInfo = {
  path: string;
  exists: boolean;
  size: number;
  tail: string;
};

export type UpdateCheckResult = {
  info: UpdateInfo | null;
  error: string | null;
};

function progressText(downloaded: number, total?: number): string {
  if (!total) return `${(downloaded / 1024 / 1024).toFixed(1)} MB 다운로드 중...`;
  const pct = Math.min(100, Math.round((downloaded / total) * 100));
  return `${pct}% 다운로드 중 (${(downloaded / 1024 / 1024).toFixed(1)} / ${(total / 1024 / 1024).toFixed(1)} MB)`;
}

export function useSettingsActions() {
  async function pickDirectory(): Promise<string | null> {
    const result = await openDialog({ directory: true, multiple: false });
    return typeof result === "string" ? result : null;
  }

  async function pickCloudFolder(): Promise<string | null> {
    const result = await pickDirectory();
    return result ? ipc.setCloudFolder(result) : null;
  }

  async function checkForUpdates(
    onProgress: (message: string) => void,
  ): Promise<UpdateCheckResult> {
    try {
      const releaseInfo = await ipc.checkUpdate().catch(() => null);
      const update = await check({ timeout: 30000 });
      if (!update) {
        onProgress("현재 최신 릴리즈를 사용 중입니다.");
        return { info: releaseInfo, error: null };
      }

      const info = {
        currentVersion: update.currentVersion,
        latestVersion: update.version,
        hasUpdate: true,
        releaseUrl:
          releaseInfo?.releaseUrl ||
          "https://github.com/nowJDev/codex-session-manager/releases/latest",
      };
      onProgress(`${update.version} 업데이트를 다운로드합니다.`);

      let downloaded = 0;
      let contentLength: number | undefined;
      await update.downloadAndInstall((event: DownloadEvent) => {
        switch (event.event) {
          case "Started":
            downloaded = 0;
            contentLength = event.data.contentLength;
            onProgress("다운로드를 시작합니다.");
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            onProgress(progressText(downloaded, contentLength));
            break;
          case "Finished":
            onProgress("다운로드 완료. 업데이트를 설치합니다.");
            break;
        }
      });

      onProgress("업데이트 설치 완료. 앱을 재시작합니다.");
      await relaunch();
      return { info, error: null };
    } catch (err) {
      const fallback = await ipc.checkUpdate().catch(() => null);
      return {
        info: fallback,
        error: `${String(err)}\n설치본 자동 업데이트를 사용할 수 없으면 portable은 릴리즈 열기로 업데이트하세요.`,
      };
    }
  }

  return {
    pickDirectory,
    pickCloudFolder,
    loadDebugLog: () => ipc.getDebugLog() as Promise<DebugLogInfo>,
    openDebugLogFolder: () => ipc.openDebugLogFolder(),
    connectGoogleDrive: () => ipc.connectGoogleDrive(),
    checkEnvironment: () => ipc.checkEnvironment() as Promise<EnvironmentReport>,
    checkForUpdates,
    openReleases: (url?: string) => openUrl(url || RELEASES_URL),
    openUsagePage: () => openUrl(CODEX_WEB_URL),
    saveSettings: (patch: Settings) => ipc.saveSettings(patch),
  };
}
