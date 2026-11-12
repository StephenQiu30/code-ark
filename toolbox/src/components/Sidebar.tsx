import {
  Boxes,
  BookOpen,
  Settings,
  Activity,
  LayoutGrid,
  Play,
  ShieldCheck,
  Terminal,
  Anchor,
} from "lucide-react";
import { Button } from "./ui/button";
import { Separator } from "./ui/separator";
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
const items = [
  ["welcome", "推荐入门", Play],
  ["launchpad", "场景启动台", LayoutGrid],
  ["catalog", "全部服务", Boxes],
  ["preflight", "环境体检", ShieldCheck],
  ["operations", "运行任务", Activity],
  ["logs", "端口和日志", Terminal],
  ["guides", "教程", BookOpen],
] as const;
export function Sidebar({
  page,
  onPage,
}: {
  page: Page;
  onPage: (page: Page) => void;
}) {
  const t = useT();
  return (
    <aside className="sidebar">
      <div className="brand">
        <Anchor aria-hidden="true" />
        <div>
          <strong>Code Ark</strong>
          <p>{t("本地开发工作箱")}</p>
        </div>
      </div>
      <nav aria-label="主导航" className="flex flex-col gap-1">
        {items.map(([id, label, Icon], index) => (
          <div key={id}>
            {(index === 0 || index === 3 || index === 6) && (
              <p className="nav-label">
                {t(index === 0 ? "工作台" : index === 3 ? "运维" : "教程")}
              </p>
            )}
            <Button
              className="nav-button"
              variant={page === id ? "secondary" : "ghost"}
              aria-current={page === id ? "page" : undefined}
              onClick={() => onPage(id)}
            >
              <Icon data-icon="inline-start" />
              {t(label)}
            </Button>
          </div>
        ))}
      </nav>
      <div className="sidebar-footer">
        <Separator />
        <Button
          className="nav-button"
          variant={page === "settings" ? "secondary" : "ghost"}
          onClick={() => onPage("settings")}
        >
          <Settings data-icon="inline-start" />
          {t("设置")}
        </Button>
        <small>{t("源码实施中 · 尚未发行")}</small>
      </div>
    </aside>
  );
}
