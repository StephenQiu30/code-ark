import { ToggleGroup, ToggleGroupItem } from "../components/ui/toggle-group";
import { useState } from "react";
import { Button } from "../components/ui/button";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "../components/ui/card";
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
  return (
    <div className="flex flex-col gap-5">
      <h1>{t("设置")}</h1>
      <Card>
        <CardHeader>
          <CardTitle>{t("语言 / Language")}</CardTitle>
          <CardDescription>
            {t("默认中文，语言选择只保留在当前窗口。")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <ToggleGroup
            aria-label="语言 / Language"
            value={[language]}
            onValueChange={(value) => {
              if (value[0]) onLanguage(value[0] as Language);
            }}
            variant="outline"
          >
            <ToggleGroupItem value="zh">简体中文</ToggleGroupItem>
            <ToggleGroupItem value="en">English</ToggleGroupItem>
          </ToggleGroup>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>{t("工作区")}</CardTitle>
          <CardDescription>
            {t("切换不移动数据，不停止旧工作区实例。")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p className="break-all">
            {state.snapshot?.workspace.root ?? "未连接"}
          </p>
          <Button variant="outline" onClick={() => setConfirm(true)}>
            {t("打开另一工作区")}
          </Button>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>{t("数据与发行状态")}</CardTitle>
          <CardDescription>
            {t("源码实施中；最终安装包与全部原生目标验证尚未完成。")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p>
            {t("备份与恢复使用离线迁移指南。已迁移的数据没有自动降级按钮。")}
          </p>
        </CardContent>
      </Card>
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
