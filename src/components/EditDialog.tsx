// 세션의 이름과 설명을 편집하고 저장 결과를 표시한다.
import { useEffect, useRef, useState } from "react";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface Props {
  open: boolean;
  title: string;
  label: string;
  initialValue: string;
  placeholder?: string;
  t: (key: string) => string;
  onSubmit: (value: string) => Promise<void>;
  onClose: () => void;
}

export function EditDialog({ open, title, label, initialValue, placeholder, t, onSubmit, onClose }: Props) {
  const [value, setValue] = useState(initialValue);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const inFlight = useRef(false);

  useEffect(() => {
    if (open) {
      setValue(initialValue);
      setError("");
    }
  }, [open, initialValue]);

  async function save() {
    if (inFlight.current) return;
    inFlight.current = true;
    setSaving(true);
    setError("");
    try {
      await onSubmit(value);
    } catch (failure) {
      setError(String(failure));
    } finally {
      inFlight.current = false;
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !next && !inFlight.current && onClose()}>
      <DialogContent closeLabel={t("common.close")} showClose={!saving}>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        <div className="space-y-2">
          <Label htmlFor="edit-session-value">{label}</Label>
          <Input
            id="edit-session-value"
            autoFocus
            value={value}
            placeholder={placeholder}
            disabled={saving}
            aria-invalid={!!error}
            aria-describedby={error ? "edit-session-error" : undefined}
            onChange={(event) => setValue(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.nativeEvent.isComposing && event.keyCode !== 229) {
                event.preventDefault();
                void save();
              }
            }}
          />
          {error && (
            <p id="edit-session-error" role="alert" className="break-all text-sm text-[hsl(var(--error))]">
              {t("edit.saveError")} {error}
            </p>
          )}
        </div>
        <DialogFooter className="gap-2">
          <Button variant="outline" disabled={saving} onClick={onClose}>{t("settings.cancel")}</Button>
          <Button disabled={saving} onClick={save}>{t(saving ? "settings.saving" : "settings.save")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
