import { useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
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
import { ipc } from "@/lib/ipc";
import type { Locale } from "@/i18n";
import type { EnvironmentReport, Settings, TerminalKind } from "@/types";

interface Props {
  open: boolean;
  current: Settings;
  locale: Locale;
  t: (k: string, p?: Record<string, string | number>) => string;
  onClose: () => void;
  onSaved: () => void;
}

const ALL_TERMINAL_OPTIONS: Array<{ value: "auto" | TerminalKind; labelKey: string; defaultLabel: string }> = [
  { value: "auto", labelKey: "settings.auto", defaultLabel: "Auto" },
  { value: "git-bash", labelKey: "", defaultLabel: "Git Bash" },
  { value: "wt", labelKey: "", defaultLabel: "Windows Terminal" },
  { value: "powershell", labelKey: "", defaultLabel: "PowerShell" },
  { value: "cmd", labelKey: "", defaultLabel: "Command Prompt" },
  { value: "terminal", labelKey: "", defaultLabel: "Terminal.app" },
  { value: "custom", labelKey: "", defaultLabel: "Custom..." },
];

function tx(t: Props["t"], key: string, fallback: string) {
  const v = t(key);
  return v === key ? fallback : v;
}

export function SettingsDialog({ open, current, locale, t, onClose, onSaved }: Props) {
  const [chosenLocale, setChosenLocale] = useState<string>(locale);
  const [cloudPath, setCloudPath] = useState<string>(current.cloudPath || "");
  const [terminal, setTerminal] = useState<string>(current.preferredTerminal || "auto");
  const [flagBypass, setFlagBypass] = useState<boolean>(false);
  const [flagDebug, setFlagDebug] = useState<boolean>(false);
  const [flagVerbose, setFlagVerbose] = useState<boolean>(false);
  const [extraFlags, setExtraFlags] = useState<string>("");
  const [customProgram, setCustomProgram] = useState<string>(current.customTerminalProgram || "");
  const [customArgs, setCustomArgs] = useState<string>(current.customTerminalArgs || "");

  function parseFlags(raw: string): { bypass: boolean; debug: boolean; verbose: boolean; extra: string } {
    const tokens = raw.trim().split(/\s+/).filter(Boolean);
    let bypass = false, debug = false, verbose = false;
    const rest: string[] = [];
    for (const tk of tokens) {
      if (tk === "--dangerously-skip-permissions") bypass = true;
      else if (tk === "--debug") debug = true;
      else if (tk === "--verbose") verbose = true;
      else rest.push(tk);
    }
    return { bypass, debug, verbose, extra: rest.join(" ") };
  }

  function composeFlags(): string {
    const parts: string[] = [];
    if (flagBypass) parts.push("--dangerously-skip-permissions");
    if (flagDebug) parts.push("--debug");
    if (flagVerbose) parts.push("--verbose");
    const tail = extraFlags.trim();
    if (tail) parts.push(tail);
    return parts.join(" ");
  }
  const [report, setReport] = useState<EnvironmentReport | null>(null);
  const [diagLoading, setDiagLoading] = useState(false);

  useEffect(() => {
    if (open) {
      setChosenLocale(current.locale || locale);
      setCloudPath(current.cloudPath || "");
      setTerminal(current.preferredTerminal || "auto");
      const parsed = parseFlags(current.resumeFlags || "");
      setFlagBypass(parsed.bypass);
      setFlagDebug(parsed.debug);
      setFlagVerbose(parsed.verbose);
      setExtraFlags(parsed.extra);
      setCustomProgram(current.customTerminalProgram || "");
      setCustomArgs(current.customTerminalArgs || "");
      setReport(null);
    }
  }, [open, current, locale]);

  async function pickFolder() {
    const result = await openDialog({ directory: true, multiple: false });
    if (typeof result === "string") {
      const saved = await ipc.setCloudFolder(result);
      setCloudPath(saved);
    }
  }

  const [debugLog, setDebugLog] = useState<{ path: string; exists: boolean; size: number; tail: string } | null>(null);

  async function loadDebugLog() {
    try {
      const r = await ipc.getDebugLog();
      setDebugLog(r);
    } catch (err) {
      alert(String(err));
    }
  }

  async function openLogFolder() {
    try {
      await ipc.openDebugLogFolder();
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

  async function autoConnectGoogleDrive() {
    try {
      const saved = await ipc.connectGoogleDrive();
      setCloudPath(saved);
    } catch (err) {
      alert(String(err));
    }
  }

  async function runDiagnostics() {
    setDiagLoading(true);
    try {
      const r = await ipc.checkEnvironment();
      setReport(r);
    } catch (err) {
      console.error(err);
      alert(String(err));
    } finally {
      setDiagLoading(false);
    }
  }

  async function save() {
    await ipc.saveSettings({
      locale: chosenLocale,
      cloudPath: cloudPath || null,
      preferredTerminal: terminal,
      resumeFlags: composeFlags() || null,
      customTerminalProgram: customProgram || null,
      customTerminalArgs: customArgs || null,
    });
    onSaved();
    onClose();
  }

  // Filter terminal options to ones plausible for current OS, if known
  const availableOptions = report
    ? ALL_TERMINAL_OPTIONS.filter(
        (opt) =>
          opt.value === "auto" ||
          report.terminals.some((d) => d.kind === opt.value),
      )
    : ALL_TERMINAL_OPTIONS;

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="max-w-xl max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{tx(t, "settings.title", "Settings")}</DialogTitle>
        </DialogHeader>

        <div className="space-y-5">
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
            <Label>{tx(t, "settings.preferredTerminal", "Preferred terminal")}</Label>
            <select
              value={terminal}
              onChange={(e) => setTerminal(e.target.value)}
              className="w-full h-9 rounded-md border border-input bg-background px-3 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
            >
              {availableOptions.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.value === "auto"
                    ? tx(t, "settings.auto", "Auto")
                    : opt.defaultLabel}
                </option>
              ))}
            </select>
            <p className="text-xs text-muted-foreground">
              {tx(t, "settings.preferredTerminalHelp", "Auto picks the first available terminal.")}
            </p>
          </div>

          {terminal === "custom" && (
            <div className="space-y-2 rounded-md border border-border/60 bg-muted/20 p-3">
              <Label>Custom terminal</Label>
              <div className="space-y-1">
                <span className="text-xs text-muted-foreground">Program (exe path)</span>
                <Input
                  value={customProgram}
                  onChange={(e) => setCustomProgram(e.target.value)}
                  placeholder='C:\Program Files\Git\usr\bin\mintty.exe'
                />
              </div>
              <div className="space-y-1">
                <span className="text-xs text-muted-foreground">
                  Args template — tokens: <code>{`{cwd}`}</code>, <code>{`{id}`}</code>, <code>{`{flags}`}</code>, <code>{`{claude_invoke}`}</code>
                </span>
                <Input
                  value={customArgs}
                  onChange={(e) => setCustomArgs(e.target.value)}
                  placeholder='-e bash -c "cd {cwd} && {claude_invoke}; exec bash"'
                />
              </div>
            </div>
          )}

          <div className="space-y-3">
            <Label>Resume options</Label>
            <p className="text-xs text-muted-foreground -mt-1">
              세션 실행 시 <code>claude</code>에 전달되는 시작 플래그. 실행 후 변경 불가한 옵션만 노출.
            </p>

            <label className="flex items-start gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={flagBypass}
                onChange={(e) => setFlagBypass(e.target.checked)}
                className="mt-0.5"
              />
              <div className="flex-1">
                <div className="text-sm font-medium">
                  Skip permissions <span className="font-mono text-xs text-muted-foreground">--dangerously-skip-permissions</span>
                </div>
                <div className="text-xs text-muted-foreground">
                  모든 도구 사용 권한 확인을 건너뜀(bypass 모드). 신뢰하는 작업만 켜기.
                </div>
              </div>
            </label>

            <label className="flex items-start gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={flagDebug}
                onChange={(e) => setFlagDebug(e.target.checked)}
                className="mt-0.5"
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
                className="mt-0.5"
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
                Additional flags (자유 입력 — <code>--mcp-config</code>, <code>--allowedTools</code> 등 특수 케이스용)
              </span>
              <Input
                value={extraFlags}
                onChange={(e) => setExtraFlags(e.target.value)}
                placeholder="--mcp-config /path/to/mcp.json"
              />
            </div>

            <div className="rounded bg-muted/30 px-2 py-1.5 font-mono text-[11px] text-muted-foreground">
              <span className="text-foreground/70">미리보기:</span>{" "}
              claude {composeFlags() || <span className="italic">(플래그 없음)</span>} --resume &lt;id&gt;
            </div>
          </div>

          <div className="space-y-2">
            <Label>Google Drive 동기화</Label>
            <div className="flex gap-2">
              <Input
                value={cloudPath}
                placeholder="아직 연결되지 않음"
                readOnly
              />
              <Button variant="default" onClick={autoConnectGoogleDrive}>
                자동 연결
              </Button>
              <Button variant="outline" onClick={pickFolder}>
                직접 선택
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              Google Drive 데스크탑 클라이언트가 설치돼 있으면 [자동 연결] 한 번으로 끝.
              세션은 <code>{`<드라이브>/Claude Sessions/`}</code>에 저장됩니다.
              업로드 후 로컬 jsonl은 자동 삭제(단일 본체 원칙).
            </p>
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
                  {report.claudeCliFound ? (
                    <div className="space-y-0.5">
                      <div className="text-green-500">
                        ✓ {tx(t, "settings.claudeCliFound", "Claude CLI: {path}").replace(
                          "{path}",
                          report.claudeCliPath || "",
                        )}
                      </div>
                      {report.claudeCliVersion && (
                        <div className="text-muted-foreground">
                          {tx(t, "settings.claudeCliVersion", "version {version}").replace(
                            "{version}",
                            report.claudeCliVersion,
                          )}
                        </div>
                      )}
                    </div>
                  ) : (
                    <div className="text-red-500">
                      ✗ {tx(t, "settings.claudeCliMissing", "Claude CLI not found on PATH")}
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
            <div className="flex items-center justify-between">
              <Label>문제 신고 / 디버그 로그</Label>
              <Button variant="outline" size="sm" onClick={loadDebugLog}>
                로그 불러오기
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              세션 실행/자동 요약 시 무엇이 spawn되는지 자세한 로그를 남깁니다. 문제가 생기면 아래 로그와 함께{" "}
              <a
                href="https://github.com/glowElephant/claude-session-manager/issues/new"
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

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={onClose}>
            {tx(t, "settings.cancel", "Cancel")}
          </Button>
          <Button onClick={save}>{tx(t, "settings.save", "Save")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
