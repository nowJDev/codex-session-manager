import { useEffect, useRef, useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { Locale } from "@/i18n";
import type { CloudFolderCandidate } from "@/application/useSettingsActions";
import type { EnvironmentReport, Settings, TerminalKind, UpdateInfo } from "@/types";

type DebugLogInfo = { path: string; exists: boolean; size: number; tail: string };
type UpdateCheckResult = { info: UpdateInfo | null; error: string | null };

interface Props {
  open: boolean;
  current: Settings;
  locale: Locale;
  t: (k: string, p?: Record<string, string | number>) => string;
  onClose: () => void;
  onSaved: () => void;
  onPickDirectory: () => Promise<string | null>;
  onPickCloudFolder: () => Promise<CloudFolderCandidate | null>;
  onLoadDebugLog: () => Promise<DebugLogInfo>;
  onOpenDebugLogFolder: () => Promise<void>;
  onDetectGoogleDrive: () => Promise<CloudFolderCandidate | null>;
  onCheckEnvironment: () => Promise<EnvironmentReport>;
  onCheckForUpdates: (onProgress: (message: string) => void) => Promise<UpdateCheckResult>;
  onOpenReleases: (url?: string) => Promise<void>;
  onSaveSettings: (patch: Settings, cloudFolderRoot?: string) => Promise<void>;
}

const ALL_TERMINAL_OPTIONS: Array<{ value: "auto" | TerminalKind; labelKey: string; defaultLabel: string }> = [
  { value: "auto", labelKey: "settings.auto", defaultLabel: "Auto" },
  { value: "git-bash", labelKey: "", defaultLabel: "Git Bash" },
  { value: "wt", labelKey: "", defaultLabel: "Windows Terminal" },
  { value: "powershell", labelKey: "", defaultLabel: "PowerShell" },
  { value: "cmd", labelKey: "", defaultLabel: "Command Prompt" },
  { value: "terminal", labelKey: "", defaultLabel: "Terminal.app" },
  { value: "custom", labelKey: "settings.customTerminal", defaultLabel: "Custom..." },
];

function tx(t: Props["t"], key: string, fallback: string) {
  const v = t(key);
  return v === key ? fallback : v;
}

export function SettingsDialog({
  open,
  current,
  locale,
  t,
  onClose,
  onSaved,
  onPickDirectory,
  onPickCloudFolder,
  onLoadDebugLog,
  onOpenDebugLogFolder,
  onDetectGoogleDrive,
  onCheckEnvironment,
  onCheckForUpdates,
  onOpenReleases,
  onSaveSettings,
}: Props) {
  const [chosenLocale, setChosenLocale] = useState<string>(locale);
  const [savedCloudPath, setSavedCloudPath] = useState(current.cloudPath || "");
  const [cloudCandidate, setCloudCandidate] = useState<CloudFolderCandidate | null>(null);
  const [cloudBusy, setCloudBusy] = useState(false);
  const [cloudError, setCloudError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const wasOpen = useRef(false);
  const draftVersion = useRef(0);
  const cloudSelectionPending = useRef(false);
  const savePending = useRef(false);
  const [terminal, setTerminal] = useState<string>(current.preferredTerminal || "auto");
  const [flagBypass, setFlagBypass] = useState<boolean>(false);
  const [flagDebug, setFlagDebug] = useState<boolean>(false);
  const [flagVerbose, setFlagVerbose] = useState<boolean>(false);
  const [extraFlags, setExtraFlags] = useState<string>("");
  const [customProgram, setCustomProgram] = useState<string>(current.customTerminalProgram || "");
  const [customArgs, setCustomArgs] = useState<string>(current.customTerminalArgs || "");
  const [extraDirs, setExtraDirs] = useState<string[]>(current.extraProjectDirs || []);
  const [wslAuto, setWslAuto] = useState<boolean>(
    current.wslAutoDetect ?? true,
  );
  function parseFlags(raw: string): { bypass: boolean; debug: boolean; verbose: boolean; extra: string } {
    const tokens = raw.trim().split(/\s+/).filter(Boolean);
    let bypass = false, debug = false, verbose = false;
    const rest: string[] = [];
    for (const tk of tokens) {
      if (tk === "--dangerously-bypass-approvals-and-sandbox" || tk === "--dangerously-skip-permissions") bypass = true;
      else if (tk === "--debug") debug = true;
      else if (tk === "--verbose") verbose = true;
      else rest.push(tk);
    }
    return { bypass, debug, verbose, extra: rest.join(" ") };
  }

  function composeFlags(): string {
    const parts: string[] = [];
    if (flagBypass) parts.push("--dangerously-bypass-approvals-and-sandbox");
    if (flagDebug) parts.push("--debug");
    if (flagVerbose) parts.push("--verbose");
    const tail = extraFlags.trim();
    if (tail) parts.push(tail);
    return parts.join(" ");
  }
  const [report, setReport] = useState<EnvironmentReport | null>(null);
  const [diagLoading, setDiagLoading] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateLoading, setUpdateLoading] = useState(false);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [updateProgress, setUpdateProgress] = useState<string | null>(null);

  useEffect(() => {
    if (open && !wasOpen.current) {
      draftVersion.current++;
      setChosenLocale(current.locale || locale);
      setSavedCloudPath(current.cloudPath || "");
      setCloudCandidate(null);
      setCloudError(null);
      setSaveError(null);
      cloudSelectionPending.current = false;
      setCloudBusy(false);
      setTerminal(current.preferredTerminal || "auto");
      const parsed = parseFlags(current.resumeFlags || "");
      setFlagBypass(parsed.bypass);
      setFlagDebug(parsed.debug);
      setFlagVerbose(parsed.verbose);
      setExtraFlags(parsed.extra);
      setCustomProgram(current.customTerminalProgram || "");
      setCustomArgs(current.customTerminalArgs || "");
      setExtraDirs(current.extraProjectDirs || []);
      setWslAuto(current.wslAutoDetect ?? true);
      setReport(null);
      setUpdateInfo(null);
      setUpdateError(null);
      setUpdateProgress(null);
    }
    if (!open && wasOpen.current) draftVersion.current++;
    wasOpen.current = open;
  }, [open, current, locale]);

  async function selectCloudFolder(
    select: () => Promise<CloudFolderCandidate | null>,
    detecting = false,
  ) {
    if (savePending.current || cloudSelectionPending.current) return;
    const version = draftVersion.current;
    cloudSelectionPending.current = true;
    setCloudBusy(true);
    setCloudError(null);
    try {
      const candidate = await select();
      if (draftVersion.current !== version) return;
      if (candidate) setCloudCandidate(candidate);
      else if (detecting) setCloudError(tx(t, "settings.cloudDriveNotFound", "Google Drive was not detected. You can choose a folder manually."));
    } catch (err) {
      if (draftVersion.current === version) {
        setCloudError(`${tx(t, "settings.cloudFolderError", "Could not select a sync folder.")} ${String(err)}`);
      }
    } finally {
      if (draftVersion.current === version) {
        cloudSelectionPending.current = false;
        setCloudBusy(false);
      }
    }
  }

  const [debugLog, setDebugLog] = useState<DebugLogInfo | null>(null);

  async function loadDebugLog() {
    try {
      const r = await onLoadDebugLog();
      setDebugLog(r);
    } catch (err) {
      alert(String(err));
    }
  }

  async function openLogFolder() {
    try {
      await onOpenDebugLogFolder();
    } catch (err) {
      alert(String(err));
    }
  }

  async function copyLogTail() {
    if (!debugLog?.tail) return;
    try {
      await navigator.clipboard.writeText(debugLog.tail);
      alert("로그 마지막 부분을 클립보드에 복사했어요. GitHub Issue에 붙여넣어 주세요.");
    } catch (err) {
      alert(String(err));
    }
  }

  async function runDiagnostics() {
    setDiagLoading(true);
    try {
      const r = await onCheckEnvironment();
      setReport(r);
    } catch (err) {
      console.error(err);
      alert(String(err));
    } finally {
      setDiagLoading(false);
    }
  }

  async function checkForUpdates() {
    setUpdateLoading(true);
    setUpdateError(null);
    setUpdateProgress(null);
    try {
      const result = await onCheckForUpdates(setUpdateProgress);
      if (result.info) setUpdateInfo(result.info);
      if (result.error) setUpdateError(result.error);
    } finally {
      setUpdateLoading(false);
    }
  }

  async function openReleases(url?: string) {
    await onOpenReleases(url);
  }

  async function save() {
    if (savePending.current || cloudSelectionPending.current) return;
    savePending.current = true;
    setSaving(true);
    setSaveError(null);
    try {
      await onSaveSettings({
        locale: chosenLocale,
        preferredTerminal: terminal,
        resumeFlags: composeFlags() || null,
        customTerminalProgram: customProgram || null,
        customTerminalArgs: customArgs || null,
        extraProjectDirs: extraDirs,
        wslAutoDetect: wslAuto,
        excludedScanPaths: current.excludedScanPaths,
      }, cloudCandidate?.root);
    } catch (err) {
      setSaveError(`${tx(t, "settings.saveError", "Could not save your settings. Try again.")} ${String(err)}`);
      return;
    } finally {
      savePending.current = false;
      setSaving(false);
    }
    onSaved();
    onClose();
  }

  return (
    <Dialog open={open} onOpenChange={(v) => !v && !savePending.current && onClose()}>
      <DialogContent
        className="flex max-h-[85vh] max-w-xl flex-col gap-0 overflow-hidden p-0"
        closeLabel={tx(t, "common.close", "Close")}
      >
        <DialogHeader className="shrink-0 border-b border-border/60 px-6 py-4 pr-12">
          <DialogTitle>{tx(t, "settings.title", "Settings")}</DialogTitle>
        </DialogHeader>

        <div className="min-h-0 space-y-5 overflow-y-auto px-6 py-4">
          <div className="space-y-2">
            <Label>{tx(t, "settings.language", "Language")}</Label>
            <div className="flex gap-2">
              {(["en", "ko"] as const).map((l) => (
                <Button
                  key={l}
                  variant={chosenLocale === l ? "default" : "outline"}
                  size="sm"
                  onClick={() => setChosenLocale(l)}
                >
                  {l.toUpperCase()}
                </Button>
              ))}
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="settings-preferred-terminal">{tx(t, "settings.preferredTerminal", "Preferred terminal")}</Label>
            <select
              id="settings-preferred-terminal"
              value={terminal}
              onChange={(e) => setTerminal(e.target.value)}
              className="w-full h-9 rounded-md border border-input bg-background px-3 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
            >
              {ALL_TERMINAL_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.labelKey ? tx(t, opt.labelKey, opt.defaultLabel) : opt.defaultLabel}
                  {report && opt.value !== "auto" && opt.value !== "custom" &&
                    !report.terminals.some((d) => d.kind === opt.value)
                    ? ` (${tx(t, "settings.terminalNotDetected", "Not detected")})`
                    : ""}
                </option>
              ))}
            </select>
            <p className="text-xs text-muted-foreground">
              {tx(t, "settings.preferredTerminalHelp", "Auto picks the first available terminal.")}
            </p>
          </div>

          {terminal === "custom" && (
            <div className="space-y-2 rounded-md border border-border/60 bg-muted/20 p-3">
              <Label>{tx(t, "settings.customTerminal", "Custom terminal")}</Label>
              <div className="space-y-1">
                <Label htmlFor="settings-custom-program" className="text-xs text-muted-foreground">Program (exe path)</Label>
                <Input
                  id="settings-custom-program"
                  value={customProgram}
                  onChange={(e) => setCustomProgram(e.target.value)}
                  placeholder='C:\Program Files\Git\usr\bin\mintty.exe'
                />
              </div>
              <div className="space-y-1">
                <Label htmlFor="settings-custom-args" className="text-xs text-muted-foreground">
                  Args template — tokens: <code>{`{cwd}`}</code>, <code>{`{id}`}</code>, <code>{`{flags}`}</code>, <code>{`{codex_invoke}`}</code>
                </Label>
                <Input
                  id="settings-custom-args"
                  value={customArgs}
                  onChange={(e) => setCustomArgs(e.target.value)}
                  placeholder='-e bash -c "cd {cwd} && {codex_invoke}; exec bash"'
                />
              </div>
            </div>
          )}

          <div className="space-y-3">
            <Label>Resume options</Label>
            <p className="text-xs text-muted-foreground -mt-1">
              세션 실행 시 <code>codex resume</code>에 전달되는 시작 플래그. 실행 후 변경 불가한 옵션만 노출.
            </p>

            <label className="flex items-start gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={flagBypass}
                onChange={(e) => setFlagBypass(e.target.checked)}
                className="mt-0.5 accent-primary"
              />
              <div className="flex-1">
                <div className="text-sm font-medium">
                  Bypass approvals and sandbox <span className="font-mono text-xs text-muted-foreground">--dangerously-bypass-approvals-and-sandbox</span>
                </div>
                <div className="text-xs text-muted-foreground">
                  승인과 샌드박스를 모두 건너뜀. 신뢰하는 작업만 켜기.
                </div>
              </div>
            </label>

            <label className="flex items-start gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={flagDebug}
                onChange={(e) => setFlagDebug(e.target.checked)}
                className="mt-0.5 accent-primary"
              />
              <div className="flex-1">
                <div className="text-sm font-medium">
                  Debug <span className="font-mono text-xs text-muted-foreground">--debug</span>
                </div>
                <div className="text-xs text-muted-foreground">
                  내부 디버그 로그 출력. 문제 진단용.
                </div>
              </div>
            </label>

            <label className="flex items-start gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={flagVerbose}
                onChange={(e) => setFlagVerbose(e.target.checked)}
                className="mt-0.5 accent-primary"
              />
              <div className="flex-1">
                <div className="text-sm font-medium">
                  Verbose <span className="font-mono text-xs text-muted-foreground">--verbose</span>
                </div>
                <div className="text-xs text-muted-foreground">
                  상세 출력. 도구 호출/응답이 더 자세히 표시됨.
                </div>
              </div>
            </label>

            <div className="space-y-1 pt-1">
              <span className="text-xs text-muted-foreground">
                Additional flags (자유 입력 — <code>--sandbox</code>, <code>--ask-for-approval</code>, <code>--model</code> 등 특수 케이스용)
              </span>
              <Input
                value={extraFlags}
                onChange={(e) => setExtraFlags(e.target.value)}
                placeholder="--sandbox workspace-write --ask-for-approval on-request"
              />
            </div>

            <div className="rounded bg-muted/30 px-2 py-1.5 font-mono text-[11px] text-muted-foreground">
              <span className="text-foreground/70">미리보기:</span>{" "}
              codex resume {composeFlags() || <span className="italic">(플래그 없음)</span>} &lt;id&gt;
            </div>
          </div>

          <div className="space-y-2">
            <Label>추가 세션 경로 (WSL · 외부 디렉토리)</Label>
            <p className="text-xs text-muted-foreground">
              기본 <code>~/.codex/sessions</code>와 <code>~/.codex/archived_sessions</code> 외에 추가로 스캔할 폴더.
            </p>

            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={wslAuto}
                onChange={(e) => setWslAuto(e.target.checked)}
                className="accent-primary"
              />
              <span className="text-sm">WSL 배포판 자동 탐지</span>
            </label>

            <div className="space-y-1">
              {extraDirs.length === 0 ? (
                <p className="text-xs text-muted-foreground italic">추가된 경로 없음</p>
              ) : (
                extraDirs.map((p, i) => (
                  <div key={i} className="flex items-center gap-2 text-xs">
                    <span className="flex-1 font-mono break-all rounded bg-muted/30 px-2 py-1">
                      {p}
                    </span>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() =>
                        setExtraDirs(extraDirs.filter((_, idx) => idx !== i))
                      }
                    >
                      제거
                    </Button>
                  </div>
                ))
              )}
              <Button
                variant="outline"
                size="sm"
                onClick={async () => {
                  const result = await onPickDirectory();
                  if (result && !extraDirs.includes(result)) {
                    setExtraDirs([...extraDirs, result]);
                  }
                }}
              >
                폴더 추가
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="settings-cloud-folder">{tx(t, "settings.cloudSync", "Sync folder")}</Label>
            <Input
              id="settings-cloud-folder"
              value={cloudCandidate?.path ?? savedCloudPath}
              title={cloudCandidate?.path ?? savedCloudPath}
              aria-describedby="settings-cloud-help"
              placeholder={tx(t, "settings.cloudUnconfigured", "No sync folder configured")}
              readOnly
            />
            <div className="flex flex-wrap items-center gap-2">
              <Button
                variant="outline"
                onClick={() => selectCloudFolder(onDetectGoogleDrive, true)}
                disabled={cloudBusy || saving}
              >
                {tx(t, "settings.cloudDetect", "Detect Google Drive")}
              </Button>
              <Button
                variant="outline"
                onClick={() => selectCloudFolder(onPickCloudFolder)}
                disabled={cloudBusy || saving}
              >
                {tx(t, "settings.cloudPick", "Choose folder")}
              </Button>
            </div>
            <p id="settings-cloud-help" className="text-xs text-muted-foreground">
              {tx(t, "settings.cloudHelp", "Save creates the Codex Sessions folder if needed. Sync copies supported sessions and keeps their local originals.")}
            </p>
            {cloudCandidate && (
              <p role="status" className="text-xs text-muted-foreground">
                {tx(t, "settings.cloudPending", "This folder will be used after you save.")}
              </p>
            )}
            {cloudError && <p role="alert" className="break-words text-xs text-destructive">{cloudError}</p>}
          </div>

          <div className="space-y-2 border-t border-border/60 pt-4">
            <div className="flex items-center justify-between">
              <Label>{tx(t, "settings.diagnostics", "Environment diagnostics")}</Label>
              <Button variant="outline" size="sm" onClick={runDiagnostics} disabled={diagLoading}>
                {diagLoading
                  ? tx(t, "settings.running", "Running...")
                  : tx(t, "settings.runDiagnostics", "Run diagnostics")}
              </Button>
            </div>
            {report && (
              <div className="rounded-md border border-border/60 bg-muted/30 p-3 text-xs space-y-2">
                <div>
                  {report.codexCliFound ? (
                    <div className="space-y-0.5">
                      <div className="text-green-500">
                        ✓ {tx(t, "settings.codexCliFound", "Codex CLI: {path}").replace(
                          "{path}",
                          report.codexCliPath || "",
                        )}
                      </div>
                      {report.codexCliVersion && (
                        <div className="text-muted-foreground">
                          {tx(t, "settings.codexCliVersion", "version {version}").replace(
                            "{version}",
                            report.codexCliVersion,
                          )}
                        </div>
                      )}
                    </div>
                  ) : (
                    <div className="text-red-500">
                      ✗ {tx(t, "settings.codexCliMissing", "Codex CLI not found on PATH")}
                    </div>
                  )}
                </div>
                <div>
                  <div className="font-medium mb-1">
                    {tx(t, "settings.detectedTerminals", "Detected terminals")} ({report.terminals.length})
                  </div>
                  {report.terminals.length === 0 ? (
                    <div className="text-muted-foreground">
                      {tx(t, "settings.noTerminalsFound", "No terminals detected")}
                    </div>
                  ) : (
                    <ul className="space-y-0.5">
                      {report.terminals.map((d) => (
                        <li key={d.kind} className="font-mono">
                          <span className="text-foreground">{d.displayName}</span>
                          <span className="text-muted-foreground"> — {d.program}</span>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              </div>
            )}
          </div>

          <div className="space-y-2 border-t border-border/60 pt-4">
            <div className="flex items-center justify-between gap-3">
              <Label>업데이트</Label>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={checkForUpdates}
                  disabled={updateLoading}
                >
                  {updateLoading ? "진행 중..." : "확인 및 자동 업데이트"}
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => openReleases(updateInfo?.releaseUrl)}
                >
                  portable 릴리즈 열기
                </Button>
              </div>
            </div>
            <div className="rounded-md border border-border/60 bg-muted/30 p-3 text-xs">
              {updateInfo ? (
                <div className="space-y-1">
                  <div>
                    현재 버전 <span className="font-mono">{updateInfo.currentVersion}</span>
                    {" / "}
                    최신 버전 <span className="font-mono">{updateInfo.latestVersion}</span>
                  </div>
                  <div className={updateInfo.hasUpdate ? "text-amber-400" : "text-green-500"}>
                    {updateInfo.hasUpdate
                      ? "설치본은 자동 다운로드와 설치를 진행합니다. portable은 릴리즈 페이지에서 zip을 내려받아 교체하세요."
                      : "현재 최신 릴리즈를 사용 중입니다."}
                  </div>
                  {updateProgress && (
                    <div className="text-muted-foreground">{updateProgress}</div>
                  )}
                </div>
              ) : updateError ? (
                <div className="space-y-1 text-amber-400">
                  <div>업데이트 정보를 가져오지 못했습니다.</div>
                  <div className="break-all font-mono text-[11px] text-muted-foreground">
                    {updateError}
                  </div>
                </div>
              ) : (
                <div className="text-muted-foreground">
                  설치본은 자동 업데이트를 시도하고, portable은 릴리즈 페이지를 엽니다.
                </div>
              )}
              {!updateInfo && updateProgress && (
                <div className="mt-1 text-muted-foreground">{updateProgress}</div>
              )}
            </div>
          </div>

          <div className="space-y-2 border-t border-border/60 pt-4">
            <div className="flex items-center justify-between">
              <Label>문제 신고 / 디버그 로그</Label>
              <Button variant="outline" size="sm" onClick={loadDebugLog}>
                로그 불러오기
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              세션 실행/자동 요약 시 무엇이 spawn되는지 자세한 로그를 남깁니다. 문제가 생기면 아래 로그와 함께{" "}
              <a
                href="https://github.com/nowJDev/codex-session-manager/issues/new"
                target="_blank"
                rel="noreferrer"
                className="underline hover:text-foreground"
              >
                GitHub Issue
              </a>
              에 등록해주세요.
            </p>
            {debugLog && (
              <div className="space-y-2 rounded-md border border-border/60 bg-muted/30 p-3">
                <div className="flex flex-wrap items-center gap-2 text-xs">
                  <span className="font-mono text-muted-foreground break-all">{debugLog.path}</span>
                  <span className="text-muted-foreground">
                    ({debugLog.exists ? `${(debugLog.size / 1024).toFixed(1)} KB` : "파일 없음"})
                  </span>
                </div>
                <div className="flex gap-2">
                  <Button variant="outline" size="sm" onClick={openLogFolder}>
                    폴더 열기
                  </Button>
                  {debugLog.exists && (
                    <Button variant="outline" size="sm" onClick={copyLogTail}>
                      마지막 부분 복사
                    </Button>
                  )}
                </div>
                {debugLog.tail && (
                  <pre className="max-h-40 overflow-auto rounded bg-background/80 p-2 text-[10px] leading-tight font-mono whitespace-pre-wrap break-all">
                    {debugLog.tail}
                  </pre>
                )}
              </div>
            )}
          </div>
        </div>

        <DialogFooter className="shrink-0 flex-wrap items-center gap-2 border-t border-border/60 px-6 py-3">
          {saveError && <p role="alert" className="mr-auto max-h-20 min-w-0 basis-full overflow-auto break-words text-sm text-destructive">{saveError}</p>}
          <Button variant="outline" onClick={onClose} disabled={saving}>
            {tx(t, "settings.cancel", "Cancel")}
          </Button>
          <Button onClick={save} disabled={saving || cloudBusy}>
            {saving ? tx(t, "settings.saving", "Saving…") : tx(t, "settings.save", "Save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
