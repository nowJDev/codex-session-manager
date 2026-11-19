// 세션 삭제 전 확인과 삭제 진행 상태를 표시하는 다이얼로그입니다.
import { useEffect, useState } from "react";
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

interface Props {
  open: boolean;
  count: number;
  pending: boolean;
  canDelete: boolean;
  title: string;
  description: string;
  confirmLabel: string;
  cancelLabel: string;
  t: (key: string, params?: Record<string, string | number>) => string;
  onConfirm: (reportProgress: (update: DeleteProgress) => void) => void;
  onCancel: () => void;
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
  t,
  onConfirm,
  onCancel,
}: Props) {
  const [progress, setProgress] = useState<DeleteProgress | null>(null);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const showProgress = open && pending;
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
  if (progress?.phase === "deleting") {
    const { processed, total, elapsedMs } = progress;
    progressLabel = total > 0 ? t("delete.requestProgress", { processed, total }) : t("delete.pending");
    if (processed >= 2 && processed < total && elapsedMs > 0) {
      const seconds = Math.ceil((elapsedMs / processed) * (total - processed) / 1000);
      remainingLabel = t("delete.remaining", { duration: formatDuration(seconds, t) });
    }
  } else if (progress?.phase === "verifying") {
    progressLabel = t("delete.verifying");
  }

  return (
    <Dialog open={open} onOpenChange={(v) => !v && !pending && onCancel()}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <div className="flex items-start gap-3 pr-6">
            <div className="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-destructive/15 text-destructive">
              {pending ? <Loader2 className="h-5 w-5 animate-spin" /> : <AlertTriangle className="h-5 w-5" />}
            </div>
            <div className="space-y-2">
              <DialogTitle>{title}</DialogTitle>
              <DialogDescription>{description}</DialogDescription>
            </div>
          </div>
        </DialogHeader>

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
          <Button variant="outline" onClick={onCancel} disabled={pending}>
            {cancelLabel}
          </Button>
          <Button variant="destructive" onClick={() => onConfirm(setProgress)} disabled={pending || count === 0 || !canDelete}>
            {pending ? <Loader2 className="h-4 w-4 animate-spin" /> : <Trash2 className="h-4 w-4" />}
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
