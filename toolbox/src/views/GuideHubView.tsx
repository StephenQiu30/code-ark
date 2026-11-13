import { useEffect, useState } from "react";
import { ArrowUpRight } from "lucide-react";
import { PageHeader } from "../components/PageHeader";
import { ResourceList } from "../components/ResourceList";
import {
  Item,
  ItemContent,
  ItemTitle,
  ItemDescription,
  ItemActions,
} from "../components/ui/item";
import { Separator } from "../components/ui/separator";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { useWorkspace } from "../lib/workspace";
import { useT } from "../lib/i18n";
import type { GuideSpec } from "../types/toolbox";
export function GuideHubView() {
  const { api, run } = useWorkspace();
  const t = useT();
  const [guides, setGuides] = useState<GuideSpec[]>([]);
  const [selected, setSelected] = useState<GuideSpec | null>(null);
  const [body, setBody] = useState("");
  const [exportBusy, setExportBusy] = useState(false);
  useEffect(() => {
    let alive = true;
    void run(() => api.listGuides()).then((result) => {
      if (result && alive) setGuides(result);
    });
    return () => {
      alive = false;
    };
  }, [api, run]);
  return (
    <div className="flex flex-col gap-5">
      <PageHeader
        title={t("使用指南")}
        description="从启动第一个服务，到接入你的项目。"
      />
      <div className="flex flex-wrap gap-2">
        <Button
          variant="outline"
          onClick={() => void run(() => api.openOfficial("docker-install"))}
        >
          {t("Docker 安装与复检")}
        </Button>
        <Button
          variant="outline"
          onClick={() =>
            void run(() => api.openOfficial("tauri-prerequisites"))
          }
        >
          {t("系统运行库与首次打开")}
        </Button>
      </div>
      {selected ? (
        <section className="flex min-w-0 flex-col gap-6">
          <div className="flex flex-col gap-3">
            <Button
              variant="ghost"
              onClick={() => {
                setSelected(null);
                setBody("");
              }}
            >
              {t("返回全部教程")}
            </Button>
            <h2 className="section-title">{selected.title}</h2>
            <p className="muted">{selected.scopeNotice}</p>
          </div>
          <div className="flex min-w-0 flex-col gap-4">
            <dl className="details">
              <dt>{t("客户端版本")}</dt>
              <dd>
                {selected.clientFamily}{" "}
                {selected.lockedClientVersion ?? "待锁定"}
              </dd>
              <dt>{t("所需工具")}</dt>
              <dd>{selected.requiredTools.join("、") || "无需业务开发工具"}</dd>
              <dt>{t("验证状态")}</dt>
              <dd>{selected.status}</dd>
            </dl>
            <Button
              variant="outline"
              disabled={exportBusy}
              onClick={async () => {
                if (!selected) return;
                setExportBusy(true);
                try {
                  await run(() => api.exportGuideSources(selected.guideId));
                } finally {
                  setExportBusy(false);
                }
              }}
            >
              {t("导出教学源码")}
            </Button>
            <p className="muted">
              {t(
                "在原生目录选择器中选择导出位置。只写入公开白名单源码，不会运行示例或复制凭据。",
              )}
            </p>
            <pre className="guide-text">{body}</pre>
            <h3>{t("执行")}</h3>
            <pre>{selected.invocation}</pre>
            <h3>{t("预期结果（不是本次执行证据）")}</h3>
            <pre>{selected.expectedOutput}</pre>
            <h3>{t("清理教学数据")}</h3>
            <pre>{selected.cleanup}</pre>
          </div>
        </section>
      ) : (
        <ResourceList items={guides} getKey={(guide) => guide.guideId}>
          {(guide) => (
            <Item asChild variant="service">
              <button
                type="button"
                aria-label={`阅读指南 ${guide.title}`}
                onClick={async () => {
                  const result = await run(() => api.readGuide(guide.guideId));
                  if (result !== null) {
                    setSelected(guide);
                    setBody(result);
                  }
                }}
              >
                <ItemContent className="min-w-0">
                  <ItemTitle>{guide.title}</ItemTitle>
                  <ItemDescription>{guide.scopeNotice}</ItemDescription>
                </ItemContent>
                <ItemActions>
                  <Badge variant="status">{guide.status}</Badge>
                  <ArrowUpRight className="size-4" />
                </ItemActions>
              </button>
            </Item>
          )}
        </ResourceList>
      )}
      <Separator />
      <section className="flex min-w-0 flex-col gap-6">
        <div className="flex flex-col gap-3">
          <h2 className="section-title">{t("MediaCrawler · 宿主机工具")}</h2>
          <p className="muted">
            {t("不计入 17 个服务，不通过 Docker 服务启动。")}
          </p>
        </div>
        <div className="flex min-w-0 flex-col gap-4">
          {t(
            "需要 Git / uv。请按包内锁定 commit 与补丁指南操作，实际抓取能力及站点条件单独验证。",
          )}
        </div>
      </section>
    </div>
  );
}
