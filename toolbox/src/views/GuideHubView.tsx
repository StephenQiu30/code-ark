import { useEffect, useState } from "react";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardFooter,
} from "../components/ui/card";
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
  const [exportPath, setExportPath] = useState<string | null>(null);
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
      <h1>{t("教程")}</h1>
      <p className="muted">
        {t(
          "基础设施连接测试不需要 Git / Node / Rust。教学源码的语言工具链另行安装；待验证示例不代表已通过。",
        )}
      </p>
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
        <Card>
          <CardHeader>
            <Button
              variant="ghost"
              onClick={() => {
                setSelected(null);
                setBody("");
                setExportPath(null);
              }}
            >
              {t("返回全部教程")}
            </Button>
            <CardTitle>{selected.title}</CardTitle>
            <CardDescription>{selected.scopeNotice}</CardDescription>
          </CardHeader>
          <CardContent>
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
                  const result = await run(() =>
                    api.exportGuideSources(selected.guideId),
                  );
                  if (result) setExportPath(result.path);
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
            {exportPath && <p role="status">{exportPath}</p>}
            <pre className="guide-text">{body}</pre>
            <h3>{t("执行")}</h3>
            <pre>{selected.invocation}</pre>
            <h3>{t("预期结果（不是本次执行证据）")}</h3>
            <pre>{selected.expectedOutput}</pre>
            <h3>{t("清理教学数据")}</h3>
            <pre>{selected.cleanup}</pre>
          </CardContent>
        </Card>
      ) : (
        <div className="card-grid">
          {guides.map((guide) => (
            <Card key={guide.guideId}>
              <CardHeader>
                <CardTitle>{guide.title}</CardTitle>
                <CardDescription>{guide.scopeNotice}</CardDescription>
              </CardHeader>
              <CardContent>
                <Badge variant="secondary">{guide.status}</Badge>
                <p className="muted">
                  {guide.requiredTools.join(" / ") || "离线指南"}
                </p>
              </CardContent>
              <CardFooter>
                <Button
                  variant="outline"
                  onClick={async () => {
                    const result = await run(() =>
                      api.readGuide(guide.guideId),
                    );
                    if (result !== null) {
                      setSelected(guide);
                      setBody(result);
                    }
                  }}
                >
                  {t("阅读指南")}
                </Button>
              </CardFooter>
            </Card>
          ))}
        </div>
      )}
      <Card>
        <CardHeader>
          <CardTitle>{t("MediaCrawler · 宿主机工具")}</CardTitle>
          <CardDescription>
            {t("不计入 17 个服务，不通过 Docker 服务启动。")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {t(
            "需要 Git / uv。请按包内锁定 commit 与补丁指南操作，实际抓取能力及站点条件单独验证。",
          )}
        </CardContent>
      </Card>
    </div>
  );
}
