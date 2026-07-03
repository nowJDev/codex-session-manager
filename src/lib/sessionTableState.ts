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
