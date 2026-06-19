import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
  Archive,
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
import { sessionDescriptionText } from "@/lib/sessionDisplay";
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
  onToggleArchive: (s: Session) => void;
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
const SORT_KEY = "csm.sort.v1";

type SortKey = "name" | "id" | "desc" | "project" | "lastActive" | "size" | "type";
type SortDir = "asc" | "desc";
type SortState = { key: SortKey; dir: SortDir };

const DEFAULT_SORT: SortState = { key: "lastActive", dir: "desc" };

function loadSort(): SortState {
  try {
    const raw = localStorage.getItem(SORT_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (parsed && typeof parsed.key === "string" && (parsed.dir === "asc" || parsed.dir === "desc")) {
        return parsed as SortState;
      }
    }
  } catch {}
  return { ...DEFAULT_SORT };
}

function compareSessions(a: Session, b: Session, sort: SortState): number {
  // 즐겨찾기는 항상 최상단 (사용자 정렬 무시)
  if (a.favorite !== b.favorite) return a.favorite ? -1 : 1;
  const sign = sort.dir === "asc" ? 1 : -1;
  const cmp = (x: string | null | undefined, y: string | null | undefined) =>
    (x ?? "").localeCompare(y ?? "");
  switch (sort.key) {
    case "name":
      return sign * cmp(a.name, b.name);
    case "id":
      return sign * a.sessionId.localeCompare(b.sessionId);
    case "desc":
      return sign * cmp(sessionDescriptionText(a), sessionDescriptionText(b));
    case "project":
      return sign * cmp(a.project, b.project);
    case "lastActive":
      return sign * cmp(a.lastTimestamp, b.lastTimestamp);
    case "size":
      return sign * (a.size - b.size);
    case "type":
      return sign * a.storageType.localeCompare(b.storageType);
  }
}

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
  sortKey,
  sortState,
  onSort,
}: {
  colKey: ColKey;
  width: number;
  onResize: (key: ColKey, w: number) => void;
  className?: string;
  children?: React.ReactNode;
  /** 이 컬럼이 정렬 가능하면 SortKey, 아니면 undefined */
  sortKey?: SortKey;
  sortState?: SortState;
  onSort?: (key: SortKey) => void;
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

  const isSortable = !!sortKey && !!onSort;
  const isActiveSort = isSortable && sortState && sortKey === sortState.key;

  return (
    <TableHead
      style={{ width, minWidth: width, maxWidth: width }}
      className={cn("relative select-none", className)}
    >
      <div
        className={cn(
          "flex items-center gap-1 truncate pr-2",
          isSortable && "cursor-pointer hover:text-foreground"
        )}
        onClick={isSortable ? () => onSort!(sortKey!) : undefined}
        role={isSortable ? "button" : undefined}
        title={isSortable ? "정렬: 다시 클릭하면 역순" : undefined}
      >
        <span className="truncate">{children}</span>
        {isActiveSort && (
          sortState!.dir === "asc"
            ? <ArrowUp className="h-3 w-3 shrink-0 text-foreground/70" />
            : <ArrowDown className="h-3 w-3 shrink-0 text-foreground/70" />
        )}
      </div>
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
  onToggleArchive,
  onToggleCloud,
  onGenerateSummary,
  onToggleFavorite,
}: Props) {
  const [widths, setWidths] = useState<Record<ColKey, number>>(() => loadWidths());
  const [sort, setSort] = useState<SortState>(() => loadSort());

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(widths));
    } catch {}
  }, [widths]);

  useEffect(() => {
    try {
      localStorage.setItem(SORT_KEY, JSON.stringify(sort));
    } catch {}
  }, [sort]);

  const handleResize = useCallback((key: ColKey, w: number) => {
    setWidths((prev) => ({ ...prev, [key]: w }));
  }, []);

  const handleSort = useCallback((key: SortKey) => {
    setSort((prev) =>
      prev.key === key
        ? { key, dir: prev.dir === "asc" ? "desc" : "asc" }
        : { key, dir: "asc" }
    );
  }, []);

  const sortedSessions = useMemo(
    () => [...sessions].sort((a, b) => compareSessions(a, b, sort)),
    [sessions, sort]
  );

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
          <ResizableHead colKey="name" width={widths.name} onResize={handleResize}
            sortKey="name" sortState={sort} onSort={handleSort}>
            {t("list.name")}
          </ResizableHead>
          <ResizableHead colKey="id" width={widths.id} onResize={handleResize}
            sortKey="id" sortState={sort} onSort={handleSort}>
            {t("list.id")}
          </ResizableHead>
          <ResizableHead colKey="desc" width={widths.desc} onResize={handleResize}
            sortKey="desc" sortState={sort} onSort={handleSort}>
            {t("list.description")}
          </ResizableHead>
          <ResizableHead colKey="project" width={widths.project} onResize={handleResize}
            sortKey="project" sortState={sort} onSort={handleSort}>
            {t("list.project")}
          </ResizableHead>
          <ResizableHead colKey="lastActive" width={widths.lastActive} onResize={handleResize}
            sortKey="lastActive" sortState={sort} onSort={handleSort}>
            {t("list.lastActive")}
          </ResizableHead>
          <ResizableHead
            colKey="size"
            width={widths.size}
            onResize={handleResize}
            className="text-right"
            sortKey="size" sortState={sort} onSort={handleSort}
          >
            {t("list.size")}
          </ResizableHead>
          <ResizableHead colKey="type" width={widths.type} onResize={handleResize}
            sortKey="type" sortState={sort} onSort={handleSort}>
            {t("list.type")}
          </ResizableHead>
          <TableHead
            style={{ width: widths.actions, minWidth: widths.actions, maxWidth: widths.actions }}
          />
        </TableRow>
      </TableHeader>
      <TableBody>
        {sortedSessions.map((s) => {
          const selected = selectedId === s.sessionId;
          const desc = sessionDescriptionText(s);
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
                    const label = s.archived
                      ? t("list.archived") !== "list.archived"
                        ? t("list.archived")
                        : "archived"
                      : isSynced
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
                    const Icon = s.archived
                      ? Archive
                      : isSynced
                      ? Cloud
                      : isCloudOnly
                      ? Cloud
                      : HardDrive;
                    const colorClass = s.archived
                      ? "text-amber-400"
                      : isSynced
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
                    <DropdownMenuItem onSelect={() => onToggleArchive(s)}>
                      {s.archived
                        ? t("action.unarchive") !== "action.unarchive"
                          ? t("action.unarchive")
                          : "Unarchive"
                        : t("action.archive") !== "action.archive"
                        ? t("action.archive")
                        : "Archive"}
                    </DropdownMenuItem>
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
