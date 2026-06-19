export interface Session {
  sessionId: string;
  name: string | null;
  description: string | null;
  autoSummary: string | null;
  project: string;
  projectDir: string;
  filePath: string;
  size: number;
  totalLines: number;
  firstTimestamp: string | null;
  lastTimestamp: string | null;
  cwd: string | null;
  version: string | null;
  firstUserMessage: string | null;
  storageType: string;
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

export interface Settings {
  locale?: string | null;
  cloudPath?: string | null;
  anthropicApiKey?: string | null;
  preferredTerminal?: TerminalKind | "auto" | string | null;
  resumeFlags?: string | null;
  customTerminalProgram?: string | null;
  customTerminalArgs?: string | null;
  extraProjectDirs?: string[] | null;
  wslAutoDetect?: boolean | null;
  /** 스캔에서 제외할 경로/폴더명 substring 목록. 매치되는 프로젝트는 전부 스킵. */
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

export interface AppConfig {
  sessions: Record<string, SessionMeta>;
  settings: Settings;
}
