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
  type SessionBranchActivities,
  type SortKey,
  type SortState,
} from "@/lib/sessionTableState";
import { cn, formatBytes, formatRelativeTime } from "@/lib/utils";
import type { Session } from "@/types";
import type { Locale } from "@/i18n";

interface Props {
  sessions: Session[];
  query?: string;
  branchActivities?: SessionBranchActivities;
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
  left,
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
  left?: number;
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

  const activeDirection = sortKey && sortKey === sortState?.key ? sortState.dir : undefined;
  const content = <span className={cn("truncate", childrenClassName)}>{children}</span>;

  return (
    <TableHead
      scope="col"
      style={{ width, minWidth: width, maxWidth: width, left }}
      aria-sort={activeDirection ? activeDirection === "asc" ? "ascending" : "descending" : undefined}
      className={cn("relative select-none", left !== undefined && "sticky z-10 bg-card", className)}
    >
      {sortKey && onSort ? (
        <button
          type="button"
          className={cn("flex w-full items-center gap-1 rounded pr-2 text-inherit hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring", contentClassName)}
          onClick={() => onSort(sortKey)}
          title={title}
        >
          {content}
          {activeDirection === "asc" && <ArrowUp aria-hidden="true" className="h-3 w-3 shrink-0" />}
          {activeDirection === "desc" && <ArrowDown aria-hidden="true" className="h-3 w-3 shrink-0" />}
        </button>
      ) : (
        <div className={cn("flex items-center gap-1 truncate pr-2", contentClassName)}>{content}</div>
      )}
      <div
        onPointerDown={onPointerDown}
        className="absolute right-0 top-0 h-full w-1.5 cursor-col-resize hover:bg-primary/40 active:bg-primary/60"
        aria-hidden="true"
      />
    </TableHead>
  );
}

function SessionTableInner({
  sessions,
  query = "",
  branchActivities,
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
  const treeRows = useMemo(
    () => buildSessionTreeRows(sessions, expandedIds, sort, branchActivities),
    [sessions, expandedIds, sort, branchActivities],
  );
  const allRows = useMemo(
    () => buildSessionTreeRows(sessions, new Set(sessions.map((s) => s.sessionId)), sort, branchActivities),
    [sessions, sort, branchActivities],
  );
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

  const leftOffsets: Partial<Record<ColKey, number>> = { select: 0, star: widths.select, name: widths.select + widths.star };
  const pinnedCell = "sticky z-10 bg-background group-hover:bg-accent group-data-[state=selected]:bg-accent";
  const cellStyle = (key: ColKey): React.CSSProperties => ({
    left: leftOffsets[key],
    width: widths[key],
    minWidth: widths[key],
    maxWidth: widths[key],
  });

  return (
    <div ref={tableRef} className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 flex-wrap gap-2 border-b border-border/60 px-3 py-2">
        <Button variant="outline" size="sm" aria-label={t("action.expandAll")} onClick={() => setAllExpanded(true)}>{t("action.expandAll")}</Button>
        <Button variant="outline" size="sm" aria-label={t("action.collapseAll")} onClick={() => setAllExpanded(false)}>{t("action.collapseAll")}</Button>
      </div>
      <div className="min-h-0 flex-1">
    <Table
      className="table-fixed border-separate border-spacing-0"
      style={{ width: tableWidth }}
      viewportLabel={t("list.sessionList")}
      viewportStyle={{
        scrollPaddingTop: "2.5rem",
        scrollPaddingLeft: widths.select + widths.star + widths.name,
        scrollPaddingRight: widths.actions,
      }}
    >
      <TableHeader className="sticky top-0 z-20 bg-card">
        <TableRow>
          <ResizableHead
            colKey="select"
            width={widths.select}
            left={leftOffsets.select}
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
            left={leftOffsets.star}
            onResize={handleResize}
            className="px-0"
            contentClassName="justify-center pr-0"
          />
          <ResizableHead
            colKey="name"
            width={widths.name}
            left={leftOffsets.name}
            onResize={handleResize}
            className="border-r border-border/60"
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
            contentClassName="justify-end"
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
            scope="col"
            style={cellStyle("actions")}
            className="sticky right-0 z-20 border-l border-border/60 bg-card px-0"
          />
        </TableRow>
      </TableHeader>
      <TableBody>
        {treeRows.map(({ session: s, depth, childCount, parentMissing, branchLastTimestamp }) => {
          const selected = selectedId === s.sessionId;
          const checked = selectedSessionIds.has(s.sessionId);
          const desc = sessionDescriptionText(s);
          const expanded = expandedIds.has(s.sessionId);
          const displayName = sessionName(s, t);
          const selectLabel = t(checked ? "action.deselectSession" : "action.selectSession", { name: displayName });
          const favoriteLabel = t(s.favorite ? "action.unfavoriteSession" : "action.favoriteSession", { name: displayName });
          const actionsLabel = t("action.sessionActions", { name: displayName });
          const hasBranchActivity = (branchActivities?.get(s.sessionId)?.childCount ?? childCount) > 0;
          const cloudOnly = s.storageType === "cloud" || s.storageType === "cloud-only";
          const resumeDisabled = cloudOnly ? s.historyMode !== "legacy" : !s.registered;
          const cloudDisabled = s.historyMode !== "legacy" || !s.filePath;
          const cloudReason = cloudDisabled ? t("action.legacyRequired") : undefined;
          const apiReason = !s.registered ? t("action.registeredRequired") : undefined;
          return (
            <TableRow
              key={s.sessionId}
              data-depth={depth}
              data-session-id={s.sessionId}
              data-state={selected ? "selected" : undefined}
              className="group [&>td]:border-b [&>td]:border-border/40"
            >
              <TableCell
                style={cellStyle("select")}
                className={cn(pinnedCell, "px-0 text-center")}
              >
                <input
                  type="checkbox"
                  checked={checked}
                  disabled={!s.registered}
                  onChange={() => onToggleSelected(s)}
                  aria-label={selectLabel}
                  title={selectLabel}
                  className="mx-auto block h-4 w-4 min-w-4 shrink-0 rounded border-border accent-primary"
                />
              </TableCell>
              <TableCell style={cellStyle("star")} className={cn(pinnedCell, "px-0 text-center")}>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7"
                  onClick={() => onToggleFavorite(s)}
                  aria-label={t("action.favoriteSession", { name: displayName })}
                  title={favoriteLabel}
                  aria-pressed={s.favorite}
                >
                  <Star
                    className={cn(
                      "h-4 w-4",
                      s.favorite
                        ? "fill-amber-400 text-amber-400"
                        : "text-muted-foreground"
                    )}
                  />
                </Button>
              </TableCell>
              <TableCell style={cellStyle("name")} className={cn(pinnedCell, "border-r border-border/60")} title={displayName}>
                <div className="flex min-w-0 items-start gap-1" style={{ paddingLeft: Math.min(depth * 8, 48, widths.name / 4) }}>
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
                    <button
                      type="button"
                      data-session-title=""
                      aria-current={selected || undefined}
                      onClick={() => onSelect(s)}
                      onDoubleClick={() => { if (!resumeDisabled) onResume(s); }}
                      className={cn("block w-full truncate rounded text-left hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                        s.name || s.agentNickname ? "font-medium" : "italic text-muted-foreground")}
                    >
                      {displayName}
                    </button>
                    {parentMissing && <span className="block break-words text-xs text-amber-400">{t("list.parentMissing")}</span>}
                    {s.agentNickname && s.name && s.agentNickname !== s.name && (
                      <span className="block truncate text-xs text-muted-foreground">{s.agentNickname}</span>
                    )}
                    <div className="mt-0.5 flex flex-wrap items-center gap-1">
                      {sessionKindLabels(s).map((kind) => <span key={kind} className={cn("max-w-full rounded px-1 text-xs [overflow-wrap:anywhere]", kind === "guardian" ? "bg-amber-500/15 text-amber-400" : kind === "worker" ? "bg-sky-500/15 text-sky-400" : "bg-muted text-muted-foreground")}>{t("kind." + kind)}</span>)}
                      {childCount > 0 && <span className="shrink-0 text-xs tabular-nums text-muted-foreground" aria-hidden="true">{childCount}</span>}
                    </div>
                  </div>
                </div>
              </TableCell>
              <TableCell
                style={cellStyle("lastActive")}
                className="text-sm text-muted-foreground"
                title={branchLastTimestamp || undefined}
              >
                <span className="block truncate">
                  {branchLastTimestamp ? formatRelativeTime(branchLastTimestamp, locale) : "—"}
                </span>
                {hasBranchActivity && <span className="block text-xs">{t("list.branchActivityValue")}</span>}
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
                <div className="flex min-w-0 items-center gap-1">
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
                        <span className="flex min-w-0 items-center gap-1 text-xs text-muted-foreground" title={label}>
                          <Icon aria-hidden="true" className={`h-3 w-3 shrink-0 ${colorClass}`} />
                          <span className="truncate">{label}</span>
                        </span>
                        <button
                          type="button"
                          onClick={() => onToggleCloud(s)}
                          disabled={cloudDisabled}
                          title={cloudReason ?? syncTooltip}
                          aria-label={cloudReason ?? syncTooltip}
                          className="inline-flex h-6 w-6 shrink-0 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-muted/40 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
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
                className={cn(pinnedCell, "right-0 z-20 border-l border-border/60 px-0 text-center")}
              >
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" size="icon" className="h-8 w-8" aria-label={actionsLabel} title={actionsLabel}>
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
