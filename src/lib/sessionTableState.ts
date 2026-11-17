// 세션 테이블의 정렬과 컬럼 폭 상태를 저장하고 계산한다.
import type { Session } from "@/types";
import { sessionDescriptionText, sessionKindLabels } from "@/lib/sessionDisplay";

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
  if (sort.key === "size" && (a.size === null) !== (b.size === null)) return a.size === null ? 1 : -1;
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
      if (a.size === null) return b.size === null ? 0 : 1;
      if (b.size === null) return -1;
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
  parentMissing: boolean;
}

export function buildSessionTreeRows(
  sessions: Session[],
  expandedIds: ReadonlySet<string>,
  sort?: SortState,
): SessionTreeRow[] {
  const byId = new Map(sessions.map((session) => [session.sessionId, session]));
  const parents = new Map<string, string>();
  for (const session of byId.values()) {
    if (session.parentId && session.parentId !== session.sessionId && byId.has(session.parentId)) {
      parents.set(session.sessionId, session.parentId);
    }
  }
  // 순환 간선만 표시 계산에서 끊고 원본 직접 부모 ID는 보존한다.
  const checked = new Set<string>();
  for (const id of byId.keys()) {
    const path = new Set<string>();
    let current: string | undefined = id;
    while (current && !checked.has(current)) {
      if (path.has(current)) { parents.delete(current); break; }
      path.add(current);
      current = parents.get(current);
    }
    for (const visited of path) checked.add(visited);
  }
  const children = new Map<string, Session[]>();
  const roots: Session[] = [];
  for (const session of byId.values()) {
    const parent = parents.get(session.sessionId);
    if (parent) {
      const siblings = children.get(parent) ?? [];
      siblings.push(session);
      children.set(parent, siblings);
    } else roots.push(session);
  }
  const activity = new Map<string, string>();
  if (sort?.key === "lastActive") {
    const traversal: Session[] = [];
    const stack = [...roots];
    while (stack.length) {
      const session = stack.pop()!;
      traversal.push(session);
      stack.push(...children.get(session.sessionId) ?? []);
    }
    for (let i = traversal.length - 1; i >= 0; i--) {
      const session = traversal[i];
      let latest = session.lastTimestamp ?? "";
      for (const child of children.get(session.sessionId) ?? []) {
        const stamp = activity.get(child.sessionId) ?? "";
        if (stamp > latest) latest = stamp;
      }
      activity.set(session.sessionId, latest);
    }
  }
  if (sort) {
    const compare = (a: Session, b: Session) => compareSessions(
      sort.key === "lastActive" ? { ...a, lastTimestamp: activity.get(a.sessionId) ?? null } : a,
      sort.key === "lastActive" ? { ...b, lastTimestamp: activity.get(b.sessionId) ?? null } : b,
      sort,
    );
    roots.sort(compare);
    for (const siblings of children.values()) siblings.sort(compare);
  }
  const rows: SessionTreeRow[] = [];
  const stack = roots.slice().reverse().map((session) => ({ session, depth: 0, visible: true }));
  while (stack.length) {
    const { session, depth, visible } = stack.pop()!;
    const descendants = children.get(session.sessionId) ?? [];
    if (visible) rows.push({ session, depth, childCount: descendants.length,
      parentMissing: !!session.parentId && !byId.has(session.parentId) });
    for (let i = descendants.length - 1; i >= 0; i--) {
      stack.push({ session: descendants[i], depth: depth + 1,
        visible: visible && expandedIds.has(session.sessionId) });
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
      session.sessionId, session.firstUserMessage, session.preview, session.agentNickname,
      ...sessionKindLabels(session),
      ...sessionKindLabels(session).map((kind) => ({ main: "Main 일반", worker: "Worker 작업 서브 작업자", guardian: "Guardian", otherInternal: "Other internal 기타 내부", unknown: "Unknown 알 수 없음", summary: "Summary run 자동요약 자동 요약" })[kind])]
      .filter(Boolean).join(" ").toLowerCase();
    if (!text.includes(q)) continue;
    let ancestor: Session | undefined = session;
    while (ancestor && !included.has(ancestor.sessionId)) {
      included.add(ancestor.sessionId);
      ancestor = ancestor.parentId ? byId.get(ancestor.parentId) : undefined;
    }
  }
  return sessions.filter((session) => included.has(session.sessionId));
}
