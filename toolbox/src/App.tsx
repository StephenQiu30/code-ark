import { useState } from "react";
import { tauriApi } from "./api/tauri";
import type { ToolboxApi } from "./api/tauri";
import { WorkspaceProvider, useWorkspace } from "./lib/workspace";
import { LanguageContext, useT } from "./lib/i18n";
import type { Language } from "./lib/i18n";
import { Sidebar } from "./components/Sidebar";
import type { Page } from "./components/Sidebar";
import { EnvironmentBanner } from "./components/EnvironmentBanner";
import { WelcomeView } from "./views/WelcomeView";
import { LaunchpadView } from "./views/LaunchpadView";
import { CatalogView } from "./views/CatalogView";
import { PreflightView } from "./views/PreflightView";
import { OperationsView } from "./views/OperationsView";
import { GuideHubView } from "./views/GuideHubView";
import { SettingsView } from "./views/SettingsView";
import { ServiceDetailDrawer } from "./components/ServiceDetailDrawer";
import { DiagnosticsPreviewDialog } from "./components/DiagnosticsPreviewDialog";
import { Button } from "./components/ui/button";
import { Badge } from "./components/ui/badge";
import { Skeleton } from "./components/ui/skeleton";
import type { DiagnosticsPreview } from "./types/toolbox";
function Workspace({
  language,
  onLanguage,
}: {
  language: Language;
  onLanguage: (language: Language) => void;
}) {
  const { state, api, run } = useWorkspace();
  const t = useT();
  const [page, setPage] = useState<Page>("welcome");
  const [serviceId, setServiceId] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState<DiagnosticsPreview | null>(
    null,
  );
  const [diagnosticsBusy, setDiagnosticsBusy] = useState(false);
  const ready =
    state.snapshot?.services.filter((s) => s.phase === "Ready").length ?? 0;
  const managed =
    state.snapshot?.services.filter(
      (s) => s.instanceId && s.ownership.toLowerCase() === "managed",
    ).length ?? 0;
  const running =
    state.snapshot?.operations.filter((o) =>
      o.allowedActions?.includes("cancel"),
    ).length ?? 0;
  return (
    <div className="app-shell">
      <Sidebar page={page} onPage={setPage} />
      <div className="workspace-column">
        <header className="topbar">
          <div className="flex flex-wrap items-center gap-3">
            <Badge variant="outline">
              {state.snapshot?.target?.contextName ?? "Docker 未连接"}
            </Badge>
            <span className="muted">
              {ready} {t("已就绪")} / {managed} {t("托管")}
              {running ? ` · ${running} 个任务进行中` : ""}
            </span>
          </div>
          <Button
            variant="outline"
            size="sm"
            disabled={diagnosticsBusy || !state.snapshot}
            onClick={async () => {
              setDiagnosticsBusy(true);
              try {
                const result = await run(() => api.previewDiagnostics());
                if (result) setDiagnostics(result);
              } finally {
                setDiagnosticsBusy(false);
              }
            }}
          >
            {t("预览诊断")}
          </Button>
        </header>
        <main id="main-content" className="workspace-main">
          <EnvironmentBanner />
          {!state.catalog && !state.error ? (
            <div className="flex flex-col gap-3">
              <Skeleton className="h-10" />
              <Skeleton className="h-40" />
              <p>{t("等待后端快照")}</p>
            </div>
          ) : (
            <>
              {page === "welcome" && <WelcomeView onService={setServiceId} />}{" "}
              {page === "launchpad" && <LaunchpadView />}
              {page === "catalog" && (
                <CatalogView onService={setServiceId} />
              )}{" "}
              {page === "preflight" && <PreflightView />}
              {page === "operations" && <OperationsView />}
              {page === "guides" && <GuideHubView />}
              {page === "settings" && (
                <SettingsView language={language} onLanguage={onLanguage} />
              )}{" "}
              {page === "logs" && (
                <div className="flex flex-col gap-4">
                  <h1>{t("端口和日志")}</h1>
                  <p className="muted">
                    选择已登记实例查看实际端口、连接与有界脱敏日志。
                  </p>
                  {state.snapshot?.services
                    .filter((s) => s.instanceId)
                    .map((s) => (
                      <Button
                        key={s.serviceId}
                        variant="outline"
                        onClick={() => setServiceId(s.serviceId)}
                      >
                        {state.catalog?.services.find(
                          (spec) => spec.id === s.serviceId,
                        )?.name ?? s.serviceId}{" "}
                        · {s.appliedGeneration ?? "尚未应用"}
                      </Button>
                    ))}
                </div>
              )}
            </>
          )}
        </main>
      </div>
      {serviceId && (
        <ServiceDetailDrawer
          key={`${state.snapshot?.workspace.workspaceId}:${serviceId}`}
          serviceId={serviceId}
          onClose={() => setServiceId(null)}
        />
      )}{" "}
      {diagnostics && (
        <DiagnosticsPreviewDialog
          preview={diagnostics}
          onClose={() => setDiagnostics(null)}
        />
      )}
    </div>
  );
}
export default function App({ api = tauriApi }: { api?: ToolboxApi }) {
  const [language, setLanguage] = useState<Language>("zh");
  return (
    <LanguageContext.Provider value={language}>
      <WorkspaceProvider api={api}>
        <Workspace language={language} onLanguage={setLanguage} />
      </WorkspaceProvider>
    </LanguageContext.Provider>
  );
}
