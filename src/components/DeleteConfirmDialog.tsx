// 세션 삭제 전 확인과 삭제 진행 상태를 표시하는 다이얼로그입니다.
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

interface Props {
  open: boolean;
  count: number;
  pending: boolean;
  title: string;
  description: string;
  confirmLabel: string;
  cancelLabel: string;
  pendingLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function DeleteConfirmDialog({
  open,
  count,
  pending,
  title,
  description,
  confirmLabel,
  cancelLabel,
  pendingLabel,
  onConfirm,
  onCancel,
}: Props) {
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

        {pending && (
          <div className="flex items-center gap-2 rounded-md border border-border bg-muted/35 px-3 py-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin text-foreground" />
            <span>{pendingLabel}</span>
          </div>
        )}

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={onCancel} disabled={pending}>
            {cancelLabel}
          </Button>
          <Button variant="destructive" onClick={onConfirm} disabled={pending || count === 0}>
            {pending ? <Loader2 className="h-4 w-4 animate-spin" /> : <Trash2 className="h-4 w-4" />}
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
