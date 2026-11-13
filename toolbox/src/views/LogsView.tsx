import { useState } from "react";
import { Download } from "lucide-react";
import { PageHeader } from "../components/PageHeader";
import { LogViewer } from "../components/LogViewer";
import { DiagnosticsPreviewDialog } from "../components/DiagnosticsPreviewDialog";
import { Button } from "../components/ui/button";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../components/ui/select";
import {
  Empty,
  EmptyHeader,
  EmptyTitle,
  EmptyDescription,
} from "../components/ui/empty";
import { StatusBadge } from "../components/StatusBadge";
import { useWorkspace } from "../lib/workspace";
import type { DiagnosticsPreview } from "../types/toolbox";
export function LogsView() {
  const { state, api, run } = useWorkspace();
  const [selection, setSelection] = useState("");
  const [preview, setPreview] = useState<DiagnosticsPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const services =
    state.snapshot?.services.filter(
      (s) =>
        s.instanceId &&
        s.phase !== "ExternalObserve" &&
        !s.ownership.toLowerCase().startsWith("external"),
    ) ?? [];
  const selected =
    services.find((s) => s.instanceId === selection) ?? services[0];
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title="端口与日志"
        description="查看服务入口与最近输出。"
        actions={
          <Button
            variant="ghost"
            disabled={busy || !state.snapshot}
            onClick={async () => {
              setBusy(true);
              try {
                const result = await run(() => api.previewDiagnostics());
                if (result) setPreview(result);
              } finally {
                setBusy(false);
              }
            }}
          >
            <Download data-icon="inline-start" />
            预览诊断
          </Button>
        }
      />
      {selected?.instanceId ? (
        <>
          <div className="flex items-center gap-4">
            <Select value={selected.instanceId} onValueChange={setSelection}>
              <SelectTrigger aria-label="选择日志服务">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {services.map((s) => (
                    <SelectItem key={s.instanceId} value={s.instanceId!}>
                      {state.catalog?.services.find(
                        (spec) => spec.id === s.serviceId,
                      )?.name ?? s.serviceId}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
            <StatusBadge phase={selected.phase} />
          </div>
          <LogViewer
            key={selected.instanceId}
            instanceId={selected.instanceId}
          />
        </>
      ) : (
        <Empty>
          <EmptyHeader>
            <EmptyTitle>还没有可查看的实例</EmptyTitle>
            <EmptyDescription>
              启动一个服务后，可以在这里查看日志。
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      )}
      {preview && (
        <DiagnosticsPreviewDialog
          preview={preview}
          onClose={() => setPreview(null)}
        />
      )}
    </div>
  );
}
