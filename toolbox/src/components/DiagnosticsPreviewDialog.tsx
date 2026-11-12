import { Checkbox } from "./ui/checkbox";
import { useT } from "../lib/i18n";
import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "./ui/dialog";
import { Button } from "./ui/button";
import { FieldSet, FieldLegend, Field, FieldLabel } from "./ui/field";
import { Alert, AlertTitle, AlertDescription } from "./ui/alert";
import { useWorkspace } from "../lib/workspace";
import type { DiagnosticsPreview } from "../types/toolbox";
export function DiagnosticsPreviewDialog({
  preview,
  onClose,
}: {
  preview: DiagnosticsPreview;
  onClose: () => void;
}) {
  const { api, run } = useWorkspace();
  const t = useT();
  const [excluded, setExcluded] = useState<string[]>([]);
  const [includeLogs, setIncludeLogs] = useState(false);
  const [busy, setBusy] = useState(false);
  const [path, setPath] = useState<string | null>(null);
  const hasLogs = Object.keys(preview.sections).some((key) =>
    key.toLowerCase().includes("log"),
  );
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent className="diagnostics-dialog">
        <DialogHeader>
          <DialogTitle>{t("预览诊断内容")}</DialogTitle>
          <DialogDescription>
            {t(
              "只导出以下白名单摘要。不会自动发送给任何人。日志默认排除；未知秘密仍可能需要手动排除。",
            )}
          </DialogDescription>
        </DialogHeader>
        <p>已排除字段：{preview.excludedFields.join("、") || "无"}</p>
        {!!preview.riskFlags.length && (
          <Alert>
            <AlertTitle>{t("请复核高风险内容")}</AlertTitle>
            <AlertDescription>{preview.riskFlags.join("；")}</AlertDescription>
          </Alert>
        )}
        <FieldSet>
          <FieldLegend>{t("选择要包含的摘要")}</FieldLegend>
          {Object.entries(preview.sections).map(([key, body]) => (
            <Field key={key}>
              <FieldLabel>
                <Checkbox
                  checked={!excluded.includes(key)}
                  onCheckedChange={(checked) =>
                    setExcluded(
                      checked
                        ? excluded.filter((id) => id !== key)
                        : [...excluded, key],
                    )
                  }
                />
                {key}
              </FieldLabel>
              <pre>{body}</pre>
            </Field>
          ))}
        </FieldSet>
        {hasLogs && (
          <Field>
            <FieldLabel>
              <Checkbox
                checked={includeLogs}
                onCheckedChange={(checked) => setIncludeLogs(checked)}
              />
              包含已预览的限定日志（最多 100 行）
            </FieldLabel>
          </Field>
        )}
        {path && <p role="status">{path}</p>}
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t("关闭")}
          </Button>
          <Button
            disabled={busy || !!path}
            onClick={async () => {
              setBusy(true);
              try {
                const result = await run(() =>
                  api.exportDiagnostics({
                    previewId: preview.previewId,
                    excludedSections: excluded,
                    includeLogs,
                  }),
                );
                if (result) setPath(result.path);
              } finally {
                setBusy(false);
              }
            }}
          >
            {t("确认导出")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
