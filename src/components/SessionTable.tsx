// 등록 상태와 트리 관계를 유지하며 세션 목록과 작업을 표시한다.
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
  Archive,
  Cloud,
  CloudUpload,
  CloudDownload,
  ChevronDown,
  ChevronRight,
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
import { sessionDescriptionText, sessionKindLabels, sessionName } from "@/lib/sessionDisplay";
import {
  buildSessionTreeRows,
  loadColumnWidths,
  loadSortPreference,
  MIN_COLUMN_WIDTH,
  saveColumnWidths,
  saveSortPreference,
  shortSessionId,
  totalColumnWidth,
  type ColKey,
  type SortKey,
  type SortState,
} from "@/lib/sessionTableState";
import { cn, formatBytes, formatRelativeTime } from "@/lib/utils";
import type { Session } from "@/types";
import type { Locale } from "@/i18n";

interface Props {
  sessions: Session[];
  query?: string;
  canDelete: boolean;
  onVisibleRowsChange: (sessions: Session[]) => void;
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
  selectedSessionIds: Set<string>;
  onToggleSelected: (s: Session) => void;
  onToggleVisibleSelection: (sessions: Session[], selected: boolean) => void;
}

function ResizableHead({
  colKey,
  width,
  onResize,
  className,
  children,
  contentClassName,
  childrenClassName,
  sortKey,
  title,
  sortState,
  onSort,
}: {
  colKey: ColKey;
  width: number;
  onResize: (key: ColKey, w: number) => void;
  className?: string;
  children?: React.ReactNode;
  contentClassName?: string;
  childrenClassName?: string;
  /** 이 컬럼이 정렬 가능하면 SortKey, 아니면 undefined */
  sortKey?: SortKey;
  title?: string;
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
        const next = Math.max(MIN_COLUMN_WIDTH[colKey], startW.current + delta);
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
          isSortable && "cursor-pointer hover:text-foreground",
          contentClassName
        )}
        onClick={isSortable ? () => onSort!(sortKey!) : undefined}
        role={isSortable ? "button" : undefined}
        title={isSortable ? title : undefined}
      >
        <span className={cn("truncate", childrenClassName)}>{children}</span>
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
  query = "",
  canDelete,
  onVisibleRowsChange,
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
  selectedSessionIds,
  onToggleSelected,
  onToggleVisibleSelection,
}: Props) {
  const [widths, setWidths] = useState<Record<ColKey, number>>(() => loadColumnWidths());
  const [sort, setSort] = useState<SortState>(() => loadSortPreference());
  // 검색 접기는 일반 목록 접기와 분리하여 검색을 지우면 원래 상태로 돌아간다.
  const [expandedBranches, setExpandedBranches] = useState<Set<string>>(() => new Set());
  const [searchCollapse, setSearchCollapse] = useState<{ query: string; ids: Set<string> }>(() => ({ query: "", ids: new Set() }));
  const tableRef = useRef<HTMLDivElement | null>(null);
  const searching = !!query.trim();
  const searchCollapsedIds = searchCollapse.query === query ? searchCollapse.ids : undefined;
  useEffect(() => { setSearchCollapse({ query, ids: new Set() }); }, [query]);

  useEffect(() => {
    saveColumnWidths(widths);
  }, [widths]);

  useEffect(() => {
    saveSortPreference(sort);
  }, [sort]);

  const handleResize = useCallback((key: ColKey, w: number) => {
    setWidths((prev) => ({ ...prev, [key]: Math.max(MIN_COLUMN_WIDTH[key], w) }));
  }, []);

  const handleSort = useCallback((key: SortKey) => {
    setSort((prev) =>
      prev.key === key
        ? { key, dir: prev.dir === "asc" ? "desc" : "asc" }
        : { key, dir: "asc" }
    );
  }, []);

  const expandedIds = useMemo(
    () => searching
      ? new Set(sessions.filter((s) => !searchCollapsedIds?.has(s.sessionId)).map((s) => s.sessionId))
      : expandedBranches,
    [sessions, searching, searchCollapsedIds, expandedBranches],
  );
  const treeRows = useMemo(() => buildSessionTreeRows(sessions, expandedIds, sort), [sessions, expandedIds, sort]);
  const allRows = useMemo(() => buildSessionTreeRows(sessions, new Set(sessions.map((s) => s.sessionId)), sort), [sessions, sort]);
  const displayedSessions = useMemo(() => treeRows.map((row) => row.session), [treeRows]);
  useEffect(() => { onVisibleRowsChange(displayedSessions); }, [displayedSessions, onVisibleRowsChange]);
  function setAllExpanded(expand: boolean) {
    const parentIds = () => new Set(allRows.filter((row) => row.childCount > 0).map((row) => row.session.sessionId));
    const selectedIndex = expand ? -1 : allRows.findIndex((row) => row.session.sessionId === selectedId);
    if (selectedIndex >= 0 && allRows[selectedIndex].depth > 0) {
      let rootIndex = selectedIndex;
      while (rootIndex > 0 && allRows[rootIndex].depth > 0) rootIndex--;
      const rootId = allRows[rootIndex].session.sessionId;
      const parentRow = Array.from(tableRef.current?.querySelectorAll<HTMLTableRowElement>("[data-session-id]") ?? [])
        .find((row) => row.dataset.sessionId === rootId);
      parentRow?.querySelector<HTMLButtonElement>("button[aria-expanded]")?.focus();
    }
    if (searching) setSearchCollapse({ query, ids: expand ? new Set() : parentIds() });
    else setExpandedBranches(expand ? parentIds() : new Set());
  }
  const visibleSessions = useMemo(() => treeRows.map((row) => row.session).filter((s) => s.registered), [treeRows]);
  const tableWidth = useMemo(() => totalColumnWidth(widths), [widths]);
  const visibleSelectedCount = visibleSessions.filter((s) => selectedSessionIds.has(s.sessionId)).length;
  const allVisibleSelected = visibleSessions.length > 0 && visibleSelectedCount === visibleSessions.length;
  const someVisibleSelected = visibleSelectedCount > 0 && !allVisibleSelected;
  const selectAllRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (selectAllRef.current) {
      selectAllRef.current.indeterminate = someVisibleSelected;
    }
  }, [someVisibleSelected]);

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
    <div ref={tableRef} className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 gap-2 border-b border-border/60 px-3 py-2">
        <Button variant="outline" size="sm" aria-label={t("action.expandAll")} onClick={() => setAllExpanded(true)}>{t("action.expandAll")}</Button>
        <Button variant="outline" size="sm" aria-label={t("action.collapseAll")} onClick={() => setAllExpanded(false)}>{t("action.collapseAll")}</Button>
      </div>
      <div className="min-h-0 flex-1">
    <Table className="table-fixed" style={{ minWidth: tableWidth }}>
      <TableHeader>
        <TableRow>
          <ResizableHead
            colKey="select"
            width={widths.select}
            onResize={handleResize}
            className="px-0"
            contentClassName="justify-center overflow-visible pr-0"
            childrenClassName="flex shrink-0 items-center justify-center overflow-visible"
          >
            <input
              ref={selectAllRef}
              type="checkbox"
              checked={allVisibleSelected}
              onChange={(e) => onToggleVisibleSelection(visibleSessions, e.currentTarget.checked)}
              disabled={visibleSessions.length === 0}
              aria-label={allVisibleSelected ? t("action.deselectVisible") : t("action.selectVisible")}
              title={allVisibleSelected ? t("action.deselectVisible") : t("action.selectVisible")}
              className="h-4 w-4 min-w-4 shrink-0 rounded border-border accent-primary"
            />
          </ResizableHead>
          <ResizableHead
            colKey="star"
            width={widths.star}
            onResize={handleResize}
            className="px-0"
            contentClassName="justify-center pr-0"
          />
          <ResizableHead
            colKey="name"
            width={widths.name}
            onResize={handleResize}
            sortKey="name"
            sortState={sort}
            onSort={handleSort}
          >
            {t("list.name")}
          </ResizableHead>
          <ResizableHead
            colKey="lastActive"
            width={widths.lastActive}
            onResize={handleResize}
            title={t("list.branchActivity")}
            sortKey="lastActive"
            sortState={sort}
            onSort={handleSort}
          >
            {t("list.lastActive")}
          </ResizableHead>
          <ResizableHead
            colKey="desc"
            width={widths.desc}
            onResize={handleResize}
            sortKey="desc"
            sortState={sort}
            onSort={handleSort}
          >
            {t("list.description")}
          </ResizableHead>
          <ResizableHead
            colKey="size"
            width={widths.size}
            onResize={handleResize}
            className="text-right"
            sortKey="size"
            sortState={sort}
            onSort={handleSort}
          >
            {t("list.size")}
          </ResizableHead>
          <ResizableHead
            colKey="project"
            width={widths.project}
            onResize={handleResize}
            sortKey="project"
            sortState={sort}
            onSort={handleSort}
          >
            {t("list.project")}
          </ResizableHead>
          <ResizableHead
            colKey="id"
            width={widths.id}
            onResize={handleResize}
            sortKey="id"
            sortState={sort}
            onSort={handleSort}
          >
            {t("list.id")}
          </ResizableHead>
          <ResizableHead
            colKey="type"
            width={widths.type}
            onResize={handleResize}
            sortKey="type"
            sortState={sort}
            onSort={handleSort}
          >
            {t("list.type")}
          </ResizableHead>
          <TableHead
            style={{
              width: widths.actions,
              minWidth: widths.actions,
              maxWidth: widths.actions,
            }}
            className="px-0"
          />
        </TableRow>
      </TableHeader>
      <TableBody>
        {treeRows.map(({ session: s, depth, childCount, parentMissing }) => {
          const selected = selectedId === s.sessionId;
          const checked = selectedSessionIds.has(s.sessionId);
          const desc = sessionDescriptionText(s);
          const expanded = expandedIds.has(s.sessionId);
          const displayName = sessionName(s, t);
          const cloudOnly = s.storageType === "cloud" || s.storageType === "cloud-only";
          const resumeDisabled = cloudOnly ? s.historyMode !== "legacy" : !s.registered;
          const cloudDisabled = s.historyMode !== "legacy" || !s.filePath;
          const cloudReason = cloudDisabled ? t("action.legacyRequired") : undefined;
          const apiReason = !s.registered ? t("action.registeredRequired") : undefined;
          // ponytail: 들여쓰기는 6단계까지 표시하며 더 깊은 트리는 이름 열 폭과 상한을 함께 늘린다.
          return (
            <TableRow
              key={s.sessionId}
              data-depth={depth}
              data-session-id={s.sessionId}
              data-state={selected ? "selected" : undefined}
              onClick={() => onSelect(s)}
              onDoubleClick={() => !resumeDisabled && onResume(s)}
              className="cursor-pointer"
            >
              <TableCell
                style={cellStyle("select")}
                onClick={(e) => e.stopPropagation()}
                className="px-0 text-center"
              >
                <input
                  type="checkbox"
                  checked={checked}
                  disabled={!s.registered}
                  onChange={() => onToggleSelected(s)}
                  aria-label={checked ? t("action.deselectSession") : t("action.selectSession")}
                  title={checked ? t("action.deselectSession") : t("action.selectSession")}
                  className="mx-auto block h-4 w-4 min-w-4 shrink-0 rounded border-border accent-primary"
                />
              </TableCell>
              <TableCell style={cellStyle("star")} onClick={(e) => e.stopPropagation()} className="px-0 text-center">
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
              <TableCell style={cellStyle("name")} title={displayName}>
                <div className="flex min-w-0 items-center gap-1" style={{ paddingLeft: Math.min(depth, 6) * 16 }}>
                  {childCount > 0 ? (
                    <button
                      type="button"
                      aria-expanded={expanded}
                      aria-label={t(expanded ? "action.collapseChildren" : "action.expandChildren", { name: displayName, count: childCount })}
                      title={t(expanded ? "action.collapseChildren" : "action.expandChildren", { name: displayName, count: childCount })}
                      onDoubleClick={(e) => e.stopPropagation()}
                      onClick={(e) => {
                        e.stopPropagation();
                        if (expanded) {
                          const index = allRows.findIndex((row) => row.session.sessionId === s.sessionId);
                          let hidesSelection = false;
                          for (let i = index + 1; i < allRows.length && allRows[i].depth > depth; i++) {
                            if (allRows[i].session.sessionId === selectedId) hidesSelection = true;
                          }
                          if (hidesSelection) e.currentTarget.focus();
                        }
                        const next = new Set(searching ? searchCollapsedIds : expandedBranches);
                        if (next.has(s.sessionId)) next.delete(s.sessionId);
                        else next.add(s.sessionId);
                        if (searching) setSearchCollapse({ query, ids: next });
                        else setExpandedBranches(next);
                      }}
                      className="inline-flex h-6 w-5 shrink-0 items-center justify-center rounded text-muted-foreground hover:bg-muted/40 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    >
                      {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
                    </button>
                  ) : <span className="w-5 shrink-0" />}
                  <div className="min-w-0 flex-1">
                    <span className={cn("block truncate", s.name || s.agentNickname ? "font-medium" : "text-sm italic text-muted-foreground/60")}>
                      {displayName}
                    </span>
                    {parentMissing && <span className="block truncate text-[10px] text-amber-500">{t("list.parentMissing")}</span>}
                    {s.agentNickname && s.name && s.agentNickname !== s.name && (
                      <span className="block truncate text-[10px] text-muted-foreground">{s.agentNickname}</span>
                    )}
                  </div>
                  {sessionKindLabels(s).map((kind) => <span key={kind} className={cn("shrink-0 rounded px-1 text-[10px]", kind === "guardian" ? "bg-amber-500/15 text-amber-400" : kind === "worker" ? "bg-sky-500/15 text-sky-400" : "bg-muted text-muted-foreground")}>{t("kind." + kind)}</span>)}
                  {childCount > 0 && <span className="shrink-0 text-[10px] tabular-nums text-muted-foreground">{childCount}</span>}
                </div>
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
              <TableCell style={cellStyle("desc")} title={desc || undefined}>
                <span className="block truncate text-sm text-foreground/80">{desc || "—"}</span>
                {s.preview && <span className="block truncate text-xs text-muted-foreground"><span>{t("detail.preview")} · </span><span>{s.preview}</span></span>}
              </TableCell>
              <TableCell
                style={cellStyle("size")}
                className="text-right tabular-nums text-sm text-muted-foreground"
                title={s.size === null ? undefined : `${s.size} bytes`}
              >
                {s.size === null ? "—" : formatBytes(s.size)}
              </TableCell>
              <TableCell style={cellStyle("project")} title={s.project}>
                <span className="block truncate font-mono text-xs text-muted-foreground">
                  {s.project}
                </span>
              </TableCell>
              <TableCell style={cellStyle("id")} title={s.sessionId}>
                <span className="block truncate font-mono text-xs text-muted-foreground/80">
                  {shortSessionId(s.sessionId)}
                </span>
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
                          disabled={cloudDisabled}
                          title={cloudReason ?? syncTooltip}
                          aria-label={cloudReason ?? syncTooltip}
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
                className="px-0 text-center"
              >
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" size="icon" className="h-8 w-8">
                      <MoreHorizontal className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem disabled={resumeDisabled} title={resumeDisabled ? (cloudOnly ? cloudReason : apiReason) : undefined} onSelect={() => onResume(s)}>
                      {t("action.resumeNew")}
                    </DropdownMenuItem>
                    <DropdownMenuItem onSelect={() => onRename(s)}>
                      {t("action.rename")}
                    </DropdownMenuItem>
                    <DropdownMenuItem onSelect={() => onDescribe(s)}>
                      {t("action.describe")}
                    </DropdownMenuItem>
                    <DropdownMenuItem disabled={!s.registered} title={apiReason} onSelect={() => onGenerateSummary(s)}>
                      {t("action.generateSummary")}
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem disabled={!s.registered} title={apiReason} onSelect={() => onToggleArchive(s)}>
                      {s.archived
                        ? t("action.unarchive") !== "action.unarchive"
                          ? t("action.unarchive")
                          : "Unarchive"
                        : t("action.archive") !== "action.archive"
                        ? t("action.archive")
                        : "Archive"}
                    </DropdownMenuItem>
                    <DropdownMenuItem disabled={cloudDisabled} title={cloudReason} onSelect={() => onToggleCloud(s)}>
                      {s.storageType === "cloud-only" || s.storageType === "cloud"
                        ? t("action.syncFromCloud")
                        : s.storageType === "synced"
                        ? t("action.resync") !== "action.resync"
                          ? t("action.resync")
                          : t("action.syncToCloud")
                        : t("action.syncToCloud")}
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem disabled={!s.registered || !canDelete} title={!canDelete ? t("delete.snapshotRequired") : apiReason} destructive onSelect={() => onDelete(s)}>
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
      </div>
    </div>
  );
}

export const SessionTable = memo(SessionTableInner);
