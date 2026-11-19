// 세션 삭제 전 확인과 삭제 진행 상태를 표시하는 다이얼로그입니다.
import { useEffect, useRef, useState } from "react";
import { AlertTriangle, Loader2, Trash2 } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import type { DeleteProgress } from "@/types";

const PREVIEW_PAGE_SIZE = 25;

interface Props {
  open: boolean;
  count: number;
  pending: boolean;
  canDelete: boolean;
  title: string;
  description: string;
  confirmLabel: string;
  cancelLabel: string;
  previewRows?: readonly { id: string; title: string; project: string; reason: string }[];
  hiddenSelectedCount?: number;
  preparing?: boolean;
  t: (key: string, params?: Record<string, string | number>) => string;
  onConfirm: (reportProgress: (update: DeleteProgress) => void) => void;
  onCancel: () => void;
  onUnselectHidden?: () => void;
  returnFocus?: () => void;
}

function formatDuration(totalSeconds: number, t: Props["t"]): string {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor(totalSeconds / 60) % 60;
  const seconds = totalSeconds % 60;
  if (hours > 0) return t("delete.durationHours", { hours, minutes, seconds });
  if (minutes > 0) return t("delete.durationMinutes", { minutes, seconds });
  return t("delete.durationSeconds", { seconds });
}

export function DeleteConfirmDialog({
  open,
  count,
  pending,
  canDelete,
  title,
  description,
  confirmLabel,
  cancelLabel,
  previewRows = [],
  hiddenSelectedCount = 0,
  preparing = false,
  t,
  onConfirm,
  onCancel,
  onUnselectHidden,
  returnFocus,
}: Props) {
  const [progress, setProgress] = useState<DeleteProgress | null>(null);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [page, setPage] = useState(0);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);
  const locked = pending || preparing;
  const showProgress = open && locked;
  const pageCount = Math.max(1, Math.ceil(previewRows.length / PREVIEW_PAGE_SIZE));
  const currentPage = Math.min(page, pageCount - 1);
  const firstRow = currentPage * PREVIEW_PAGE_SIZE;

  useEffect(() => {
    if (!open) setPage(0);
  }, [open]);
  useEffect(() => {
    if (!showProgress) return;
    const startedAt = performance.now();
    setElapsedSeconds(0);
    const timer = window.setInterval(() => {
      setElapsedSeconds(Math.floor((performance.now() - startedAt) / 1000));
    }, 1000);
    return () => window.clearInterval(timer);
  }, [showProgress]);

  let progressLabel = t("delete.preparing");
  let remainingLabel: string | null = null;
  if (pending && progress?.phase === "deleting") {
    const { processed, total, elapsedMs } = progress;
    progressLabel = total > 0 ? t("delete.requestProgress", { processed, total }) : t("delete.pending");
    if (processed >= 2 && processed < total && elapsedMs > 0) {
      const seconds = Math.ceil((elapsedMs / processed) * (total - processed) / 1000);
      remainingLabel = t("delete.remaining", { duration: formatDuration(seconds, t) });
    }
  } else if (pending && progress?.phase === "verifying") {
    progressLabel = t("delete.verifying");
  }

  return (
    <Dialog open={open} onOpenChange={(v) => !v && !locked && onCancel()}>
      <DialogContent
        className="max-h-[90vh] max-w-2xl overflow-y-auto"
        closeLabel={t("common.close")}
        showClose={!locked}
        onOpenAutoFocus={(event) => {
          if (typeof document !== "undefined" && document.activeElement instanceof HTMLElement) {
            previousFocus.current = document.activeElement;
          }
          if (cancelRef.current && !cancelRef.current.disabled) {
            event.preventDefault();
            cancelRef.current.focus();
          }
        }}
        onCloseAutoFocus={(event) => {
          if (returnFocus) {
            event.preventDefault();
            returnFocus();
          } else if (previousFocus.current?.isConnected) {
            event.preventDefault();
            previousFocus.current.focus();
          }
        }}
        onEscapeKeyDown={(event) => { if (locked) event.preventDefault(); }}
        onInteractOutside={(event) => { if (locked) event.preventDefault(); }}
      >
        <DialogHeader>
          <div className="flex items-start gap-3 pr-6">
            <div className="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-destructive/15 text-destructive">
              {locked ? <Loader2 className="h-5 w-5 animate-spin" /> : <AlertTriangle className="h-5 w-5" />}
            </div>
            <div className="space-y-2">
              <DialogTitle>{title}</DialogTitle>
              <DialogDescription>{description}</DialogDescription>
            </div>
          </div>
        </DialogHeader>

        {hiddenSelectedCount > 0 && (
          <div className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border bg-muted/35 px-3 py-2 text-sm">
            <span>{t("delete.hiddenSelected", { count: hiddenSelectedCount })}</span>
            {onUnselectHidden && (
              <Button variant="outline" size="sm" disabled={locked} onClick={() => {
                setPage(0);
                onUnselectHidden();
              }}>
                {t("delete.unselectHidden")}
              </Button>
            )}
          </div>
        )}

        {previewRows.length > 0 && (
          <section className="space-y-2" aria-labelledby="delete-preview-title">
            <h3 id="delete-preview-title" className="text-sm font-medium">
              {t("delete.previewTitle", { count: previewRows.length })}
            </h3>
            <ul
              key={currentPage}
              className="max-h-[42vh] divide-y divide-border overflow-y-auto rounded-md border border-border focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              tabIndex={0}
              aria-labelledby="delete-preview-title"
            >
              {previewRows.slice(firstRow, firstRow + PREVIEW_PAGE_SIZE).map((row) => (
                <li key={row.id} className="flex flex-wrap items-start justify-between gap-x-3 gap-y-1 px-3 py-2">
                  <div className="min-w-0 flex-1">
                    <div className="break-words text-sm font-medium [overflow-wrap:anywhere]" title={row.title}>{row.title}</div>
                    <div className="break-all text-xs text-muted-foreground" title={row.id}>{row.project} · {row.id}</div>
                  </div>
                  <span className="rounded bg-muted px-2 py-1 text-xs text-muted-foreground">{row.reason}</span>
                </li>
              ))}
            </ul>
            {pageCount > 1 && (
              <nav className="flex justify-between gap-2" aria-labelledby="delete-preview-title">
                <Button variant="outline" size="sm" disabled={currentPage === 0} onClick={() => setPage(currentPage - 1)}>
                  {t("delete.previousPage")}
                </Button>
                <span className="self-center text-xs tabular-nums text-muted-foreground" role="status">
                  {t("delete.page", { current: currentPage + 1, total: pageCount })}
                </span>
                <Button variant="outline" size="sm" disabled={currentPage + 1 === pageCount} onClick={() => setPage(currentPage + 1)}>
                  {t("delete.nextPage")}
                </Button>
              </nav>
            )}
          </section>
        )}

        {showProgress && (
          <div className="flex items-start gap-2 rounded-md border border-border bg-muted/35 px-3 py-2 text-sm text-muted-foreground">
            <Loader2 aria-hidden="true" className="mt-0.5 h-4 w-4 shrink-0 animate-spin text-foreground" />
            <div className="space-y-1 tabular-nums">
              <div className="space-y-1" role="status">
                <p>{progressLabel}</p>
                {remainingLabel && <p>{remainingLabel}</p>}
              </div>
              <p>{t("delete.elapsed", { duration: formatDuration(elapsedSeconds, t) })}</p>
            </div>
          </div>
        )}

        <DialogFooter className="gap-2">
          <Button ref={cancelRef} variant="outline" onClick={onCancel} disabled={locked}>
            {cancelLabel}
          </Button>
          {count > 0 && <Button variant="destructive" onClick={() => onConfirm(setProgress)} disabled={locked || !canDelete}>
            {pending ? <Loader2 className="h-4 w-4 animate-spin" /> : <Trash2 className="h-4 w-4" />}
            {confirmLabel}
          </Button>}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
