import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  EnvironmentReport,
  Session,
  SessionMeta,
  Settings,
} from "@/types";

export const ipc = {
  listSessions: () => invoke<Session[]>("list_sessions"),
  getConfig: () => invoke<AppConfig>("get_config_cmd"),
  saveSessionMeta: (sessionId: string, patch: SessionMeta) =>
    invoke<void>("save_session_meta", { sessionId, patch }),
  deleteSession: (sessionId: string, filePath: string) =>
    invoke<void>("delete_session", { sessionId, filePath }),
  saveSettings: (patch: Settings) => invoke<void>("save_settings", { patch }),
  setCloudFolder: (root: string) =>
    invoke<string>("set_cloud_folder", { root }),
  uploadToCloud: (session: Session) =>
    invoke<void>("upload_to_cloud", { session }),
  checkoutSession: (session: Session) =>
    invoke<string>("checkout_session", { session }),
  checkinSession: (session: Session) =>
    invoke<void>("checkin_session", { session }),
  resumeSession: (sessionId: string, cwd: string | null) =>
    invoke<void>("resume_session", { sessionId, cwd }),
  generateSummary: (sessionId: string, filePath: string) =>
    invoke<string>("generate_summary_cmd", { sessionId, filePath }),
  checkEnvironment: () => invoke<EnvironmentReport>("check_environment_cmd"),
  startAutoSummary: () => invoke<boolean>("start_auto_summary"),
  detectGoogleDrive: () => invoke<{ found: boolean; path: string | null }>("detect_google_drive_cmd"),
  connectGoogleDrive: () => invoke<string>("connect_google_drive_cmd"),
  getDebugLog: () =>
    invoke<{ path: string; exists: boolean; size: number; tail: string }>("get_debug_log_cmd"),
  openDebugLogFolder: () => invoke<void>("open_debug_log_folder_cmd"),
};
