import { memo, useCallback, useEffect, useRef, useState } from "react";
import {
  Cloud,
  CloudUpload,
  CloudDownload,
  RefreshCw,
  HardDrive,
  MoreHorizontal,
  Star,
} from "lucide-react";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn, formatBytes, formatRelativeTime } from "@/lib/utils";
import type { Session } from "@/types";
import type { Locale } from "@/i18n";

interface Props {
  sessions: Session[];
  selectedId: string | null;
  locale: Locale;
  t: (k: string, p?: Record<string, string | number>) => string;
  onSelect: (s: Session) => void;
  onResume: (s: Session) => void;
  onRename: (s: Session) => void;
  onDescribe: (s: Session) => void;
  onDelete: (s: Session) => void;
  onToggleCloud: (s: Session) => void;
  onGenerateSummary: (s: Session) => void;
  onToggleFavorite: (s: Session) => void;
}

type ColKey =
  | "star"
  | "name"
  | "id"
  | "desc"
  | "project"
  | "lastActive"
  | "size"
  | "type"
  | "actions";

const DEFAULTS: Record<ColKey, number> = {
  star: 36,
  name: 180,
  id: 100,
  desc: 360,
  project: 220,
  lastActive: 120,
  size: 90,
  type: 70,
  actions: 44,
};

const MIN_WIDTH: Record<ColKey, number> = {
  star: 36,
  name: 80,
  id: 60,
  desc: 120,
  project: 100,
  lastActive: 80,
  size: 70,
  type: 60,
  actions: 44,
};

const STORAGE_KEY = "csm.colWidths.v1";

function loadWidths(): Record<ColKey, number> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return { ...DEFAULTS, ...parsed };
    }
  } catch {}
  return { ...DEFAULTS };
}

function ResizableHead({
  colKey,
  width,
  onResize,
  className,
  children,
}: {
  colKey: ColKey;
  width: number;
  onResize: (key: ColKey, w: number) => void;
  className?: string;
  children?: React.ReactNode;
}) {
  const startX = useRef(0);
  const startW = useRef(0);
  const dragging = useRef(false);

  const onPointerDown = useCallback(
    (e: React.PointerEvent) => {
      e.preventDefault();
      e.stopPropagation();
      dragging.current = true;
      startX.current = e.clientX;
      startW.current = width;
      (e.target as Element).setPointerCapture(e.pointerId);

      const move = (ev: PointerEvent) => {
        if (!dragging.current) return;
        const delta = ev.clientX - startX.current;
        const next = Math.max(MIN_WIDTH[colKey], startW.current + delta);
        onResize(colKey, next);
      };
      const up = () => {
        dragging.current = false;
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", up);
      };
      window.addEventListener("pointermove", move);
      window.addEventListener("pointerup", up);
    },
    [colKey, width, onResize]
  );

  return (
    <TableHead
      style={{ width, minWidth: width, maxWidth: width }}
      className={cn("relative select-none", className)}
    >
      <div className="truncate pr-2">{children}</div>
      <div
        onPointerDown={onPointerDown}
        className="absolute right-0 top-0 h-full w-1.5 cursor-col-resize hover:bg-primary/40 active:bg-primary/60"
        aria-label={`Resize ${colKey}`}
      />
    </TableHead>
  );
}

function SessionTableInner({
  sessions,
  selectedId,
  locale,
  t,
  onSelect,
  onResume,
  onRename,
  onDescribe,
  onDelete,
  onToggleCloud,
  onGenerateSummary,
  onToggleFavorite,
}: Props) {
  const [widths, setWidths] = useState<Record<ColKey, number>>(() => loadWidths());

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(widths));
    } catch {}
  }, [widths]);

  const handleResize = useCallback((key: ColKey, w: number) => {
    setWidths((prev) => ({ ...prev, [key]: w }));
  }, []);

  if (sessions.length === 0) {
    return (
      <div className="flex h-60 items-center justify-center text-sm text-muted-foreground">
        {t("list.noSessions")}
      </div>
    );
  }

  const cellStyle = (key: ColKey): React.CSSProperties => ({
    width: widths[key],
    minWidth: widths[key],
    maxWidth: widths[key],
  });

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <ResizableHead colKey="star" width={widths.star} onResize={handleResize} />
          <ResizableHead colKey="name" width={widths.name} onResize={handleResize}>
            {t("list.name")}
          </ResizableHead>
          <ResizableHead colKey="id" width={widths.id} onResize={handleResize}>
            {t("list.id")}
          </ResizableHead>
          <ResizableHead colKey="desc" width={widths.desc} onResize={handleResize}>
            {t("list.description")}
          </ResizableHead>
          <ResizableHead colKey="project" width={widths.project} onResize={handleResize}>
            {t("list.project")}
          </ResizableHead>
          <ResizableHead colKey="lastActive" width={widths.lastActive} onResize={handleResize}>
            {t("list.lastActive")}
          </ResizableHead>
          <ResizableHead
            colKey="size"
            width={widths.size}
            onResize={handleResize}
            className="text-right"
          >
            {t("list.size")}
          </ResizableHead>
          <ResizableHead colKey="type" width={widths.type} onResize={handleResize}>
            {t("list.type")}
          </ResizableHead>
          <TableHead
            style={{ width: widths.actions, minWidth: widths.actions, maxWidth: widths.actions }}
          />
        </TableRow>
      </TableHeader>
      <TableBody>
        {sessions.map((s) => {
          const selected = selectedId === s.sessionId;
          const desc = s.description || s.autoSummary || s.firstUserMessage || "";
          return (
            <TableRow
              key={s.sessionId}
              data-state={selected ? "selected" : undefined}
              onClick={() => onSelect(s)}
              onDoubleClick={() => onResume(s)}
              className="cursor-pointer"
            >
              <TableCell style={cellStyle("star")} onClick={(e) => e.stopPropagation()} className="pr-0">
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7"
                  onClick={() => onToggleFavorite(s)}
                  aria-label={s.favorite ? "Unstar" : "Star"}
                >
                  <Star
                    className={cn(
                      "h-4 w-4",
                      s.favorite
                        ? "fill-amber-400 text-amber-400"
                        : "text-muted-foreground/40"
                    )}
                  />
                </Button>
              </TableCell>
              <TableCell style={cellStyle("name")} title={s.name || undefined}>
                {s.name ? (
                  <span className="block truncate font-medium">{s.name}</span>
                ) : (
                  <span className="text-sm italic text-muted-foreground/60">
                    {t("list.noName")}
                  </span>
                )}
              </TableCell>
              <TableCell style={cellStyle("id")} title={s.sessionId}>
                <span className="block truncate font-mono text-xs text-muted-foreground/80">
                  {s.sessionId.slice(0, 8)}
                </span>
              </TableCell>
              <TableCell style={cellStyle("desc")} title={desc || undefined}>
                <span className="block truncate text-sm text-foreground/80">{desc || "—"}</span>
              </TableCell>
              <TableCell style={cellStyle("project")} title={s.project}>
                <span className="block truncate font-mono text-xs text-muted-foreground">
                  {s.project}
                </span>
              </TableCell>
              <TableCell
                style={cellStyle("lastActive")}
                className="text-sm text-muted-foreground"
                title={s.lastTimestamp || undefined}
              >
                <span className="block truncate">
                  {s.lastTimestamp ? formatRelativeTime(s.lastTimestamp, locale) : "—"}
                </span>
              </TableCell>
              <TableCell
                style={cellStyle("size")}
                className="text-right tabular-nums text-sm text-muted-foreground"
                title={`${s.size} bytes`}
              >
                {formatBytes(s.size)}
              </TableCell>
              <TableCell style={cellStyle("type")}>
                <div className="inline-flex items-center gap-1.5">
                  {(() => {
                    const st = s.storageType;
                    const isSynced = st === "synced";
                    const isCloudOnly = st === "cloud-only" || st === "cloud";
                    const isLocalOnly = st === "local-only" || st === "local";
                    const label = isSynced
                      ? t("list.synced") !== "list.synced"
                        ? t("list.synced")
                        : "synced"
                      : isCloudOnly
                      ? t("list.cloudOnly") !== "list.cloudOnly"
                        ? t("list.cloudOnly")
                        : "cloud"
                      : t("list.localOnly") !== "list.localOnly"
                      ? t("list.localOnly")
                      : "local";
                    const Icon = isSynced
                      ? Cloud
                      : isCloudOnly
                      ? Cloud
                      : HardDrive;
                    const colorClass = isSynced
                      ? "text-sky-400"
                      : isCloudOnly
                      ? "text-sky-400"
                      : "text-emerald-400";
                    const SyncIcon = isLocalOnly
                      ? CloudUpload
                      : isCloudOnly
                      ? CloudDownload
                      : RefreshCw;
                    const syncTooltip = isLocalOnly
                      ? t("action.syncToCloud") !== "action.syncToCloud"
                        ? t("action.syncToCloud")
                        : "클라우드에 업로드"
                      : isCloudOnly
                      ? t("action.syncFromCloud") !== "action.syncFromCloud"
                        ? t("action.syncFromCloud")
                        : "로컬로 다운로드"
                      : t("action.resync") !== "action.resync"
                      ? t("action.resync")
                      : "로컬→클라우드 동기화";
                    return (
                      <>
                        <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                          <Icon className={`h-3 w-3 ${colorClass}`} />
                          {label}
                        </span>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            onToggleCloud(s);
                          }}
                          title={syncTooltip}
                          aria-label={syncTooltip}
                          className="ml-0.5 inline-flex h-5 w-5 items-center justify-center rounded text-muted-foreground/70 hover:bg-muted/40 hover:text-foreground transition-colors"
                        >
                          <SyncIcon className="h-3 w-3" />
                        </button>
                      </>
                    );
                  })()}
                </div>
              </TableCell>
              <TableCell
                style={cellStyle("actions")}
                onClick={(e) => e.stopPropagation()}
              >
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" size="icon" className="h-8 w-8">
                      <MoreHorizontal className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem onSelect={() => onResume(s)}>
                      {t("action.resumeNew")}
                    </DropdownMenuItem>
                    <DropdownMenuItem onSelect={() => onRename(s)}>
                      {t("action.rename")}
                    </DropdownMenuItem>
                    <DropdownMenuItem onSelect={() => onDescribe(s)}>
                      {t("action.describe")}
                    </DropdownMenuItem>
                    <DropdownMenuItem onSelect={() => onGenerateSummary(s)}>
                      {t("action.generateSummary")}
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem onSelect={() => onToggleCloud(s)}>
                      {s.storageType === "cloud-only" || s.storageType === "cloud"
                        ? t("action.syncFromCloud")
                        : s.storageType === "synced"
                        ? t("action.resync") !== "action.resync"
                          ? t("action.resync")
                          : t("action.syncToCloud")
                        : t("action.syncToCloud")}
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem destructive onSelect={() => onDelete(s)}>
                      {t("action.delete")}
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </TableCell>
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}

export const SessionTable = memo(SessionTableInner);
