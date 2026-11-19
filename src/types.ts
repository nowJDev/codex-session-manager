// 세션과 설정 및 삭제 API의 프런트엔드 계약을 정의한다.
export type SessionKind = "main" | "worker" | "guardian" | "otherInternal" | "unknown";

export interface SessionSnapshot {
  sessions: Session[];
  complete: boolean;
  missingCount: number;
  errors: string[];
}

export interface Session {
  sessionId: string;
  parentId: string | null;
  isSubagent: boolean;
  agentNickname: string | null;
  name: string | null;
  description: string | null;
  autoSummary: string | null;
  project: string;
  projectDir: string;
  filePath: string | null;
  size: number | null;
  preview: string | null;
  historyMode: string;
  registered: boolean;
  sessionKind: SessionKind;
  isSummaryRun: boolean;
  totalLines: number;
  firstTimestamp: string | null;
  lastTimestamp: string | null;
  cwd: string | null;
  version: string | null;
  firstUserMessage: string | null;
  storageType: string;
  archived: boolean;
  favorite: boolean;
}

export interface SessionMeta {
  name?: string | null;
  description?: string | null;
  autoSummary?: string | null;
  storageType?: string | null;
  updatedAt?: string | null;
  favorite?: boolean | null;
}

export interface DeleteSessionTarget {
  sessionId: string;
  guardianOnly?: boolean;
  activityBefore?: number;
}

export type DeleteProgress =
  | { phase: "checkingScope" }
  | { phase: "deleting"; processed: number; total: number; elapsedMs: number }
  | { phase: "verifying" };

export type DeleteSessionStatus = "deleted" | "alreadyMissing" | "failed" | "unconfirmed";

export interface DeletionPlan {
  targets: DeleteSessionTarget[];
  affectedIds: string[];
  skippedGuardianCount?: number;
  skippedAgeCount?: number;
}

export interface DeleteSessionResult extends DeleteSessionTarget {
  status: DeleteSessionStatus;
  error: string | null;
  warning: string | null;
}

export interface Settings {
  locale?: string | null;
  cloudPath?: string | null;
  preferredTerminal?: TerminalKind | "auto" | string | null;
  resumeFlags?: string | null;
  customTerminalProgram?: string | null;
  customTerminalArgs?: string | null;
  extraProjectDirs?: string[] | null;
  wslAutoDetect?: boolean | null;
  /** 자동요약 제외 정책과 과거 설정 호환을 위해 보존하는 경로 목록. */
  excludedScanPaths?: string[] | null;
}

export type TerminalKind =
  | "git-bash"
  | "wt"
  | "powershell"
  | "cmd"
  | "terminal"
  | "linux-default"
  | "custom";

export interface DetectedTerminal {
  kind: TerminalKind;
  program: string;
  displayName: string;
}

export interface EnvironmentReport {
  targetOs: "windows" | "macos" | "linux" | string;
  codexCliFound: boolean;
  codexCliPath: string | null;
  codexCliVersion: string | null;
  terminals: DetectedTerminal[];
}

export interface CodexLimitStatus {
  key: string;
  label: string;
  available: boolean;
  value: string | null;
  detail: string;
}

export interface CodexStatus {
  checkedAt: string;
  cliFound: boolean;
  cliPath: string | null;
  cliVersion: string | null;
  model: string | null;
  modelReasoningEffort: string | null;
  statusLine: string[];
  limits: CodexLimitStatus[];
  note: string;
}

export interface UpdateInfo {
  currentVersion: string;
  latestVersion: string;
  hasUpdate: boolean;
  releaseUrl: string;
}

export interface AppConfig {
  sessions: Record<string, SessionMeta>;
  settings: Settings;
}
