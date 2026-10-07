// 세션 테이블의 정렬과 컬럼 폭 상태를 저장하고 계산한다.
import type { Session } from "@/types";
import { sessionDescriptionText } from "@/lib/sessionDisplay";

export type ColKey =
  | "select"
  | "star"
  | "name"
  | "lastActive"
  | "desc"
  | "size"
  | "project"
  | "id"
  | "type"
  | "actions";

export type SortKey = "name" | "id" | "desc" | "project" | "lastActive" | "size" | "type";
export type SortDir = "asc" | "desc";
export type SortState = { key: SortKey; dir: SortDir };

export const DEFAULT_COLUMN_WIDTHS: Record<ColKey, number> = {
  select: 48,
  star: 48,
  name: 180,
  lastActive: 120,
  desc: 360,
  size: 90,
  project: 220,
  id: 100,
  type: 70,
  actions: 48,
};

export const COLUMN_ORDER: ColKey[] = [
  "select",
  "star",
  "name",
  "lastActive",
  "desc",
  "size",
  "project",
  "id",
  "type",
  "actions",
];

export const MIN_COLUMN_WIDTH: Record<ColKey, number> = {
  select: 48,
  star: 48,
  name: 80,
  lastActive: 80,
  desc: 120,
  size: 70,
  project: 100,
  id: 60,
  type: 60,
  actions: 48,
};

const STORAGE_KEY = "csm.colWidths.v1";
const SORT_KEY = "csm.sort.v1";
const DEFAULT_SORT: SortState = { key: "lastActive", dir: "desc" };

export function loadSortPreference(): SortState {
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

export function saveSortPreference(sort: SortState): void {
  try {
    localStorage.setItem(SORT_KEY, JSON.stringify(sort));
  } catch {}
}

export function normalizeColumnWidths(source: Record<ColKey, number>): Record<ColKey, number> {
  return COLUMN_ORDER.reduce((acc, key) => {
    const raw = Number.isFinite(source[key]) ? source[key] : DEFAULT_COLUMN_WIDTHS[key];
    acc[key] = Math.max(MIN_COLUMN_WIDTH[key], raw);
    return acc;
  }, {} as Record<ColKey, number>);
}

export function loadColumnWidths(): Record<ColKey, number> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return normalizeColumnWidths({ ...DEFAULT_COLUMN_WIDTHS, ...parsed });
    }
  } catch {}
  return normalizeColumnWidths(DEFAULT_COLUMN_WIDTHS);
}

export function saveColumnWidths(widths: Record<ColKey, number>): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(widths));
  } catch {}
}

export function totalColumnWidth(source: Record<ColKey, number>): number {
  return COLUMN_ORDER.reduce((sum, key) => sum + source[key], 0);
}

export function compareSessions(a: Session, b: Session, sort: SortState): number {
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

export function shortSessionId(id: string): string {
  if (id.length <= 13) return id;
  return `${id.slice(0, 8)}...${id.slice(-4)}`;
}

export interface SessionTreeRow {
  session: Session;
  depth: number;
  childCount: number;
}

export function buildSessionTreeRows(
  sortedSessions: Session[],
  expandedIds: ReadonlySet<string>,
): SessionTreeRow[] {
  const byId = new Map(sortedSessions.map((session) => [session.sessionId, session]));
  const children = new Map<string, Session[]>();
  const roots: Session[] = [];
  for (const session of sortedSessions) {
    const parentId = session.isSubagent ? session.parentId : null;
    if (parentId && parentId !== session.sessionId && byId.has(parentId)) {
      const siblings = children.get(parentId) ?? [];
      siblings.push(session);
      children.set(parentId, siblings);
    } else {
      roots.push(session);
    }
  }

  const rows: SessionTreeRow[] = [];
  const visited = new Set<string>();
  // 접힌 자손도 방문하여 순환 관계의 대체 루트 처리에서 다시 노출하지 않는다.
  for (const root of [...roots, ...sortedSessions]) {
    if (visited.has(root.sessionId)) continue;
    const stack = [{ session: root, depth: 0, visible: true }];
    while (stack.length) {
      const { session, depth, visible } = stack.pop()!;
      if (visited.has(session.sessionId)) continue;
      visited.add(session.sessionId);
      const descendants = (children.get(session.sessionId) ?? [])
        .filter((child) => !visited.has(child.sessionId));
      if (visible) rows.push({ session, depth, childCount: descendants.length });
      for (let i = descendants.length - 1; i >= 0; i--) {
        stack.push({
          session: descendants[i],
          depth: depth + 1,
          visible: visible && expandedIds.has(session.sessionId),
        });
      }
    }
  }
  return rows;
}

export function filterSessionsWithAncestors(sessions: Session[], query: string): Session[] {
  const q = query.trim().toLowerCase();
  if (!q) return sessions;
  const byId = new Map(sessions.map((session) => [session.sessionId, session]));
  const included = new Set<string>();
  for (const session of sessions) {
    const text = [session.name, session.description, session.autoSummary, session.project,
      session.sessionId, session.firstUserMessage, session.agentNickname]
      .filter(Boolean).join(" ").toLowerCase();
    if (!text.includes(q)) continue;
    let ancestor: Session | undefined = session;
    while (ancestor && !included.has(ancestor.sessionId)) {
      included.add(ancestor.sessionId);
      ancestor = ancestor.isSubagent && ancestor.parentId ? byId.get(ancestor.parentId) : undefined;
    }
  }
  return sessions.filter((session) => included.has(session.sessionId));
}
