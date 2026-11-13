import {
  Activity,
  BookOpen,
  Grid2X2,
  Layers,
  LayoutDashboard,
  ListChecks,
  Plug,
  Settings,
} from "lucide-react";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "./ui/sidebar";
import { Brand } from "./Brand";
import { ModeToggle } from "./ModeToggle";
import { useT } from "../lib/i18n";

export type Page =
  | "welcome"
  | "launchpad"
  | "catalog"
  | "preflight"
  | "operations"
  | "logs"
  | "guides"
  | "settings";
export const pageLabels: Record<Page, string> = {
  welcome: "概览",
  launchpad: "场景启动台",
  catalog: "全部服务",
  preflight: "环境体检",
  operations: "运行任务",
  logs: "端口与日志",
  guides: "使用指南",
  settings: "设置",
};
const groups = [
  {
    title: "工作台",
    items: [
      { page: "welcome", icon: LayoutDashboard },
      { page: "launchpad", icon: Layers },
      { page: "catalog", icon: Grid2X2 },
    ],
  },
  {
    title: "运维",
    items: [
      { page: "preflight", icon: Activity },
      { page: "operations", icon: ListChecks },
      { page: "logs", icon: Plug },
    ],
  },
  { title: "资源", items: [{ page: "guides", icon: BookOpen }] },
] as const;

export function AppSidebar({
  page,
  onPage,
}: {
  page: Page;
  onPage: (page: Page) => void;
}) {
  const t = useT();
  const { setOpenMobile, state } = useSidebar();
  const navigate = (value: Page) => {
    onPage(value);
    setOpenMobile(false);
  };
  return (
    <Sidebar collapsible="icon">
      <SidebarHeader className="px-4 py-4 group-data-[collapsible=icon]:px-5">
        <div className="flex h-12 items-center">
          <Brand />
        </div>
      </SidebarHeader>
      <SidebarContent className="px-2">
        {groups.map((group) => (
          <SidebarGroup key={group.title}>
            <SidebarGroupLabel>{t(group.title)}</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {group.items.map(({ page: value, icon: Icon }) => (
                  <SidebarMenuItem key={value}>
                    <SidebarMenuButton
                      isActive={page === value}
                      tooltip={t(pageLabels[value])}
                      onClick={() => navigate(value)}
                      aria-current={page === value ? "page" : undefined}
                    >
                      <Icon />
                      <span>{t(pageLabels[value])}</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                ))}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        ))}
      </SidebarContent>
      <SidebarFooter className="flex-row items-center gap-2 p-4 group-data-[collapsible=icon]:flex-col group-data-[collapsible=icon]:px-3">
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              isActive={page === "settings"}
              tooltip={t("设置")}
              aria-label={t("设置")}
              onClick={() => navigate("settings")}
            >
              <Settings />
              <span>{t("设置")}</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
        <ModeToggle />
        <span className="sr-only" aria-live="polite">
          {state === "collapsed" ? t("侧栏已折叠") : ""}
        </span>
      </SidebarFooter>
    </Sidebar>
  );
}
