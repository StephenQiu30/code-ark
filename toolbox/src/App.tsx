import { lazy, Suspense, useEffect, useRef, useState } from "react";
import { tauriApi } from "./api/tauri";
import type { ToolboxApi } from "./api/tauri";
import { WorkspaceProvider, useWorkspace } from "./lib/workspace";
import { LanguageContext, useT } from "./lib/i18n";
import type { Language } from "./lib/i18n";
import { ThemeProvider } from "./lib/theme";
import { AppSidebar, pageLabels } from "./components/Sidebar";
import type { Page } from "./components/Sidebar";
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "./components/ui/sidebar";
import { TooltipProvider } from "./components/ui/tooltip";
import { AppErrorBoundary, GlobalFeedback } from "./components/GlobalFeedback";
import { WelcomeView } from "./views/WelcomeView";
import { LaunchpadView } from "./views/LaunchpadView";
import { CatalogView } from "./views/CatalogView";
import { PreflightView } from "./views/PreflightView";
import { OperationsView } from "./views/OperationsView";
import { SettingsView } from "./views/SettingsView";
import { Skeleton } from "./components/ui/skeleton";

const GuideHubView = lazy(() =>
  import("./views/GuideHubView").then((m) => ({ default: m.GuideHubView })),
);
const LogsView = lazy(() =>
  import("./views/LogsView").then((m) => ({ default: m.LogsView })),
);
const ServiceDetailView = lazy(() =>
  import("./components/ServiceDetailDrawer").then((m) => ({
    default: m.ServiceDetailView,
  })),
);

function Workspace({
  language,
  onLanguage,
}: {
  language: Language;
  onLanguage: (language: Language) => void;
}) {
  const { state } = useWorkspace();
  const t = useT();
  const [page, setPage] = useState<Page>("welcome");
  const [serviceId, setServiceId] = useState<string | null>(null);
  const trigger = useRef<HTMLElement | null>(null);
  const openService = (id: string) => {
    trigger.current =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    setServiceId(id);
  };
  const closeService = () => {
    setServiceId(null);
    requestAnimationFrame(() => {
      const fallback = Array.from(
        document.querySelectorAll<HTMLElement>("[data-service-id]"),
      ).find((n) => n.dataset.serviceId === serviceId);
      (trigger.current?.isConnected && trigger.current !== document.body
        ? trigger.current
        : fallback
      )?.focus();
    });
  };
  useEffect(() => {
    if (!serviceId) return;
    const escape = (event: KeyboardEvent) => {
      if (
        event.key === "Escape" &&
        !event.defaultPrevented &&
        !document.querySelector('[role="dialog"], [role="alertdialog"]')
      )
        closeService();
    };
    window.addEventListener("keydown", escape);
    return () => window.removeEventListener("keydown", escape);
  }, [serviceId]);
  useEffect(() => {
    setServiceId(null);
  }, [state.snapshot?.workspace.workspaceId]);
  return (
    <SidebarProvider
      style={{ "--sidebar-width-icon": "4rem" } as React.CSSProperties}
    >
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:absolute focus:p-3"
      >
        {t("跳至正文")}
      </a>
      <AppSidebar
        page={page}
        onPage={(value) => {
          setServiceId(null);
          setPage(value);
        }}
      />
      <SidebarInset className="min-w-0">
        <header className="sticky top-0 flex h-16 shrink-0 items-center gap-4 bg-background px-6">
          <SidebarTrigger aria-label={t("切换侧栏")} />
          <span className="text-[13px] font-medium">
            {serviceId
              ? (state.catalog?.services.find((s) => s.id === serviceId)
                  ?.name ?? t("服务详情"))
              : t(pageLabels[page])}
          </span>
        </header>
        <div id="main-content" className="workspace-page">
          {!state.catalog && !state.error ? (
            <div className="flex flex-col gap-3">
              <Skeleton className="h-10 w-48" />
              <Skeleton className="h-40" />
            </div>
          ) : (
            <Suspense fallback={<Skeleton className="h-64" />}>
              <div hidden={!!serviceId}>
                {page === "welcome" && <WelcomeView onService={openService} />}
                {page === "launchpad" && <LaunchpadView />}
                {page === "catalog" && <CatalogView onService={openService} />}
                {page === "preflight" && <PreflightView />}
                {page === "operations" && <OperationsView />}
                {page === "guides" && <GuideHubView />}
                {page === "settings" && (
                  <SettingsView language={language} onLanguage={onLanguage} />
                )}
                {page === "logs" && <LogsView />}
              </div>
              {serviceId && (
                <ServiceDetailView
                  key={`${state.snapshot?.workspace.workspaceId}:${serviceId}`}
                  serviceId={serviceId}
                  onClose={closeService}
                />
              )}
            </Suspense>
          )}
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}
export default function App({ api = tauriApi }: { api?: ToolboxApi }) {
  const [language, setLanguage] = useState<Language>("zh");
  return (
    <LanguageContext.Provider value={language}>
      <ThemeProvider>
        <GlobalFeedback />
        <AppErrorBoundary>
          <TooltipProvider>
            <WorkspaceProvider api={api}>
              <Workspace language={language} onLanguage={setLanguage} />
            </WorkspaceProvider>
          </TooltipProvider>
        </AppErrorBoundary>
      </ThemeProvider>
    </LanguageContext.Provider>
  );
}
