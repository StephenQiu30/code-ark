import { ToggleGroup, ToggleGroupItem } from "../components/ui/toggle-group";
import { useState } from "react";
import { Button } from "../components/ui/button";
import { Folder } from "lucide-react";
import { EnvironmentConnectButton } from "../components/EnvironmentConnectButton";
import { PageHeader } from "../components/PageHeader";
import { Separator } from "../components/ui/separator";
import { ModeToggle } from "../components/ModeToggle";
import { useTheme } from "../lib/theme";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "../components/ui/dialog";
import { useWorkspace } from "../lib/workspace";
import { useT } from "../lib/i18n";
import type { Language } from "../lib/i18n";
export function SettingsView({
  language,
  onLanguage,
}: {
  language: Language;
  onLanguage: (language: Language) => void;
}) {
  const { api, state, run } = useWorkspace();
  const [confirm, setConfirm] = useState(false);
  const t = useT();
  const { theme } = useTheme();
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title={t("设置")}
        description={t("管理本机的外观与工作区。")}
      />
      <section className="flex items-center justify-between gap-6">
        <div className="flex flex-col gap-2">
          <h2 className="section-title">{t("外观")}</h2>
          <p className="muted">{t("选择浅色、深色，或跟随系统。")}</p>
        </div>
        <div className="flex shrink-0 items-center gap-3">
          <span className="muted">
            {t({ light: "浅色", dark: "深色", system: "跟随系统" }[theme])}
          </span>
          <ModeToggle />
        </div>
      </section>
      <Separator />
      <section className="flex flex-col gap-5">
        <h2 className="section-title">{t("工作区")}</h2>
        <div className="flex flex-wrap items-center justify-between gap-4">
          <p className="min-w-0 break-all">
            {state.snapshot?.workspace.root ?? t("未连接")}
          </p>
          <Button variant="ghost" onClick={() => setConfirm(true)}>
            <Folder data-icon="inline-start" />
            {t("打开另一工作区")}
          </Button>
        </div>
        <p className="muted">{t("切换不移动数据，不停止旧工作区实例。")}</p>
      </section>
      <Separator />
      <section className="flex flex-col gap-5">
        <h2 className="section-title">{t("Docker 目标")}</h2>
        <div className="flex items-center justify-between gap-4">
          <p>{state.snapshot?.target?.contextName ?? t("未连接")}</p>
          <EnvironmentConnectButton />
        </div>
      </section>
      <Separator />
      <section className="flex flex-wrap items-center justify-between gap-6">
        <div className="flex flex-col gap-2">
          <h2 className="section-title">{t("语言 / Language")}</h2>
          <p className="muted">{t("默认中文，语言选择只保留在当前窗口。")}</p>
        </div>
        <ToggleGroup
          type="single"
          aria-label="语言 / Language"
          value={language}
          onValueChange={(value) => {
            if (value === "zh" || value === "en") onLanguage(value);
          }}
          variant="outline"
        >
          <ToggleGroupItem value="zh">简体中文</ToggleGroupItem>
          <ToggleGroupItem value="en">English</ToggleGroupItem>
        </ToggleGroup>
      </section>
      <Separator />
      <p className="text-xs text-muted-foreground">代码方舟 · Code Ark</p>
      <Dialog open={confirm} onOpenChange={setConfirm}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("打开另一工作区")}</DialogTitle>
            <DialogDescription>
              {t(
                "当前工作区的实例仍然运行。新位置不会自动迁移旧数据，请先按需停止与备份。",
              )}
            </DialogDescription>
          </DialogHeader>
          <p>
            已登记实例：
            {state.snapshot?.services
              .filter((s) => s.instanceId)
              .map((s) => s.serviceId)
              .join("、") || "无"}
          </p>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConfirm(false)}>
              {t("取消")}
            </Button>
            <Button
              onClick={async () => {
                const result = await run(() => api.chooseWorkspace());
                if (result) setConfirm(false);
              }}
            >
              {t("选择工作区")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
